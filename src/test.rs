//! Our custom test runner.
//! The test_runner function is defined as Cargo's test runner (see main.rs).
// cfg(test) for whole module.
#![cfg(test)]

use crate::{print, println, serial_print, serial_println};
use core::arch::asm;
use core::panic::PanicInfo;

/// Test exit codes returned by QEMU.
/// Note that when exiting, QEMU will shift the exit code:
/// (exit << 1) | 1
/// so the value returned to the shell will NOT be these constants!
#[repr(u32)]
pub enum TestResult {
    // Returned value: 33 (0x10 << 1) | 1
    Success = 0x10,
    // Returned value: 35 (0x11 << 1) | 1
    Failure = 0x11,
}

pub fn exit_qemu(exit_code: TestResult) {
    // Write to our configured debug exit isa device:
    // the -device isa-debug-exit,iobase=0xf4,iosize=0x04 argument to qemu,
    // which will exit QEMU.
    unsafe {
        asm!("out dx, eax", in("dx") 0xf4, in("eax") exit_code as u32, options(nomem, nostack, preserves_flags));
    }
}

pub trait Test {
    fn run(&self);
}
impl<T> Test for T
where
    T: Fn(),
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

pub fn test_runner(tests: &[&dyn Test]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(TestResult::Success)
}

// our panic handler in test mode
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    exit_qemu(TestResult::Failure);
    loop {}
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}
