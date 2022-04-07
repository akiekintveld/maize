//! Various wrappers which force minimum alignment.

/// Align a variable up to an L0 frame.
///
/// This is most useful when setting up static data structures.
#[repr(align(4096))]
pub struct L0FrameAligned<T>(pub T);
