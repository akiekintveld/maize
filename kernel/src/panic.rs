//! Contains all panic handling.

use {
    crate::sbi::srst::{reset_system, Reason, Type},
    ::core::{cell::Cell, panic::PanicInfo},
};

/// Handles a panic by attempting to print an error message and shutting down
/// the system.
#[panic_handler]
pub fn handle_panic(panic_info: &PanicInfo) -> ! {
    #[thread_local]
    static IS_PANICKING: Cell<bool> = Cell::new(false);

    if IS_PANICKING.replace(true) {
        // If we hit a nested panic or a multi-hart panic, just spin I guess?
        loop {}
    }

    kernel!("{}", panic_info);
    reset_system(Type::Shutdown, Reason::SystemFailure).unwrap();
    unreachable!();
}
