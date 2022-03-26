#![doc = include_str!("../../README.md")]
#![no_std]
#![no_main]
#![feature(
    asm_sym,
    asm_const,
    cfg_target_abi,
    fn_align,
    naked_functions,
    thread_local,
    const_eval_limit
)]
#![deny(absolute_paths_not_starting_with_crate, unsafe_op_in_unsafe_fn)]
#![const_eval_limit = "4294967296"]

use static_assertions as _;

use crate::{
    frame::{Idx, FREE_FRAMES_START},
    layout::KERNEL_LAYOUT,
    machine::{FRAME_COUNT, L0_FRAME_SIZE, L1_FRAME_SIZE},
    sbi::srst::{reset_system, Reason, Type},
    table::{set_kernel_l1_table, TABLE_LEN},
};

static_assertions::assert_cfg!(target_arch = "riscv64");
static_assertions::assert_cfg!(target_vendor = "unknown");
static_assertions::assert_cfg!(target_os = "none");
static_assertions::assert_cfg!(target_env = "sbi");

#[macro_use]
pub mod debug;

pub mod align;
pub mod entry;
pub mod frame;
pub mod layout;
pub mod machine;
pub mod page;
pub mod panic;
pub mod ptr;
pub mod sbi;
pub mod plat;
pub mod sync;
pub mod table;
pub mod thread;

pub fn main() -> ! {
    use crate::{
        page::L0PageCap,
        sbi::{base, legacy, srst},
        sync::Token,
        table::{L0TableCap, L1TableCap, L2TableCap},
        thread::{Context, ThreadCap},
    };

    let mut token = Token::acquire();

    kernel!("Hello, world!");

    kernel!("Kernel layout: {:#?}", layout::KERNEL_LAYOUT);

    let ver = base::spec_version();
    kernel!("SBI specification version: {}", ver);
    assert!(ver.minor() >= 2);

    let impl_id = base::impl_id();
    kernel!("SBI implementation ID: {}", impl_id);

    let impl_ver = base::impl_version();
    kernel!("SBI implementation version: {:#x}", impl_ver);

    let legacy_console_put = base::probe_extension(legacy::CONSOLE_PUT_EID);
    assert!(matches!(legacy_console_put, base::ExtAvail::Available(_)));

    let srst = base::probe_extension(srst::EID);
    assert!(matches!(srst, base::ExtAvail::Available(_)));

    let mvendor_id = base::machine_vendor_id();
    kernel!("SBI machine vendor ID: {}", mvendor_id);

    let march_id = base::machine_arch_id();
    kernel!("SBI machine architecture ID: {:#x}", march_id);

    let mimpl_id = base::machine_impl_id();
    kernel!("SBI machine implementation ID: {:#x}", mimpl_id);

    let mut boot_alloc = BootAlloc::new(FREE_FRAMES_START, FRAME_COUNT);

    kernel!("Boot allocator has {} frames of memory.", boot_alloc.len());

    //                                  0xffffffc000000000
    const KERNELMODE_BASE_ADDR: usize = 0xffffffffc0000000;
    const KERNELMODE_BASE_PHYS: usize = 0x0000000080000000;

    let kernel_l1_table = boot_alloc.alloc(L1TableCap::new);
    for l1_index in 0..TABLE_LEN {
        let l0_table = boot_alloc.alloc(L0TableCap::new);
        for l0_index in 0..TABLE_LEN {
            let addr = l0_index * L0_FRAME_SIZE + l1_index * L1_FRAME_SIZE + KERNELMODE_BASE_ADDR;
            for section in KERNEL_LAYOUT {
                if (section.start as usize..section.end as usize).contains(&addr) {
                    let phys_addr =
                        l0_index * L0_FRAME_SIZE + l1_index * L1_FRAME_SIZE + KERNELMODE_BASE_PHYS;
                    let idx = Idx::from_raw(phys_addr / L0_FRAME_SIZE).unwrap();
                    let l0_page = unsafe { L0PageCap::already_init(idx) }.unwrap();
                    unsafe {
                        l0_table.map_l0_kernel_page(
                            &mut token,
                            l0_index,
                            l0_page,
                            section.permissions,
                        )
                    };
                    break;
                }
            }
        }
        kernel_l1_table.map_l0_table(&mut token, l1_index, l0_table);
    }

    unsafe { set_kernel_l1_table(kernel_l1_table, &mut token) };

    const USERMODE_IMAGE: &'static [u8] = include_bytes!("../usermode_image");
    const USERMODE_BASE_ADDR: usize = 0x4000_0000usize;

    let l2_table = boot_alloc.alloc(|idx| L2TableCap::new(idx, &token));
    for (l2_index, l2_frame) in USERMODE_IMAGE
        .chunks(crate::machine::L2_FRAME_SIZE)
        .enumerate()
    {
        let l2_index = l2_index + USERMODE_BASE_ADDR / crate::machine::L2_FRAME_SIZE;
        let l1_table = boot_alloc.alloc(L1TableCap::new);
        for (l1_index, l1_frame) in l2_frame.chunks(crate::machine::L1_FRAME_SIZE).enumerate() {
            let l0_table = boot_alloc.alloc(L0TableCap::new);
            for (l0_index, l0_frame) in l1_frame.chunks(crate::machine::L0_FRAME_SIZE).enumerate() {
                let mut bytes = [0x0; crate::machine::L0_FRAME_SIZE];
                bytes[..l0_frame.len()].copy_from_slice(l0_frame);
                kernel!("Copying {} bytes into a l0 page.", l0_frame.len());
                let l0_page = boot_alloc.alloc(|idx| L0PageCap::new(idx, bytes));
                kernel!("Mapping that l0 page at l0 index {}.", l0_index);
                l0_table.map_l0_page(
                    &mut token,
                    l0_index,
                    l0_page,
                    table::Permissions::ReadWriteExecute,
                );
            }
            kernel!("Mapping that l0 table at l1 index {}.", l1_index);
            l1_table.map_l0_table(&mut token, l1_index, l0_table);
        }
        kernel!("Mapping that l1 table at l2 index {}.", l2_index);
        l2_table.map_l1_table(&mut token, l2_index, l1_table);
    }

    kernel!("Boot allocator has {} frames of memory.", boot_alloc.len());

    let thread = boot_alloc.alloc(|frame_number| {
        ThreadCap::new(
            frame_number,
            Context {
                pc: USERMODE_BASE_ADDR,
                ..Default::default()
            },
            l2_table,
        )
    });

    loop {
        let scause;
        let stval;
        (token, scause, stval) = thread.resume(token).unwrap();

        // TODO: define a new hart-local capability(s) that will allow a thread to
        // block waiting on timer or device interrupts, switch to other threads, extend
        // the timer, claim IRQs from the PLIC, and acknowledge those IRQs.

        // TODO: define a system call interface

        match scause {
            0x8 => {
                let context = thread.context_mut(&mut token).unwrap();
                match context.a[0] {
                    0x0 => {
                        reset_system(Type::Shutdown, Reason::None).unwrap();
                    }
                    0x1 => {
                        let bytes = context.a[1].to_be_bytes();
                        if let Ok(str) = core::str::from_utf8(&bytes) {
                            user!("{}", str.escape_debug());
                        } else {
                            user!("{:x?}", bytes);
                        }
                    }
                    _ => {
                        kernel!("Unexpected syscall attempt with context: {:?}", context);
                    }
                }
                context.pc += 0x4;
            }
            _ => {
                panic!(
                    "Unexpected user trap with context: {:?}, scause: {:#x}, stval: {:#x}",
                    thread.context(&token),
                    scause,
                    stval,
                );
            }
        }
    }
}

impl BootAlloc {
    pub const fn new(start_frame_number: usize, end_frame_number: usize) -> Self {
        assert!(start_frame_number <= end_frame_number);
        Self {
            start_frame_number,
            end_frame_number,
        }
    }

    pub fn len(&self) -> usize {
        self.end_frame_number - self.start_frame_number
    }

    pub fn alloc<T, F>(&mut self, f: F) -> T
    where
        F: FnOnce(Idx) -> Option<T>,
    {
        assert!(self.len() != 0);
        let frame_number = self.end_frame_number - 1;
        let idx = Idx::from_raw(frame_number).expect("Invalid frame number.");
        let frame = f(idx).expect("Frame already in use.");
        self.end_frame_number = frame_number;
        frame
    }
}

pub struct BootAlloc {
    start_frame_number: usize,
    end_frame_number: usize,
}
