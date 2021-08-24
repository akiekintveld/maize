use crate::sbi::call;

pub const CONSOLE_PUT_EID: u32 = 0x1;

pub fn console_put(b: u8) {
    // Safety: It is always legal to put a character to the debug console via
    // SBI in supervisor mode.
    let r = unsafe { call(CONSOLE_PUT_EID, 0x0, b as usize, 0, 0, 0, 0, 0) };
    drop(r)
}
