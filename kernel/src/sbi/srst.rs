use crate::sbi::call;

#[repr(usize)]
pub enum Type {
    Shutdown = 0,
    ColdReboot = 1,
    WarmReboot = 2,
}

#[repr(usize)]
pub enum Reason {
    None = 0,
    SystemFailure = 1,
}

pub const EID: u32 = 0x53525354;

pub fn reset_system(t: Type, r: Reason) -> Result<(), super::StandardError> {
    // Safety: It is always legal to attempt to reset the system SBI in
    // supervisor mode.
    let res = unsafe { call(EID, 0x0, t as usize, r as usize, 0, 0, 0, 0) };
    res.map(|_| unreachable!())
}
