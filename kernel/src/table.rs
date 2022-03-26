// TODO:
// - Use ASIDs to avoid unecessarily general TLB flushes.
//     - Does this put policy into the kernel? Can we let user mode control ASID
//       allocation and just ensure isolation in the kernel, like we do with
//       frames?
// - Allow user mode to use accessed, dirty, and global bits.
//

use {
    crate::{
        frame::{Arc, Idx},
        page::L0PageCap,
        sync::{Token, TokenCell},
        thread::{CallCap, ThreadCap},
    },
    ::core::cell::Cell,
};

pub const TABLE_LEN: usize = 0x200;

pub enum Cap {
    L2Table(L2TableCap),
    L1Table(L1TableCap),
    L0Table(L0TableCap),
    L0Page(L0PageCap),
    Thread(ThreadCap),
    Call(CallCap),
}

#[derive(Debug, Clone, Copy)]
pub enum Permissions {
    ReadOnly,
    ReadWrite,
    ExecuteOnly,
    ReadExecute,
    ReadWriteExecute,
}

impl Permissions {
    const fn bits(&self) -> u64 {
        const READ: u64 = 0b1 << 1;
        const WRITE: u64 = 0b1 << 2;
        const EXECUTE: u64 = 0b1 << 3;

        match self {
            Self::ReadOnly => READ,
            Self::ReadWrite => READ | WRITE,
            Self::ExecuteOnly => EXECUTE,
            Self::ReadExecute => READ | EXECUTE,
            Self::ReadWriteExecute => READ | WRITE | EXECUTE,
        }
    }
}

impl Cap {
    fn l0_entry(self) -> L0Entry {
        let (frame_number, tag) = match self {
            Self::L2Table(l2_table) => (l2_table.into_frame_number(), 0x0u8),
            Self::L1Table(l1_table) => (l1_table.into_frame_number(), 0x1u8),
            Self::L0Table(l0_table) => (l0_table.into_frame_number(), 0x2u8),
            Self::L0Page(l0_page) => (l0_page.into_frame_number(), 0x5u8),
            Self::Thread(thread) => (thread.into_frame_number(), 0x6u8),
            Self::Call(call) => (call.into_frame_number(), 0x7u8),
        };
        L0Entry::cap(frame_number, tag)
    }
}

#[derive(Clone)]
pub struct L2TableCap {
    entries: Arc<TokenCell<[L2Entry; TABLE_LEN]>>,
}

impl ::core::fmt::Debug for L2TableCap {
    fn fmt(&self, _: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        Ok(())
    }
}

#[derive(Clone)]
pub struct L1TableCap {
    entries: Arc<TokenCell<[L1Entry; TABLE_LEN]>>,
}

#[derive(Clone)]
pub struct L0TableCap {
    entries: Arc<TokenCell<[L0Entry; TABLE_LEN]>>,
}

pub const fn boot_l2_table() -> [L2Entry; TABLE_LEN] {
    // # Kernel Address Space
    // `(0x0000_0000_0000_0000..=0x0000_0000_7fff_ffff)`: unmapped (2GiB)
    // `(0x0000_0000_8000_0000..=0x0000_003f_ffff_ffff)`: user mappable (254GiB)
    // `(0xffff_ffc0_0000_0000..=0xffff_ffff_bfff_ffff)`: frame mapped
    // (254GiB)
    // `(0xffff_ffff_c000_0000..=0xffff_ffff_ffff_ffff)`: kernel mapped (2GiB)
    const INVALID_ENTRY: L2Entry = L2Entry::invalid();
    let mut entries = [INVALID_ENTRY; TABLE_LEN];
    let mut index = TABLE_LEN / 2 + 0x0;
    while index < TABLE_LEN - 1 {
        entries[index] = unsafe { L2Entry::kernel(index - TABLE_LEN / 2, Permissions::ReadWrite) };
        index += 1;
    }
    entries[TABLE_LEN - 1] = unsafe { L2Entry::kernel(0x2, Permissions::ReadWriteExecute) };
    entries
}

static KERNEL_L1_TABLE: TokenCell<Option<L1TableCap>> = TokenCell::new(None);

pub unsafe fn set_kernel_l1_table(l1_table: L1TableCap, token: &mut Token) {
    let kernel_l1_table = KERNEL_L1_TABLE.borrow_mut(token);
    *kernel_l1_table = Some(l1_table);
}

// TODO: Entries should drop capabilities when they are dropped.

#[repr(transparent)]
pub struct L2Entry(u64);

#[repr(transparent)]
struct L1Entry(u64);

#[repr(transparent)]
struct L0Entry(u64);

impl L2TableCap {
    pub fn activate(self) {
        #[thread_local]
        static BOOTSTRAPPED: Cell<bool> = Cell::new(false);

        // TODO: We should be more precise about our fences.
        // TODO: We need to do remote fences for the other harts.

        if BOOTSTRAPPED.replace(true) {
            let mut satp = 0x0;
            satp |= self.entries.into_raw().into_raw() as u64;
            const SATP_MODE_SV39: u64 = 0x8000_0000_0000_0000u64;
            satp |= SATP_MODE_SV39;
            satp = unsafe { crate::plat::swap_satp(satp) };
            let entries: Arc<TokenCell<[L2Entry; TABLE_LEN]>> =
                unsafe { Arc::from_raw(Idx::from_raw((satp & !SATP_MODE_SV39) as usize).unwrap()) };
            drop(entries);
        } else {
            let mut satp = 0x0;
            satp |= self.entries.into_raw().into_raw() as u64;
            const SATP_MODE_SV39: u64 = 0x8000_0000_0000_0000u64;
            satp |= SATP_MODE_SV39;
            unsafe { crate::plat::set_satp(satp) };
        }
    }

    pub fn new(frame_number: Idx, token: &Token) -> Option<Self> {
        let mut l2_entries = boot_l2_table();
        let kernel_l1_table = KERNEL_L1_TABLE.borrow(&token);
        let kernel_l1_table = kernel_l1_table.clone().unwrap();
        l2_entries[TABLE_LEN - 1] = L2Entry::kernel_interior(kernel_l1_table);
        let entries = Arc::new(frame_number, TokenCell::new(l2_entries))?;
        Some(Self { entries })
    }

    pub fn map_l1_table(&self, token: &mut Token, index: usize, l1_table: L1TableCap) {
        assert!(index > 0);
        assert!(index < TABLE_LEN / 2);
        let entries = self.entries.borrow_mut(token);
        entries[index] = L2Entry::interior(l1_table);
    }

    pub fn into_frame_number(self) -> Idx {
        self.entries.into_raw()
    }
}

impl L1TableCap {
    pub fn new(frame_number: Idx) -> Option<Self> {
        const INVALID_ENTRY: L1Entry = L1Entry::invalid();
        let entries = Arc::new(frame_number, TokenCell::new([INVALID_ENTRY; TABLE_LEN]))?;
        Some(Self { entries })
    }

    pub fn map_l0_table(&self, token: &mut Token, index: usize, l0_table: L0TableCap) {
        let entries = self.entries.borrow_mut(token);
        entries[index] = L1Entry::interior(l0_table);
    }

    pub fn map_l0_kernel_table(&self, token: &mut Token, index: usize, l0_table: L0TableCap) {
        let entries = self.entries.borrow_mut(token);
        entries[index] = unsafe { L1Entry::kernel_interior(l0_table) };
    }

    pub fn into_frame_number(self) -> Idx {
        self.entries.into_raw()
    }
}

impl L0TableCap {
    pub fn new(frame_number: Idx) -> Option<Self> {
        const INVALID_ENTRY: L0Entry = L0Entry::invalid();
        let entries = Arc::new(frame_number, TokenCell::new([INVALID_ENTRY; TABLE_LEN]))?;
        Some(Self { entries })
    }

    pub fn map_l0_page(
        &self,
        token: &mut Token,
        index: usize,
        l0_page: L0PageCap,
        permissions: Permissions,
    ) {
        let entries = self.entries.borrow_mut(token);
        entries[index] = L0Entry::leaf(l0_page, permissions);
    }

    pub unsafe fn map_l0_kernel_page(
        &self,
        token: &mut Token,
        index: usize,
        l0_page: L0PageCap,
        permissions: Permissions,
    ) {
        let entries = self.entries.borrow_mut(token);
        entries[index] = unsafe { L0Entry::kernel_leaf(l0_page, permissions) };
    }

    pub fn give_capability(&self, token: &mut Token, index: usize, cap: Cap) {
        let entries = self.entries.borrow_mut(token);
        entries[index] = cap.l0_entry();
    }

    // TODO: allow cloning, revoking, and fetching capabilities.

    pub fn into_frame_number(self) -> Idx {
        self.entries.into_raw()
    }
}

impl L2Entry {
    pub const unsafe fn kernel(l2_frame_number: usize, permissions: Permissions) -> Self {
        let frame_number = (l2_frame_number << 18) as u64;
        const VALID: u64 = 0b1 << 0;
        let permissions = permissions.bits();
        const USER: u64 = 0b0 << 4;
        const GLOBAL: u64 = 0b1 << 5;
        const ACCESSED: u64 = 0b1 << 6;
        const DIRTY: u64 = 0b1 << 7;
        const RSW: u64 = 0b00 << 8;
        let ppn = (frame_number & ((1 << 44) - 1)) << 10;
        Self(VALID | permissions | USER | GLOBAL | ACCESSED | DIRTY | RSW | ppn)
    }

    pub fn interior(l1_table: L1TableCap) -> Self {
        let frame_number = l1_table.entries.into_raw().into_raw() as u64;
        const VALID: u64 = 0b1 << 0;
        const RSW: u64 = 0b00 << 8;
        let ppn = (frame_number & ((1 << 44) - 1)) << 10;
        Self(VALID | RSW | ppn)
    }

    pub fn kernel_interior(l1_table: L1TableCap) -> Self {
        let frame_number = l1_table.entries.into_raw().into_raw() as u64;
        const VALID: u64 = 0b1 << 0;
        const GLOBAL: u64 = 0b1 << 5;
        const RSW: u64 = 0b00 << 8;
        let ppn = (frame_number & ((1 << 44) - 1)) << 10;
        Self(VALID | GLOBAL | RSW | ppn)
    }

    pub const fn invalid() -> Self {
        const VALID: u64 = 0b0 << 0;
        const DONT_CARE: u64 = 0x0 << 1;
        Self(VALID | DONT_CARE)
    }
}

impl L1Entry {
    pub fn interior(l0_table: L0TableCap) -> Self {
        let frame_number = l0_table.entries.into_raw().into_raw() as u64;
        const VALID: u64 = 0b1 << 0;
        const RSW: u64 = 0b00 << 8;
        let ppn = (frame_number & ((1 << 44) - 1)) << 10;
        Self(VALID | RSW | ppn)
    }

    pub unsafe fn kernel_interior(l0_table: L0TableCap) -> Self {
        let frame_number = l0_table.entries.into_raw().into_raw() as u64;
        const VALID: u64 = 0b1 << 0;
        const GLOBAL: u64 = 0b1 << 5;
        const RSW: u64 = 0b00 << 8;
        let ppn = (frame_number & ((1 << 44) - 1)) << 10;
        Self(VALID | GLOBAL | RSW | ppn)
    }

    pub const fn invalid() -> Self {
        const VALID: u64 = 0b0 << 0;
        const DONT_CARE: u64 = 0x0 << 1;
        Self(VALID | DONT_CARE)
    }
}

impl L0Entry {
    pub fn leaf(l0_page: L0PageCap, permissions: Permissions) -> Self {
        let frame_number = l0_page.into_frame_number().into_raw() as u64;
        const VALID: u64 = 0b1 << 0;
        let permissions = permissions.bits();
        const USER: u64 = 0b1 << 4;
        const GLOBAL: u64 = 0b0 << 5;
        const ACCESSED: u64 = 0b1 << 6;
        const DIRTY: u64 = 0b1 << 7;
        const RSW: u64 = 0b00 << 8;
        let ppn = (frame_number & ((1 << 44) - 1)) << 10;
        Self(VALID | permissions | USER | GLOBAL | ACCESSED | DIRTY | RSW | ppn)
    }

    pub unsafe fn kernel_leaf(l0_page: L0PageCap, permissions: Permissions) -> Self {
        let frame_number = l0_page.into_frame_number().into_raw() as u64;
        const VALID: u64 = 0b1 << 0;
        let permissions = permissions.bits();
        const USER: u64 = 0b0 << 4;
        const GLOBAL: u64 = 0b1 << 5;
        const ACCESSED: u64 = 0b1 << 6;
        const DIRTY: u64 = 0b1 << 7;
        const RSW: u64 = 0b00 << 8;
        let ppn = (frame_number & ((1 << 44) - 1)) << 10;
        Self(VALID | permissions | USER | GLOBAL | ACCESSED | DIRTY | RSW | ppn)
    }

    pub const fn invalid() -> Self {
        const VALID: u64 = 0b0 << 0;
        const CAP: u64 = 0b0 << 1;
        const DONT_CARE: u64 = 0x0 << 2;
        Self(VALID | CAP | DONT_CARE)
    }

    pub const fn cap(frame_number: Idx, tag: u8) -> Self {
        const VALID: u64 = 0b0 << 0;
        const CAP: u64 = 0b1 << 1;
        let tag: u64 = (tag as u64) << 2;
        let frame_number: u64 = (frame_number.into_raw() as u64) << 10;
        Self(VALID | CAP | tag | frame_number)
    }
}
