//! Manage x86 interrupts.

use core::arch::asm;

/// Format for use by LIDT and LGDT
#[repr(C, packed(2))]
struct DescriptorTableRegister {
    /// Size of the DT in bytes.
    pub limit: u16,
    // Base address of the DT.
    pub base: *const u64,
}

#[derive(Debug)]
pub enum Ring {
    Ring0 = 0,
}

// Segment selectors (4.5 AMD64 manual)
type SegmentSelector = u16;
const fn new_segment(index: u16, rpl: Ring) -> SegmentSelector {
    index << 3 | (rpl as u16)
}

const KERNEL_CS: SegmentSelector = new_segment(1, Ring::Ring0);
const KERNEL_DS: SegmentSelector = new_segment(2, Ring::Ring0);
const TSS_SELECTOR: SegmentSelector = new_segment(3, Ring::Ring0);

mod gdt;
mod idt;

pub fn init() {
    // Segment selectors (4.5 AMD64 manual)
    debug!("Initializing GDT");
    gdt::load();
    debug!("Initializing IDT");
    idt::load();
}
