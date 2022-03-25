use crate::main;

#[naked]
#[export_name = "__entry$"]
#[link_section = ".entry"]
unsafe extern "C" fn entry() -> ! {
    unsafe extern "C" fn trampoline() -> ! {
        main()
    }

    unsafe {
        core::arch::asm!(
            // The initial register state is satp = 0, sstatus.SIE = 0,
            // a0 = hartid, and a1 = a pointer to the FDT.

            // Setup runtime registers.
            ".option push",
            ".option norelax",
            "la gp, {global_pointer}",
            ".option pop",
            "la tp, {boot_thread_pointer}",
            "la sp, {boot_stack_pointer}",

            // Copy thread image.
            "la t0, {thread_image_start}",
            "la t1, {thread_image_end}",
            "mv t2, tp",
            "1:",
            "beq t0, t1, 2f",
            "lb t3, 0(t0)",
            "sb t3, 0(t2)",
            "addi t0, t0, 1",
            "addi t2, t2, 1",
            "j 1b",
            "2:",

            // Jump into Rust.
            "j {trampoline}",
            global_pointer = sym GLOBAL_POINTER,
            boot_thread_pointer = sym BOOT_THREAD_POINTER,
            boot_stack_pointer = sym BOOT_STACK_POINTER,
            thread_image_start = sym THREAD_IMAGE_START,
            thread_image_end = sym THREAD_IMAGE_END,
            trampoline = sym trampoline,
            options(noreturn)
        )
    }
}

#[allow(improper_ctypes)]
extern "C" {
    #[link_name = "__global_pointer$"]
    static GLOBAL_POINTER: ();

    #[link_name = "__boot_thread_pointer$"]
    static BOOT_THREAD_POINTER: ();

    #[link_name = "__boot_stack_pointer$"]
    static BOOT_STACK_POINTER: ();

    #[link_name = "__thread_image_start$"]
    static THREAD_IMAGE_START: ();

    #[link_name = "__thread_image_end$"]
    static THREAD_IMAGE_END: ();
}
