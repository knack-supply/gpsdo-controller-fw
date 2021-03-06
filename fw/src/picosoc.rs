use core::convert::Infallible;
use core::fmt;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use ufmt::uWrite;
use volatile_register::RW;

#[repr(C)]
pub struct UART {
    pub clkdiv: RW<u32>,
    pub data: RW<u8>,
}

impl UART {
    fn ptr() -> *const Self {
        0x02000004 as *const _
    }

    pub fn new() -> &'static UART {
        unsafe { Self::ptr().as_ref().unwrap() }
    }

    pub fn set_speed(&self, baud: u32) {
        unsafe {
            self.clkdiv.write(u32::from(12_000_000 / baud).max(1));
        }
    }
}

#[derive(Clone, Copy)]
pub struct ConsoleDevice {}

impl uWrite for ConsoleDevice {
    type Error = Infallible;

    fn write_str(&mut self, s: &str) -> Result<(), Infallible> {
        s.as_bytes().iter().for_each(|b| unsafe {
            (*UART::ptr()).data.write(*b);
        });

        Ok(())
    }
}

impl fmt::Write for ConsoleDevice {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        s.as_bytes().iter().for_each(|b| unsafe {
            (*UART::ptr()).data.write(*b);
        });

        Ok(())
    }
}

#[repr(C)]
pub struct GPIO {
    pub io: RW<u32>,
}

impl GPIO {
    fn ptr() -> *const Self {
        0x03000000 as *const _
    }
}

macro_rules! output_pin {
    ($name:ident, $ix:literal) => {
        #[derive(Copy, Clone)]
        pub struct $name {}

        impl OutputPin for $name {
            type Error = ();

            fn set_low(&mut self) -> Result<(), Self::Error> {
                unsafe { (*GPIO::ptr()).io.modify(|r| r & !(1 << $ix)) };
                Ok(())
            }

            fn set_high(&mut self) -> Result<(), Self::Error> {
                unsafe { (*GPIO::ptr()).io.modify(|r| r | (1 << $ix)) };
                Ok(())
            }
        }
    }
}

macro_rules! input_pin {
    ($name:ident, $ix:literal) => {
        #[derive(Copy, Clone)]
        pub struct $name {}

        impl InputPin for $name {
            type Error = ();

            fn is_low(&self) -> Result<bool, Self::Error> {
                Ok(unsafe { &(*GPIO::ptr()).io } .read() & (1 << $ix) == 0)
            }

            fn is_high(&self) -> Result<bool, Self::Error> {
                Ok(unsafe { &(*GPIO::ptr()).io } .read() & (1 << $ix) != 0)
            }
        }
    }
}

output_pin!(GPIO0, 0);
output_pin!(GPIO1, 1);
output_pin!(GPIO2, 2);
output_pin!(GPIO3, 3);
input_pin!(GPIO4, 4);
output_pin!(GPIO5, 5);
