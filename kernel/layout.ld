OUTPUT_ARCH(riscv)

ENTRY(__entry$)

MEMORY {
    system_image : ORIGIN = 0xffffffffc0200000, LENGTH = 0x3fe00000
}

SECTIONS {
    .entry ALIGN(CONSTANT(COMMONPAGESIZE)) : AT(0x80200000) {
        __entry_start$ = .;
        *(.entry)
        . = ALIGN(CONSTANT(COMMONPAGESIZE));
        __entry_end$ = .;
    } > system_image

    .text ALIGN(CONSTANT(COMMONPAGESIZE)) : {
        __text_start$ = .;
        *(.text .text.*)
        . = ALIGN(CONSTANT(COMMONPAGESIZE));
        __text_end$ = .;
    } > system_image
    
    .boot ALIGN(CONSTANT(COMMONPAGESIZE)) : {
        __boot_start$ = .;
        . += 32 * CONSTANT(COMMONPAGESIZE);
        . = ALIGN(16);
        __boot_stack_pointer$ = .;
        . = ALIGN(ALIGNOF(.thread_image));
        __boot_thread_pointer$ = .;
        . += SIZEOF(.thread_image);
        . = ALIGN(CONSTANT(COMMONPAGESIZE));
        __boot_end$ = .;
    } > system_image
    
    .static ALIGN(CONSTANT(COMMONPAGESIZE)) : {
        __static_start$ = .;
        __global_pointer$ = . + CONSTANT(COMMONPAGESIZE) / 2;
        *(.srodata .srodata.*)
        *(.sdata .sdata.*)
        *(.sbss .sbss.*)
        *(.data .data.*)
        *(.bss .bss.*)
        . = ALIGN(CONSTANT(COMMONPAGESIZE));
        __static_end$ = .;
    } > system_image
    
    .thread_image ALIGN(CONSTANT(COMMONPAGESIZE)) : {
        __thread_image_start$ = .;
        __thread_data_start$ = .;
        *(.tdata .tdata.*)
        __thread_data_end$ = .;
        __thread_bss_start$ = .;
        *(.tbss .tbss.*)
        __thread_bss_end$ = .;
        . = ALIGN(CONSTANT(COMMONPAGESIZE));
        __thread_image_end$ = .;
    } > system_image
    
    .const ALIGN(CONSTANT(COMMONPAGESIZE)) : {
        __const_start$ = .;
        *(.rodata .rodata.*)
        __thread_align$ = .;
        QUAD(ALIGNOF(.thread_image))
        . = ALIGN(CONSTANT(COMMONPAGESIZE));
        __const_end$ = .;
    } > system_image
    
    /DISCARD/ : {
        *(.eh_frame)
    }
}
