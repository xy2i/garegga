//! Stivale2 boot protocol implementation
//! https://github.com/stivale/stivale/blob/master/STIVALE2.md
//! https://github.com/stivale/stivale2-barebones

use crate::kernel_main;
use core::ptr;

#[repr(C, align(0x1000))]
struct Align<T>(T);
const STACK_SIZE: usize = 0x1000 * 8;
static STACK: Align<[u8; STACK_SIZE]> = Align([0; STACK_SIZE]);

pub struct Tag {
    pub identifier: u64,
    pub next: *const Tag,
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

// #[repr(C)]
// struct StivaleStruct {
//     bootloader_brand: [u8; 64],
//     bootloader_version: [u8; 64],
//     tags: *const StructTag,
// }
// impl StivaleStruct {
//     pub fn get_tag(&self, identifier: u64) -> Option<u64> {
//         let mut current_tag = self.tags;
//
//         while !current_tag.is_null() {
//             let tag = unsafe { &*current_tag };
//
//             if tag.identifier == identifier {
//                 return Some(current_tag as u64);
//             }
//
//             current_tag = tag.next;
//         }
//
//         None
//     }
// }
//
// #[repr(C)]
// pub struct StructTag {
//     pub identifier: u64,
//     pub next: *const StructTag,
// }

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
