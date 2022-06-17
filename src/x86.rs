//! Wrappers around x86 instructions.
use core::arch::asm;

#[inline]
pub fn hlt() {
    unsafe {
        asm!("hlt", options(nostack));
    }
}
