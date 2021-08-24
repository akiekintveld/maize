OUTPUT_ARCH(riscv)

ENTRY(__entry$)

SECTIONS {
    . = 0x40000000;
    
    /* Place the entry so the supervisor knows where to jump to. */
    .entry ALIGN(CONSTANT(COMMONPAGESIZE)) : AT(0xc0000000) {
        *(.entry)
    }
    
    .boot ALIGN(CONSTANT(COMMONPAGESIZE)) : {
        __boot_start$ = .;
        . += 16 * CONSTANT(COMMONPAGESIZE);
        . = ALIGN(16);
        __boot_stack_pointer$ = .;
        . = ALIGN(ALIGNOF(.thread_image));
        __boot_thread_pointer$ = .;
        . += SIZEOF(.thread_image);
        __boot_end$ = .;
    }
    
    .static ALIGN(CONSTANT(COMMONPAGESIZE)) : {
        __static_start$ = .;
        __global_pointer$ = . + CONSTANT(COMMONPAGESIZE) / 2;
        *(.srodata .srodata.*)
        *(.sdata .sdata.*)
        *(.sbss .sbss.*)
        *(.data .data.*)
        *(.bss .bss.*)
        __static_end$ = .;
    }
    
    .thread_image ALIGN(CONSTANT(COMMONPAGESIZE)) : {
        __thread_image_start$ = .;
        *(.tdata .tdata.*)
        *(.tbss .tbss.*)
        __thread_image_end$ = .;
    }
    
    .const ALIGN(CONSTANT(COMMONPAGESIZE)) : {
        __const_start$ = .;
        *(.rodata .rodata.*)
        __thread_align$ = .;
        QUAD(ALIGNOF(.thread_image))
        __const_end$ = .;
    }

    .text ALIGN(CONSTANT(COMMONPAGESIZE)) : {
        __text_start$ = .;
        *(.text .text.*)
        __text_end$ = .;
    }
    
    /DISCARD/ : {
        *(.eh_frame)
    }
}
