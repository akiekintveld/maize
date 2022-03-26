//! Contains all possible entrypoints into the kernel.
//!
//! The kernel can be entered in two ways:
//! 1. On boot, SBI jumps into the kernel on the boot hart.
//! 2. On a trap, the CPU jumps to the trap handler.
//!
//! In both cases, we need to do a bit of work in assembly before it's safe
//! to call into Rust, and we need to set ourselves up for success for the next
//! time we try to resume the user context.
