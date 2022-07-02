use crate::interrupts::{DescriptorTableRegister, KERNEL_CS};
use bitflags::bitflags;
use core::arch::asm;
use core::mem::{size_of, MaybeUninit};

bitflags! {
    struct GateFlags: u8 {
        const INTERRUPT_GATE = 0b1110;
        const RING_0 = 0 << 5;
        const RING_3 = 3 << 5;
        const PRESENT = 1 << 7;
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct IdtDescriptor {
    offset_15_0: u16,
    segment_selector: u16,
    // Bits 0..2: IST, rest is 0
    ist: u8,
    attributes: GateFlags,
    offset_31_16: u16,
    offset_63_32: u32,
    _ignored: MaybeUninit<u32>,
}

impl IdtDescriptor {
    pub fn new(handler: usize, ist: u8) -> Self {
        let addr = handler;
        Self {
            offset_15_0: addr as u16,
            offset_31_16: (addr >> 16) as u16,
            offset_63_32: (addr >> 32) as u32,
            segment_selector: KERNEL_CS,
            ist,
            attributes: GateFlags::INTERRUPT_GATE | GateFlags::PRESENT | GateFlags::RING_0,
            _ignored: MaybeUninit::uninit(),
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct ExceptionStackFrame {
    pub ip: u64,
    pub cs: u64,
    pub flags: u64,
    pub sp: u64,
    pub ss: u64,
}

extern "x86-interrupt" fn handler() {
    log!("got handled!");
}
extern "x86-interrupt" fn seg_handler() {
    log!("got seg");
}
extern "x86-interrupt" fn stack_handler() {
    log!("got seg");
}
extern "x86-interrupt" fn gpf_handler(frame: ExceptionStackFrame, error_code: u64) {
    log!("got gpf (whatevr just debug here)");
    log!("frame: {:#x?}, error_code: {:#x?}", frame, error_code);
    panic!();
}
extern "x86-interrupt" fn double_fault_handler(frame: ExceptionStackFrame, error_code: u64) {
    error!("Exception frame: {:#x?}. Error code: {}", frame, error_code);
    panic!("Double fault");
}

const NB_ENTRIES: usize = 256;
type IdtType = [MaybeUninit<IdtDescriptor>; NB_ENTRIES];
static mut IDT: IdtType = unsafe { MaybeUninit::uninit().assume_init() };

pub fn load() {
    unsafe {
        IDT[11] = MaybeUninit::new(IdtDescriptor::new(seg_handler as usize, 0));
        IDT[12] = MaybeUninit::new(IdtDescriptor::new(stack_handler as usize, 0));
        IDT[13] = MaybeUninit::new(IdtDescriptor::new(gpf_handler as usize, 0));
        IDT[33] = MaybeUninit::new(IdtDescriptor::new(handler as usize, 0));

        //test double f
        IDT[8] = MaybeUninit::new(IdtDescriptor::new(double_fault_handler as usize, 1));
        trace!("dbl fault entry {:#x?}", IDT[8].assume_init());

        let register_format = DescriptorTableRegister {
            limit: (size_of::<IdtType>() - 1) as u16,
            base: IDT.as_ptr() as *const u64,
        };
        asm!("lidt [{}]", in(reg) &register_format, options(readonly, nostack, preserves_flags));
    }
}
