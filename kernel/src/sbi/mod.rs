pub mod base;
pub mod legacy;
pub mod srst;

/// A standard error returned from an SBI call.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum StandardError {
    Unknown,
    Failed,
    NotSupported,
    InvalidParam,
    Denied,
    InvalidAddr,
    AlreadyAvailable,
    AlreadyStarted,
    AlreadyStopped,
}

#[inline(always)]
pub unsafe fn call(
    eid: u32,
    fid: u32,
    a0: usize,
    a1: usize,
    a2: usize,
    a3: usize,
    a4: usize,
    a5: usize,
) -> Result<usize, StandardError> {
    let mut error: isize;
    let mut value;
    unsafe {
        asm!(
            "ecall",
            in("a0") a0,
            in("a1") a1,
            in("a2") a2,
            in("a3") a3,
            in("a4") a4,
            in("a5") a5,
            in("a6") fid,
            in("a7") eid,
            lateout("a0") error,
            lateout("a1") value,
        );
    }
    match error {
        0 => Ok(value),
        -1 => Err(StandardError::Failed),
        -2 => Err(StandardError::NotSupported),
        -3 => Err(StandardError::InvalidParam),
        -4 => Err(StandardError::Denied),
        -5 => Err(StandardError::InvalidAddr),
        -6 => Err(StandardError::AlreadyAvailable),
        -7 => Err(StandardError::AlreadyStarted),
        -8 => Err(StandardError::AlreadyStopped),
        _ => Err(StandardError::Unknown),
    }
}
