#![no_std]
#![no_main]

mod stivale;
mod vga;

use core::fmt::Write;
use core::{fmt, panic::PanicInfo, ptr};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[no_mangle]
extern "C" fn kernel_main(/*boot_info: &'static StivaleStruct*/) -> ! {
    println!("Hello World{}", "!");
    panic!("oops :)");
    loop {}
}
