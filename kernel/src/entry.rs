//! Contains all possible entrypoints into the kernel.
//!
//! The kernel can be entered in two ways:
//! 1. On boot, SBI jumps into the kernel on the boot hart.
//! 2. On a trap, the CPU jumps to the trap handler.
//!
//! In both cases, we need to do a bit of work in assembly before it's safe
//! to call into Rust, and we need to set ourselves up for success for the next
//! time we try to resume the user context.

use crate::{
    align::L2FrameAligned,
    layout::{
        BOOT_STACK_POINTER, BOOT_THREAD_POINTER, ENTRY_START, GLOBAL_POINTER, THREAD_BSS_END,
        THREAD_BSS_START, THREAD_DATA_END, THREAD_DATA_START,
    },
    main,
    sync::set_hart_id,
    table::{boot_l2_table, L2Entry, TABLE_LEN},
    thread::{supervisor_trap, SSTATUS_SPP_MASK},
};

/// Enters execution of the kernel in supervisor mode on boot.
///
/// # Safety
/// Must be called by the SBI exactly once on exactly one hart.
#[naked]
#[export_name = "__entry$"]
#[link_section = ".entry"]
pub unsafe extern "C" fn boot(_hart_id: u64, _fdt: u64) -> ! {
    unsafe extern "C" fn handle_boot(hart_id: u64, _fdt: u64) -> ! {
        // SAFETY: SBI ensures that the hart ID is unique and accurate.
        unsafe { set_hart_id(hart_id) };

        main()
    }

    /// A L2 page table with nothing except the kernel (high half) mapped. This
    /// is only used during boot before we bootstrap the initial context.
    static BOOT_L2_TABLE: L2FrameAligned<[L2Entry; TABLE_LEN]> = L2FrameAligned(boot_l2_table());

    // SAFETY: We entered via the SBI's boot sequence. See below for the
    // reasoning behind each block of instructions.
    unsafe {
        asm!(
            // When entering supervisor mode via the SBI, it is guaranteed that
            // address translation and protection (ATP) are disabled; and that
            // interrupts will not be taken. The code model is medium so
            // everything is adressed relative to the program counter and the
            // kernel is a statically position independent executable.
            // It is also guaranteed that the hart ID and the physical address
            // of the flattened device tree are in the first two argument
            // registers.

            // Until we've setup the global pointer, disable linker relaxation.
            ".option push",
            ".option norelax",

            // Our first task is to get into the kernel's virtual address space.

            // Because ATP is disabled, we're currently executing in the low
            // half of the address space. After we enable ATP, nothing will be
            // mapped here. This will cause a trap, so we setup traps to jump
            // directly to the translated address where we want to continue
            // execution.
            "li t0, {virt_start}",
            "la t1, {phys_start}",
            "sub t1, t0, t1",
            "la t0, 1f",
            "add t0, t0, t1",
            "csrw stvec, t0",

            // We already have an Sv39 level 2 page table with our desired
            // memory layout and, since ATP is disabled, the address we get
            // here is the physical address. We write the physical page number
            // with ASID 0 and Sv39 mode to enable ATP.
            "la t0, {boot_l2_table}",
            "srli t0, t0, 12",
            "li t1, {satp_mode_sv39}",
            "or t0, t1, t0",
            // TODO: Do we fence before or after writing to satp? It seems to
            // work both ways in QEMU, but we should figure out which is
            // strictly correct. Note that needing it after gets a bit weird
            // because the next instruction could fallthrough or trap depending
            // on if its translation was cached.
            "sfence.vma zero, zero",
            "csrw satp, t0",

            // If we get here something is very wrong and we can't continue.
            "j .",

            // Note that trap handlers must be aligned on 4-byte boundaries.
            ".align 0x4",

            // This is where we continue after the trap. We should be executing
            // in the high half of the address space now.
            "1:",

            // Our second task is to setup the rest of the supervisor state.

            // We took a trap to get here, but it wasn't a real trap, so set
            // the previous privilege back to usermode.
            "li t0, {sstatus_spp_mask}",
            "csrc sstatus, t0",

            // Setup traps to directly jump to the real supervisor mode handler.
            "la t0, {stvec_base}",
            "csrw stvec, t0",

            // Start with all interrupts disabled and none pending.
            "csrw sie, zero",
            "csrw sip, zero",

            // The kernel shouldn't ever use the FPU. Turn it off so we get a
            // trap if we happen to use it by accident.
            "li t0, {sstatus_fs_mask}",
            "csrc sstatus, t0",

            // Our final task is to setup the Rust runtime.

            // Setup the global pointer.
            "la gp, {global_pointer}",

            // Now we can finally relax. Enable linker relaxations.
            ".option pop",

            // Setup our stack pointer.
            "la sp, {boot_stack_pointer}",

            // We use ELF TLS for hart-local storage. We also force the TLS
            // model to local-exec so we don't need to mess around with thread
            // control blocks. Point the thread pointer at a suitably sized and
            // aligned buffer.
            "la tp, {boot_thread_pointer}",

            // Copy the TLS data and zero the TLS BSS.
            "la t0, {thread_data_start}",
            "la t1, {thread_data_end}",
            "mv t2, tp",
            "2:",
            "beq t0, t1, 3f",
            "lb t3, 0(t0)",
            "sb t3, 0(t2)",
            "addi t0, t0, 1",
            "addi t2, t2, 1",
            "j 2b",
            "3:",
            "la t0, {thread_bss_start}",
            "la t1, {thread_bss_end}",
            "4:",
            "beq t0, t1, 5f",
            "sb zero, 0(t2)",
            "addi t0, t0, 1",
            "addi t2, t2, 1",
            "j 4b",
            "5:",

            // At this point, we've done enough to safely enter Rust. We jump
            // to do some additional runtime setup, passing along the hart ID,
            // the physical address of the flattened device tree in the first
            // two argument registers.

            // Because the context is aligned to 16 bytes, the stack is too.
            // Call into Rust to handle the rest of the boot flow.
            "call {handle_boot}",

            phys_start = sym ENTRY_START,
            virt_start = const 0xffff_ffff_c020_0000u64,

            boot_l2_table = sym BOOT_L2_TABLE,
            satp_mode_sv39 = const 0x8000_0000_0000_0000u64,

            sstatus_spp_mask = const SSTATUS_SPP_MASK,

            stvec_base = sym supervisor_trap,

            sstatus_fs_mask = const 0x6000u64,

            global_pointer = sym GLOBAL_POINTER,

            boot_stack_pointer = sym BOOT_STACK_POINTER,

            boot_thread_pointer = sym BOOT_THREAD_POINTER,

            thread_data_start = sym THREAD_DATA_START,
            thread_data_end = sym THREAD_DATA_END,

            thread_bss_start = sym THREAD_BSS_START,
            thread_bss_end = sym THREAD_BSS_END,

            handle_boot = sym handle_boot,

            options(noreturn)
        )
    }
}
