#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![feature(abi_x86_interrupt)]
#![feature(asm_const)]
#![test_runner(crate::test::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![allow(unused_macros)]
#![allow(dead_code)]

#[cfg(not(test))]
use core::panic::PanicInfo;

use crate::boot::StivaleStruct;
use crate::x86::hlt;

mod boot;
mod io;
#[macro_use]
mod logger;
mod interrupts;
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

    let memmap = boot_info.memmap();

    log!("{}", memmap.entries);
    for i in 0..memmap.entries {
        let entry = &memmap.values[i as usize];
        log!("{entry:#x?}");
    }

    #[cfg(test)]
    test_main();

    loop {
        hlt();
    }
}
