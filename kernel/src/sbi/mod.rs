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
    let (error, value) = unsafe { crate::plat::call(eid, fid, a0, a1, a2, a3, a4, a5) };
    match error as isize {
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
