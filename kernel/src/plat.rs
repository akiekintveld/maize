use ::core::{arch::asm, mem::size_of};

pub unsafe fn swap_satp(mut satp: u64) -> u64 {
    unsafe {
        core::arch::asm!(
            "csrrw {satp}, satp, {satp}",
            "sfence.vma zero, zero",
            satp = inout(reg) satp,
        )
    }
    satp
}

pub unsafe fn set_satp(satp: u64) {
    unsafe {
        core::arch::asm!(
            "csrw satp, {satp}",
            "sfence.vma zero, zero",
            satp = in(reg) satp,
        )
    }
}

pub unsafe fn resume(context: &mut crate::thread::Context) -> (u64, u64) {
    let sstatus: u64;
    unsafe {
        core::arch::asm!(
            "csrr {sstatus}, sstatus",
            sstatus = lateout(reg) sstatus,
        )
    }
    debug_assert_eq!(sstatus & SSTATUS_SPP_MASK, 0x0);

    unsafe {
        core::arch::asm!(
            // Stash our context pointer. We're forced to use the last
            // register we restore to hold the context pointer, and then it
            // will load over itself.
            "csrw sscratch, a7",

            // Direct traps to come back to this function.
            "la t0, 1f",
            "csrw stvec, t0",

            // Save and restore integer registers.
            "ld ra, 0*{register_size}(a7)",
            "ld a6, 1*{register_size}(a7)",
            "csrw sepc, a6",
            "ld a6, 2*{register_size}(a7)",
            "sd sp, 2*{register_size}(a7)",
            "mv sp, a6",
            "ld a6, 3*{register_size}(a7)",
            "sd gp, 3*{register_size}(a7)",
            "mv gp, a6",
            "ld a6, 4*{register_size}(a7)",
            "sd tp, 4*{register_size}(a7)",
            "mv tp, a6",
            "ld t0, 5*{register_size}(a7)",
            "ld t1, 6*{register_size}(a7)",
            "ld t2, 7*{register_size}(a7)",
            "ld t3, 8*{register_size}(a7)",
            "ld t4, 9*{register_size}(a7)",
            "ld t5, 10*{register_size}(a7)",
            "ld t6, 11*{register_size}(a7)",
            "ld a6, 12*{register_size}(a7)",
            "sd s0, 12*{register_size}(a7)",
            "mv s0, a6",
            "ld a6, 13*{register_size}(a7)",
            "sd s1, 13*{register_size}(a7)",
            "mv s1, a6",
            "ld s2, 14*{register_size}(a7)",
            "ld s3, 15*{register_size}(a7)",
            "ld s4, 16*{register_size}(a7)",
            "ld s5, 17*{register_size}(a7)",
            "ld s6, 18*{register_size}(a7)",
            "ld s7, 19*{register_size}(a7)",
            "ld s8, 20*{register_size}(a7)",
            "ld s9, 21*{register_size}(a7)",
            "ld s10, 22*{register_size}(a7)",
            "ld s11, 23*{register_size}(a7)",
            "ld a0, 24*{register_size}(a7)",
            "ld a1, 25*{register_size}(a7)",
            "ld a2, 26*{register_size}(a7)",
            "ld a3, 27*{register_size}(a7)",
            "ld a4, 28*{register_size}(a7)",
            "ld a5, 29*{register_size}(a7)",
            "ld a6, 30*{register_size}(a7)",
            "ld a7, 31*{register_size}(a7)",

            // Jump back to the saved program counter in user mode.
            "sret",

            // Note that trap handlers must be aligned on 4-byte boundaries.
            ".align 0x4",
            "1:",

            // Stash trap stack pointer and retrieve ours.
            "csrrw a7, sscratch, sp",

            // Save and restore integer registers.
            "sd ra, 0*{register_size}(a7)",
            "csrr ra, sepc",
            "sd ra, 1*{register_size}(a7)",
            "ld ra, 2*{register_size}(a7)",
            "sd sp, 2*{register_size}(a7)",
            "mv sp, ra",
            "ld ra, 3*{register_size}(a7)",
            "sd gp, 3*{register_size}(a7)",
            "mv gp, ra",
            "ld ra, 4*{register_size}(a7)",
            "sd tp, 4*{register_size}(a7)",
            "mv tp, ra",
            "sd t0, 5*{register_size}(a7)",
            "sd t1, 6*{register_size}(a7)",
            "sd t2, 7*{register_size}(a7)",
            "sd t3, 8*{register_size}(a7)",
            "sd t4, 9*{register_size}(a7)",
            "sd t5, 10*{register_size}(a7)",
            "sd t6, 11*{register_size}(a7)",
            "ld ra, 12*{register_size}(a7)",
            "sd s0, 12*{register_size}(a7)",
            "mv s0, ra",
            "ld ra, 13*{register_size}(a7)",
            "sd s1, 13*{register_size}(a7)",
            "mv s1, ra",
            "sd s2, 14*{register_size}(a7)",
            "sd s3, 15*{register_size}(a7)",
            "sd s4, 16*{register_size}(a7)",
            "sd s5, 17*{register_size}(a7)",
            "sd s6, 18*{register_size}(a7)",
            "sd s7, 19*{register_size}(a7)",
            "sd s8, 20*{register_size}(a7)",
            "sd s9, 21*{register_size}(a7)",
            "sd s10, 22*{register_size}(a7)",
            "sd s11, 23*{register_size}(a7)",
            "sd a0, 24*{register_size}(a7)",
            "sd a1, 25*{register_size}(a7)",
            "sd a2, 26*{register_size}(a7)",
            "sd a3, 27*{register_size}(a7)",
            "sd a4, 28*{register_size}(a7)",
            "sd a5, 29*{register_size}(a7)",
            "sd a6, 30*{register_size}(a7)",
            "csrr ra, sscratch",
            "sd ra, 31*{register_size}(a7)",

            register_size = const size_of::<usize>(),

            out("s2") _,
            out("s3") _,
            out("s4") _,
            out("s5") _,
            out("s6") _,
            out("s7") _,
            out("s8") _,
            out("s9") _,
            out("s10") _,
            out("s11") _,
            in("a7") context,
            clobber_abi("C"),
        );
    }

    let scause: u64;
    let stval: u64;
    unsafe {
        core::arch::asm!(
            "csrw stvec, {stvec}",
            "csrr {scause}, scause",
            "csrr {stval}, stval",
            scause = lateout(reg) scause,
            stval = lateout(reg) stval,
            stvec = in(reg) supervisor_trap,
        )
    }
    (scause, stval)
}

pub unsafe fn call(
    eid: u32,
    fid: u32,
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
) -> (usize, usize) {
    let mut error: usize;
    let mut value: usize;
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a0") a0,
            in("a1") a1,
            in("a2") a2,
            in("a3") a3,
            in("a4") a4,
            in("a5") a5,
            in("a6") fid,
            in("a7") eid,
            lateout("a0") error,
            lateout("a1") value,
        );
    }
    (error, value)
}

use crate::{
    align::L2FrameAligned,
    layout::{
        BOOT_STACK_POINTER, BOOT_THREAD_POINTER, ENTRY_START, GLOBAL_POINTER, THREAD_BSS_END,
        THREAD_BSS_START, THREAD_DATA_END, THREAD_DATA_START,
    },
    main,
    sync::set_hart_id,
    table::{boot_l2_table, L2Entry, TABLE_LEN},
    thread::SSTATUS_SPP_MASK,
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
        core::arch::asm!(
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

/// Enters execution of the kernel upon a trap from supervisor mode.
///
/// # Safety
/// Must be called by a trap from supervisor mode.
#[naked]
#[repr(align(0x4))]
pub unsafe extern "C" fn supervisor_trap() -> ! {
    unsafe extern "C" fn handle_supervisor_trap(context: &crate::thread::Context) -> ! {
        let scause: usize;
        let stval: usize;

        unsafe {
            core::arch::asm!(
                "csrr {scause}, scause",
                "csrr {stval}, stval",
                scause = lateout(reg) scause,
                stval = lateout(reg) stval,
            )
        }

        panic!(
            "Unexpected supervisor trap with context: {:?}, scause: {:#x}, stval: {:#x}",
            context, scause, stval,
        );
    }

    // SAFETY: We entered via a trap. See below for the reasoning behind each
    // block of instructions.
    unsafe {
        core::arch::asm!(
            // Stash trap stack pointer.
            "csrw sscratch, sp",

            // Make space to save the context.
            "addi sp, sp, -{context_size}",

            // Save and restore integer registers.
            "sd ra, 0*{register_size}(sp)",
            "csrr ra, sepc",
            "sd ra, 1*{register_size}(sp)",
            "csrr ra, sscratch",
            "sd ra, 2*{register_size}(sp)",
            "sd gp, 3*{register_size}(sp)",
            "sd tp, 4*{register_size}(sp)",
            "sd t0, 5*{register_size}(sp)",
            "sd t1, 6*{register_size}(sp)",
            "sd t2, 7*{register_size}(sp)",
            "sd t3, 8*{register_size}(sp)",
            "sd t4, 9*{register_size}(sp)",
            "sd t5, 10*{register_size}(sp)",
            "sd t6, 11*{register_size}(sp)",
            "sd s0, 12*{register_size}(sp)",
            "sd s1, 13*{register_size}(sp)",
            "sd s2, 14*{register_size}(sp)",
            "sd s3, 15*{register_size}(sp)",
            "sd s4, 16*{register_size}(sp)",
            "sd s5, 17*{register_size}(sp)",
            "sd s6, 18*{register_size}(sp)",
            "sd s7, 19*{register_size}(sp)",
            "sd s8, 20*{register_size}(sp)",
            "sd s9, 21*{register_size}(sp)",
            "sd s10, 22*{register_size}(sp)",
            "sd s11, 23*{register_size}(sp)",
            "sd a0, 24*{register_size}(sp)",
            "mv a0, sp",
            "sd a1, 25*{register_size}(sp)",
            "sd a2, 26*{register_size}(sp)",
            "sd a3, 27*{register_size}(sp)",
            "sd a4, 28*{register_size}(sp)",
            "sd a5, 29*{register_size}(sp)",
            "sd a6, 30*{register_size}(sp)",
            "sd a7, 31*{register_size}(sp)",

            // Align the stack since we could've trapped from anywhere.
            "andi sp, sp, -0x10",

            // Jump into Rust to handle the trap.
            "j {handle_supervisor_trap}",

            context_size = const size_of::<crate::thread::Context>(),
            register_size = const size_of::<usize>(),
            handle_supervisor_trap = sym handle_supervisor_trap,

            options(noreturn),
        )
    }
}
