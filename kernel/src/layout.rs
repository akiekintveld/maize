use {crate::table::Permissions, ::core::ptr::addr_of};

#[allow(improper_ctypes)]
extern "C" {
    #[link_name = "__global_pointer$"]
    pub static GLOBAL_POINTER: ();

    #[link_name = "__boot_thread_pointer$"]
    pub static BOOT_THREAD_POINTER: ();

    #[link_name = "__boot_stack_pointer$"]
    pub static BOOT_STACK_POINTER: ();

    #[link_name = "__thread_data_start$"]
    pub static THREAD_DATA_START: ();

    #[link_name = "__thread_data_end$"]
    pub static THREAD_DATA_END: ();

    #[link_name = "__thread_bss_start$"]
    pub static THREAD_BSS_START: ();

    #[link_name = "__thread_bss_end$"]
    pub static THREAD_BSS_END: ();

    #[link_name = "__entry_start$"]
    pub static ENTRY_START: ();

    #[link_name = "__entry_end$"]
    pub static ENTRY_END: ();

    #[link_name = "__text_start$"]
    pub static TEXT_START: ();

    #[link_name = "__text_end$"]
    pub static TEXT_END: ();

    #[link_name = "__boot_start$"]
    pub static BOOT_START: ();

    #[link_name = "__boot_end$"]
    pub static BOOT_END: ();

    #[link_name = "__static_start$"]
    pub static STATIC_START: ();

    #[link_name = "__static_end$"]
    pub static STATIC_END: ();

    #[link_name = "__thread_image_start$"]
    pub static THREAD_IMAGE_START: ();

    #[link_name = "__thread_image_end$"]
    pub static THREAD_IMAGE_END: ();

    #[link_name = "__const_start$"]
    pub static CONST_START: ();

    #[link_name = "__const_end$"]
    pub static CONST_END: ();

}

#[derive(Debug, Clone, Copy)]
pub struct Section {
    pub name: &'static str,
    pub start: *const (),
    pub end: *const (),
    pub permissions: Permissions,
}

unsafe impl Sync for Section {}
unsafe impl Send for Section {}

pub static KERNEL_LAYOUT: &'static [Section] = &[
    Section {
        name: "entry",
        start: unsafe { addr_of!(ENTRY_START) },
        end: unsafe { addr_of!(ENTRY_END) },
        permissions: Permissions::ExecuteOnly,
    },
    Section {
        name: "text",
        start: unsafe { addr_of!(TEXT_START) },
        end: unsafe { addr_of!(TEXT_END) },
        permissions: Permissions::ExecuteOnly,
    },
    Section {
        name: "boot",
        start: unsafe { addr_of!(BOOT_START) },
        end: unsafe { addr_of!(BOOT_END) },
        permissions: Permissions::ReadWrite,
    },
    Section {
        name: "static",
        start: unsafe { addr_of!(STATIC_START) },
        end: unsafe { addr_of!(STATIC_END) },
        permissions: Permissions::ReadWrite,
    },
    Section {
        name: "thread_image",
        start: unsafe { addr_of!(THREAD_IMAGE_START) },
        end: unsafe { addr_of!(THREAD_IMAGE_END) },
        permissions: Permissions::ReadOnly,
    },
    Section {
        name: "const",
        start: unsafe { addr_of!(CONST_START) },
        end: unsafe { addr_of!(CONST_END) },
        permissions: Permissions::ReadOnly,
    },
];
