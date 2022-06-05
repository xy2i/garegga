#![no_std]
#![no_main]
#![warn(missing_docs)]
// required by volatile
#![feature(core_intrinsics)]
#![feature(slice_range)]
#![allow(incomplete_features)]
#![feature(slice_as_chunks)]

mod vga;
mod volatile;

use core::fmt::Write;
use core::{panic::PanicInfo, ptr};

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

struct TerminalHeaderTag {
    tag: Tag,
    flags: u64,
}

struct FramebufferHeaderTag {
    tag: Tag,
    framebuffer_width: u16,
    framebuffer_height: u16,
    framebuffer_bpp: u16,
}

struct Header {
    entry_point: *const (),
    stack: *const u8,
    flags: u64,
    tags: *const Tag,
}
unsafe impl Send for Header {}
unsafe impl Sync for Header {}

#[repr(C)]
struct StivaleStruct {
    bootloader_brand: [u8; 64],
    bootloader_version: [u8; 64],
    tags: *const StructTag,
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
}

#[repr(C)]
pub struct StivaleTerminalTag {
    pub header: Tag,
    pub flags: u32,
    pub cols: u16,
    pub rows: u16,
    pub term_write_fn: extern "C" fn(*const i8, u64),
}

impl Write for StivaleTerminalTag {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        (self.term_write_fn)(s.as_ptr() as *const i8, s.len() as u64);
        Ok(())
    }
}

#[repr(C)]
pub struct StructTag {
    pub identifier: u64,
    pub next: *const StructTag,
}

static STIVALE_TERM: TerminalHeaderTag = TerminalHeaderTag {
    tag: Tag {
        identifier: 0xa85d499b1823be72,
        next: ptr::null(),
    },
    flags: 0,
};

static STIVALE_FB: FramebufferHeaderTag = FramebufferHeaderTag {
    tag: Tag {
        identifier: 0x3ecc1bc43d0f7971,
        next: (&STIVALE_TERM as *const TerminalHeaderTag as *const Tag),
    },
    // let bootloader choose
    framebuffer_height: 240,
    framebuffer_width: 240,
    framebuffer_bpp: 0,
};

#[link_section = ".stivale2hdr"]
#[no_mangle]
#[used]
static STIVALE_HDR: Header = Header {
    entry_point: x86_64_barebones_main as *const (),
    stack: STACK.0.as_ptr_range().end,
    flags: 0,
    tags: &STIVALE_TERM as *const TerminalHeaderTag as *const Tag,
};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
extern "C" fn x86_64_barebones_main(boot_info: &'static StivaleStruct) -> ! {
    use vga::print;
    print();
    loop {}
}
