#![no_std]
#![no_main]
#![feature(
    asm_sym,
    asm_const,
    fn_align,
    naked_functions,
    never_type,
    thread_local
)]
#![deny(absolute_paths_not_starting_with_crate, unsafe_op_in_unsafe_fn)]

use ::core::mem::size_of;

pub mod abi;
pub mod entry;
pub mod panic;

pub fn main() -> ! {
    // "usermode"
    const CHECKIN: &'static str = "Hello, world!";
    for chunk in CHECKIN.as_bytes().chunks(size_of::<usize>()) {
        let mut bytes = [b' '; size_of::<usize>()];
        bytes[..chunk.len()].copy_from_slice(chunk);
        let value = usize::from_be_bytes(bytes);
        unsafe { abi::call(0x1, value) };
    }
    unsafe { abi::call(0x0, 0x0) };
    loop {}
}
