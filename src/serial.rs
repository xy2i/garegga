//! Serial port implementation (uart_16650).

use crate::io::{inb, outb};
use core::fmt::Write;
use lazy_static::lazy_static;
use spin::Mutex;

#[repr(transparent)]
pub struct Serial();

impl Serial {
    // https://en.wikibooks.org/wiki/Serial_Programming/8250_UART_Programming#Software_interrupts

    /// Base port (also Transmitter Holding Buffer/Register)
    const PORT: u16 = 0x3f8;
    /// Interrupt Enable Register
    const IER: u16 = Serial::PORT + 1;
    /// Fifo Control Register
    const FCR: u16 = Serial::PORT + 2;
    /// Line Control Register
    const LCR: u16 = Serial::PORT + 3;
    /// Modem Control Register
    const MCR: u16 = Serial::PORT + 4;
    /// Line Status Register
    const LSR: u16 = Serial::PORT + 5;

    /// Create and initialize a serial port.
    pub fn new() -> Self {
        unsafe {
            // https://www.latticesemi.com/-/media/LatticeSemi/Documents/ReferenceDesigns/SZ/UART16550Transceiver-Documentation.ashx?document_id=48168
            // p.10, Transmit Operation

            // Enable Divisor Latch Access Bit, to set baud rate.
            outb(Serial::LCR, 1 << 7);
            // Set baud rate: 115200
            outb(Serial::PORT, 0x1); // lo
            outb(Serial::PORT + 1, 0); // hi
                                       // Disable interrupts
            outb(Serial::IER, 0);

            // Unset DLAB, 8 bit word length, no parity, one stop bit
            outb(Serial::LCR, 0x3);
            // Enable fifo, clear tx/rx, 14 bytes watermark
            outb(Serial::FCR, 0b1100_0111);

            // Enable aux. output 2 for interrupts,
            // set Request To Send, Data Terminal Ready
            outb(Serial::MCR, 0b1011);

            Serial {}
        }
    }

    /// Transmit a single byte
    pub fn tx(&self, byte: u8) {
        unsafe {
            while inb(Serial::LSR) & (1 << 5) == 0 {
                core::hint::spin_loop();
            }

            outb(Serial::PORT, byte);
        }
    }
}

impl Write for Serial {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.bytes() {
            self.tx(byte);
        }
        Ok(())
    }
}

lazy_static! {
    pub static ref COM1: Mutex<Serial> = Mutex::new(Serial::new());
}

#[doc(hidden)]
pub fn _print(args: core::fmt::Arguments) {
    COM1.lock()
        .write_fmt(args)
        .expect("Printing to serial failed");
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(
        concat!($fmt, "\n"), $($arg)*));
}
