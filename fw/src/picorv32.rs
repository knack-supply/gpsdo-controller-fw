use core::convert::Infallible;
use core::fmt;
use embedded_hal::digital::v2::OutputPin;
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
    pub out: RW<u32>,
}

impl GPIO {
    fn ptr() -> *const Self {
        0x03000000 as *const _
    }
}

pub struct GPIO0 {}

impl OutputPin for GPIO0 {
    type Error = ();

    fn set_low(&mut self) -> Result<(), Self::Error> {
        unsafe { (*GPIO::ptr()).out.modify(|r| r & !1) };
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        unsafe { (*GPIO::ptr()).out.modify(|r| r | 1) };
        Ok(())
    }
}

pub struct GPIO1 {}

impl OutputPin for GPIO1 {
    type Error = ();

    fn set_low(&mut self) -> Result<(), Self::Error> {
        unsafe { (*GPIO::ptr()).out.modify(|r| r & !2) };
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        unsafe { (*GPIO::ptr()).out.modify(|r| r | 2) };
        Ok(())
    }
}

pub struct GPIO2 {}

impl OutputPin for GPIO2 {
    type Error = ();

    fn set_low(&mut self) -> Result<(), Self::Error> {
        unsafe { (*GPIO::ptr()).out.modify(|r| r & !4) };
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        unsafe { (*GPIO::ptr()).out.modify(|r| r | 4) };
        Ok(())
    }
}
