//! Manage x86 interrupts.
//!
//! CR0.PE is enabled by stivale2.

use core::arch::asm;

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

bitflags::bitflags! {
    /// An index to a segment descriptor. The RPL indicates which privilege
    /// to use with this segment, but it cannot be higher than the RPL
    /// specified in the segment descriptor.
    #[repr(transparent)]
    struct SegmentSelector: u16 {
        const RPL_0 = 0;
        const RPL_3 = 3;
        const TI_GDT = 0 << 2;
        const TI_LDT = 1 << 2;
    }
}

impl SegmentSelector {
    const fn new(index: u16, rpl: Ring) -> Self {
        Self {
            bits: index << 3 | (rpl as u16),
        }
    }
}

// Segment selectors (4.5 AMD64 manual)
const KERNEL_CS: SegmentSelector = SegmentSelector::new(1, Ring::Ring0);
const KERNEL_DS: SegmentSelector = SegmentSelector::new(2, Ring::Ring0);

/// Global Descriptor Table.
/// In AMD64, the GDT is mostly a legacy structure.
/// Its main use is for privilege level switching and the TSS.  
/// Because it's a set-once structure, this module is very infexible.
mod gdt {
    use crate::interrupts::{DescriptorTableRegister, KERNEL_CS, KERNEL_DS};
    use bitflags::bitflags;
    use core::arch::asm;
    use core::mem::size_of;

    bitflags! {
        struct SegmentUpperFlags: u8 {
            /// Disables segmentation
            const LONG_MODE = 1 << 5;
        }
    }

    bitflags! {
        /// Many flags are ignored in 64-bit mode.
        /// See 4.7 Legacy Segment Descriptors, AMD64
        struct SegmentLowerFlags: u8 {
            const PRESENT = 1 << 7;
            /// Descriptor Privilege Level 0 = ring0, kernel
            const DPL_0 = 0 << 5;
            /// Descriptor Privilege Level 3 = ring3, user
            const DPL_3 = 3 << 5;
            /// "User descriptor": can be either a Code or Segment descriptor
            /// (AMD64 manual, Table 4.2. Descriptor Types)
            const USER_DESCRIPTOR = 1 << 4;
            /// Segment is a data segment. If not set, the segment is a data segment
            const EXECUTE = 1 << 3;
            /// For a code segment, means the segment is readable.
            /// For a data segment, means the segment is writable.
            const ACCESSIBLE = 1 << 1;
            const KERNEL_CODE = Self::PRESENT.bits | Self::DPL_0.bits | Self::USER_DESCRIPTOR.bits
             | Self::EXECUTE.bits | Self::ACCESSIBLE.bits;
            const KERNEL_DATA = Self::PRESENT.bits | Self::DPL_0.bits | Self::USER_DESCRIPTOR.bits
             | Self::ACCESSIBLE.bits;
        }
    }

    #[derive(Debug)]
    #[repr(C)]
    struct SegmentDescriptor {
        limit_15_0: u16,
        base_15_0: u16,
        base_23_16: u8,
        lower_flags: SegmentLowerFlags,
        /// Bytes 19:16 is the segment limit,
        /// bytes 23:20 are the flags.  
        limit_and_upper_flags: u8,
        base_31_24: u8,
    }

    impl SegmentDescriptor {
        const NULL: Self =
            SegmentDescriptor::new(SegmentUpperFlags::empty(), SegmentLowerFlags::empty());

        const fn new(upper_flags: SegmentUpperFlags, lower_flags: SegmentLowerFlags) -> Self {
            Self {
                limit_15_0: 0,
                base_15_0: 0,
                base_23_16: 0,
                limit_and_upper_flags: upper_flags.bits,
                lower_flags,
                base_31_24: 0,
            }
        }
    }

    const NB_ENTRIES: usize = 3;

    type GdtType = [SegmentDescriptor; NB_ENTRIES];

    static GDT: GdtType = [
        SegmentDescriptor::NULL,
        SegmentDescriptor::new(SegmentUpperFlags::LONG_MODE, SegmentLowerFlags::KERNEL_CODE),
        SegmentDescriptor::new(SegmentUpperFlags::LONG_MODE, SegmentLowerFlags::KERNEL_DATA),
    ];

    pub fn load() {
        let register_format = DescriptorTableRegister {
            limit: (size_of::<GdtType>() - 1) as u16,
            base: GDT.as_ptr() as *const u64,
        };

        unsafe {
            // Flush the GDT, update CS and other segments registers.
            //
            // Note that we can't update CS directly, since it would be a JMP:
            // instructions are fetched from CS:IP.
            // https://stackoverflow.com/questions/52490438/why-cant-mov-set-cs-the-code-segment-register-even-though-it-can-set-others
            // There are no long jumps in long mode, so we'll use the far return instruction,
            // which pops IP and CS off the stack.
            asm!(
                r#"
lgdt [{gdt}]"
lea {ip}, [rip + 0f]
push {cs}    
push {ip}
retfq
0:
mov ax, {ds}
mov ds, ax
mov es, ax
mov fs, ax
mov gs, ax
mov ss, ax
            "#,
                gdt = in(reg) &register_format,
                cs = const KERNEL_CS.bits,
                ip = lateout(reg) _,
                ds = const KERNEL_DS.bits,
                options(readonly, nostack, preserves_flags)
            )
        }
    }
}

/// Interrupt Descriptor Table
mod idt {
    use crate::interrupts::{DescriptorTableRegister, KERNEL_CS};
    use bitflags::bitflags;
    use core::arch::asm;
    use core::mem::{size_of, MaybeUninit};

    use super::Ring;

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
        /// P, DPL, 0, Type
        attributes: GateFlags,
        offset_31_16: u16,
        offset_63_32: u32,
        _ignored: MaybeUninit<u32>,
    }

    impl IdtDescriptor {
        pub fn new(handler: extern "x86-interrupt" fn()) -> Self {
            let addr = handler as u64;
            Self {
                offset_15_0: addr as u16,
                offset_31_16: (addr >> 16) as u16,
                offset_63_32: (addr >> 32) as u32,
                segment_selector: KERNEL_CS.bits,
                ist: 0,
                attributes: GateFlags::INTERRUPT_GATE | GateFlags::PRESENT | GateFlags::RING_0,
                _ignored: MaybeUninit::uninit(),
            }
        }
    }

    extern "x86-interrupt" fn handler() {
        log!("got handled!");
    }

    const NB_ENTRIES: usize = 256;
    type IdtType = [MaybeUninit<IdtDescriptor>; NB_ENTRIES];
    static mut IDT: IdtType = unsafe { MaybeUninit::uninit().assume_init() };

    pub fn load() {
        unsafe {
            for i in 0..32 {
                IDT[i] = MaybeUninit::new(IdtDescriptor::new(handler));
            }

            IDT[33] = MaybeUninit::new(IdtDescriptor::new(handler));
            trace!("entry 33 {:#?}", IDT[33].assume_init());

            let register_format = DescriptorTableRegister {
                limit: (size_of::<IdtType>() - 1) as u16,
                base: IDT.as_ptr() as *const u64,
            };
            asm!("lidt [{}]", in(reg) &register_format, options(readonly, nostack, preserves_flags));
        }
    }
}

pub fn init() {
    debug!("Initializing GDT");
    gdt::load();
    debug!("Initializing IDT");
    idt::load();
    debug!("Firing int 33");
    unsafe {
        asm!("int 33");
    }
    debug!("YOU DID IT!!! :D");
}
