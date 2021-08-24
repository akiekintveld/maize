// We support up to 8GiB of physical address space.
// TODO: Longer-term we should either fetch this dynamically from the device
// tree, or have it be statically configurable for each target board.
pub const FRAME_COUNT: usize = 0x20_0000;
pub const L2_FRAME_SIZE: usize = 0x1000 * 512 * 512;
pub const L1_FRAME_SIZE: usize = 0x1000 * 512;
pub const L0_FRAME_SIZE: usize = 0x1000;
