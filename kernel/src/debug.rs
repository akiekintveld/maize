//! Contains a debug console implementation that uses the legacy SBI extension.

use {
    crate::sbi::legacy::console_put,
    ::core::fmt::{Arguments, Result, Write},
};

/// Print a formatted error message to the debug console.
#[macro_export]
macro_rules! kernel {
    ($($arg:tt)*) => (
        crate::debug::Console.log(
            "KERN",
            ::core::format_args!($($arg)*),
            ::core::file!(),
            ::core::line!(),
        )
    );
}

/// Print a formatted error message to the debug console.
#[macro_export]
macro_rules! user {
    ($($arg:tt)*) => (
        crate::debug::Console.log(
            "USER",
            ::core::format_args!($($arg)*),
            ::core::file!(),
            ::core::line!(),
        )
    );
}

/// A basic debug console that forwards to SBI.
pub struct Console;

impl Write for Console {
    fn write_str(&mut self, s: &str) -> Result {
        for b in s.bytes() {
            console_put(b)
        }
        Ok(())
    }
}

impl Console {
    pub fn log(&mut self, level: &str, args: Arguments, file: &str, line: u32) {
        writeln!(self, "[{}]\t{} ({}:{})", level, args, file, line)
            .expect("Console writes should never fail.");
    }
}
