#[inline(always)]
pub unsafe fn call(a0: usize, a1: usize) {
    unsafe {
        core::arch::asm!("ecall", inout("a0") a0 => _, inout("a1") a1 => _);
    }
}
