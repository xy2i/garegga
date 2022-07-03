//! Global Descriptor Table.
//! In AMD64, the GDT is mostly a legacy structure.
//! Its main use is for privilege level switching and the TSS.  
//! Because it's a set-once structure, this module is very infexible.

use crate::interrupts::{DescriptorTableRegister, KERNEL_CS, KERNEL_DS, TSS_SELECTOR};
use bitflags::bitflags;
use core::arch::asm;
use core::mem::{size_of, MaybeUninit};

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
        const SYSTEM_DESCRIPTOR = 0 << 4;
        /// Segment is a data segment. If not set, the segment is a data segment
        const EXECUTE = 1 << 3;
        /// For a code segment, means the segment is readable.
        /// For a data segment, means the segment is writable.
        const ACCESSIBLE = 1 << 1;

        const KERNEL_CODE = Self::PRESENT.bits | Self::DPL_0.bits | Self::USER_DESCRIPTOR.bits
         | Self::EXECUTE.bits | Self::ACCESSIBLE.bits;
        const KERNEL_DATA = Self::PRESENT.bits | Self::DPL_0.bits | Self::USER_DESCRIPTOR.bits
         | Self::ACCESSIBLE.bits;
        /// See Table 4.6,  System-Segment Descriptor Types—Long Mode AMD64
        const TSS_SEGMENT = Self::PRESENT.bits | 0b1001 | Self::SYSTEM_DESCRIPTOR.bits ;
    }
}

#[derive(Debug)]
#[repr(C)]
struct UserSegmentDescriptor {
    limit_15_0: u16,
    base_15_0: u16,
    base_23_16: u8,
    lower_flags: SegmentLowerFlags,
    /// Bytes 19:16 is the segment limit,
    /// bytes 23:20 are the flags.  
    limit_and_upper_flags: u8,
    base_31_24: u8,
}

impl UserSegmentDescriptor {
    const NULL: Self =
        UserSegmentDescriptor::new(SegmentUpperFlags::empty(), SegmentLowerFlags::empty());

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

#[derive(Debug)]
#[repr(C)]
/// This is only used for the TSS.
struct SystemSegmentDescriptor {
    limit_15_0: u16,
    base_15_0: u16,
    base_23_16: u8,
    lower_flags: SegmentLowerFlags,
    /// Bytes 19:16 is the segment limit,
    /// bytes 23:20 are the flags.  
    limit_and_upper_flags: u8,
    base_31_24: u8,
    base_63_32: u32,
    zero: u32,
}

impl SystemSegmentDescriptor {
    const fn tss() -> Self {
        Self {
            limit_15_0: 0,
            base_15_0: 0,
            base_23_16: 0,
            lower_flags: SegmentLowerFlags::TSS_SEGMENT,
            limit_and_upper_flags: SegmentUpperFlags::empty().bits,
            base_31_24: 0,
            base_63_32: 0,
            zero: 0,
        }
    }
    fn set_base(&mut self, base: &Tss) -> &mut Self {
        let base = base as *const Tss as usize;
        self.base_15_0 = base as u16;
        self.base_23_16 = (base >> 16) as u8;
        self.base_31_24 = (base >> 24) as u8;
        self.base_63_32 = (base >> 32) as u32;
        self
    }

    fn set_limit(&mut self, limit: u16) -> &mut Self {
        self.limit_15_0 = limit;
        self
    }
}

/// Figure 12-8. Long Mode TSS Format AMD64
#[repr(C, packed(4))]
struct Tss {
    _ignored1: MaybeUninit<u32>,
    pub rsp_list: [usize; 3],
    _ignored2: MaybeUninit<u64>,
    pub ist_list: [usize; 7],
    _ignored3: MaybeUninit<u64>,
    _ignored4: MaybeUninit<u16>,
    pub io_map_base: u16,
}
impl Tss {
    pub const fn new() -> Self {
        Self {
            _ignored1: MaybeUninit::uninit(),
            rsp_list: [0; 3],
            _ignored2: MaybeUninit::uninit(),
            ist_list: [0; 7],
            _ignored3: MaybeUninit::uninit(),
            _ignored4: MaybeUninit::uninit(),
            io_map_base: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug)]
struct Gdt {
    user_segments: [UserSegmentDescriptor; 3],
    tss: SystemSegmentDescriptor,
}

static mut GDT: Gdt = Gdt {
    user_segments: [
        UserSegmentDescriptor::NULL,
        UserSegmentDescriptor::new(SegmentUpperFlags::LONG_MODE, SegmentLowerFlags::KERNEL_CODE),
        UserSegmentDescriptor::new(SegmentUpperFlags::LONG_MODE, SegmentLowerFlags::KERNEL_DATA),
    ],
    tss: SystemSegmentDescriptor::tss(),
};

#[repr(C, align(8))]
struct Align8<T>(T);

const DOUBLE_FAULT_IST: usize = 0;

static mut TSS: Tss = Tss::new();

pub fn load() {
    let double_fault_stack = Align8::<[u8; 4096]>([0; 4096]);

    unsafe {
        TSS.ist_list[DOUBLE_FAULT_IST] =
            double_fault_stack.0.as_ptr_range().end as *const u8 as usize;

        // Set the TSS entry in the GDT.
        let tss_ref = &mut GDT.tss;
        tss_ref.set_base(&TSS).set_limit(size_of::<Tss>() as u16);

        // Flush the GDT, update CS and other segments registers.
        //
        // Note that we can't update CS directly, since it would be a JMP:
        // instructions are fetched from CS:IP.
        // https://stackoverflow.com/questions/52490438/why-cant-mov-set-cs-the-code-segment-register-even-though-it-can-set-others
        // There are no long jumps in long mode, so we'll use the far return instruction,
        // which pops IP and CS off the stack.
        let register_format = DescriptorTableRegister {
            limit: (size_of::<Gdt>() - 1) as u16,
            base: &GDT as *const Gdt as *const u64,
        };

        asm!(
            r#"
lgdt [{gdt}]
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
            cs = const KERNEL_CS,
            ip = lateout(reg) _,
            ds = const KERNEL_DS,
            options(readonly, nostack, preserves_flags)
        );

        // Load the TSS.
        asm!("ltr {:x}", in(reg) TSS_SELECTOR, options(readonly, nostack, preserves_flags));
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test_case]
    fn tss_size_is_correct() {
        assert_eq!(size_of::<Tss>(), 0x68);
    }

    #[test_case]
    fn code_and_data_selectors_match() {
        unsafe {
            let cs_index = ((KERNEL_CS >> 3) & 0b11) as usize;
            let gdt_cs_entry = &GDT.user_segments[cs_index];
            assert!(gdt_cs_entry
                .lower_flags
                .contains(SegmentLowerFlags::KERNEL_CODE));

            let ds_index = ((KERNEL_DS >> 3) & 0b11) as usize;
            let gdt_ds_entry = &GDT.user_segments[ds_index];
            assert!(gdt_ds_entry
                .lower_flags
                .contains(SegmentLowerFlags::KERNEL_DATA));
        }
    }

    #[test_case]
    fn tss_selector_matches() {
        impl From<&SystemSegmentDescriptor> for usize {
            fn from(a: &SystemSegmentDescriptor) -> Self {
                (a.base_63_32 as usize) << 32
                    | (a.base_31_24 as usize) << 24
                    | (a.base_23_16 as usize) << 16
                    | (a.base_15_0 as usize)
            }
        }
        unsafe {
            let gdt_tss_entry = &GDT.tss;
            let tss_addr: usize = gdt_tss_entry.into();
            assert!(gdt_tss_entry
                .lower_flags
                .contains(SegmentLowerFlags::TSS_SEGMENT));

            assert_eq!(tss_addr, &TSS as *const Tss as usize);
        }
    }
}
