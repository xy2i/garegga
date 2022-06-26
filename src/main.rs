#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![feature(asm_const)]
#![test_runner(crate::test::test_runner)]
#![reexport_test_harness_main = "test_main"]

mod boot;
mod io;
#[macro_use]
mod logger;
mod interrupts;
mod serial;
mod test;
mod vga;
mod x86;

use crate::x86::hlt;
#[cfg(not(test))]
use core::panic::PanicInfo;

// see test.rs for the panic test handler
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {
        hlt();
    }
}

#[no_mangle]
extern "C" fn kernel_main(/*boot_info: &'static StivaleStruct*/) -> ! {
    interrupts::init();

    #[cfg(test)]
    test_main();

    loop {
        hlt();
    }
}
