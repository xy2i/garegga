#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![feature(asm_const)]
#![allow(unused_macros)]
#![allow(dead_code)]
#![test_runner(crate::test::test_runner)]
#![reexport_test_harness_main = "test_main"]

#[cfg(not(test))]
use core::panic::PanicInfo;

use crate::boot::StivaleStruct;
use crate::x86::hlt;

mod boot;
mod io;
#[macro_use]
mod logger;
mod interrupts;
mod pmm;
mod serial;
mod test;
mod vga;
mod x86;

// see test.rs for the panic test handler
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("PANIC: {}", info);
    println!("PANIC: {}", info);
    loop {
        hlt();
    }
}

#[no_mangle]
extern "C" fn kernel_main(boot_info: &'static StivaleStruct) -> ! {
    interrupts::init();

    log!("Got {} memory map entries", boot_info.memory_map().len());
    for entry in boot_info.memory_map() {
        log!("{entry:#x?}");
    }

    #[cfg(test)]
    test_main();

    loop {
        hlt();
    }
}
