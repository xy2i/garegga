//! Stivale2 boot protocol implementation
//! https://github.com/stivale/stivale/blob/master/STIVALE2.md
//! https://github.com/stivale/stivale2-barebones

use core::mem::MaybeUninit;
use core::{mem, ptr};

use crate::kernel_main;

#[repr(C, align(0x1000))]
struct Align<T>(T);

const STACK_SIZE: usize = 0x1000 * 8;
static STACK: Align<[u8; STACK_SIZE]> = Align([0; STACK_SIZE]);

#[derive(Debug)]
pub struct Tag {
    pub identifier: u64,
    pub next: *const Tag,
}

impl Tag {
    pub const MEMORY_MAP: u64 = 0x2187f79e8612de07;
}

unsafe impl Send for Tag {}

unsafe impl Sync for Tag {}

struct Header {
    _entry_point: *const (),
    _stack: *const u8,
    _flags: u64,
    _tags: *const Tag,
}

unsafe impl Send for Header {}

unsafe impl Sync for Header {}

#[repr(C)]
pub struct StivaleStruct {
    bootloader_brand: [u8; 64],
    bootloader_version: [u8; 64],
    tags: *const Tag,
}

impl StivaleStruct {
    pub fn get_tag(&self, identifier: u64) -> Option<u64> {
        let mut current_tag = self.tags;

        while !current_tag.is_null() {
            let tag = unsafe { &*current_tag };

            if tag.identifier == identifier {
                return Some(current_tag as u64);
            }

            current_tag = tag.next;
        }

        None
    }

    pub fn memmap(&self) -> &'static mut MemmapStructTag {
        unsafe {
            let ptr = self.get_tag(Tag::MEMORY_MAP).unwrap() as *mut usize;
            let count = *(ptr.add(mem::size_of::<Tag>()) as *const u64);
            MemmapStructTag::new(ptr, count)
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct MemmapStructTag {
    tag: Tag,
    pub entries: u64,
    pub values: [MemmapEntry],
}

impl MemmapStructTag {
    fn new(ptr: *mut usize, entry_count: u64) -> &'static mut Self {
        unsafe {
            let slice_ptr = ptr::slice_from_raw_parts_mut(ptr, entry_count as usize);
            &mut *(slice_ptr as *mut Self)
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct MemmapEntry {
    /// Physical address of base of the memory section
    base: u64,
    /// Length of the section
    length: u64,
    mm_type: MemmapType,
    unused: MaybeUninit<u32>,
}

#[derive(Debug)]
#[repr(u32)]
enum MemmapType {
    Usable = 1,
    Reserved = 2,
    AcpiReclaimable = 3,
    AcpiNvs = 4,
    BadMemory = 5,
    BootloaderReclaimable = 0x1000,
    KernelAndModules = 0x1001,
    Framebuffer = 0x1002,
}

struct TerminalHeaderTag {
    _tag: Tag,
    _flags: u64,
}

static STIVALE_TERM: TerminalHeaderTag = TerminalHeaderTag {
    _tag: Tag {
        identifier: 0xa85d499b1823be72,
        next: ptr::null(),
    },
    _flags: 0,
};

#[link_section = ".stivale2hdr"]
#[no_mangle]
#[used]
static STIVALE_HDR: Header = Header {
    _entry_point: kernel_main as *const (),
    _stack: STACK.0.as_ptr_range().end,
    // Bit 1, if set, causes the bootloader to return to us pointers in the
    // higher half, which we likely want since this is a higher half kernel.
    // Bit 2, if set, tells the bootloader to enable protected memory ranges,
    // that is, to respect the ELF PHDR mandated permissions for the executable's
    // segments.
    // Bit 3, if set, enables fully virtual kernel mappings, which we want as
    // they allow the bootloader to pick whichever *physical* memory address is
    // available to load the kernel, rather than relying on us telling it where
    // to load it.
    // Bit 4 disables a deprecated feature and should always be set.
    // TODO: bit 2 doesn't work
    _flags: (1 << 1) | (1 << 3) | (1 << 4),
    _tags: &STIVALE_TERM as *const TerminalHeaderTag as *const Tag,
};
