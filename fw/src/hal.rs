use embedded_hal::timer::{CountDown, Periodic};
use void::Void;

#[derive(Clone, Copy)]
pub struct BusyWaitTimer {
    cycles_to_wait: u32,
}

impl BusyWaitTimer {
    pub fn new(cycles: u32) -> Self {
        Self {
            cycles_to_wait: cycles,
        }
    }
}

impl CountDown for BusyWaitTimer {
    type Time = u32;

    fn start<T>(&mut self, count: T)
        where
            T: Into<Self::Time>,
    {
        self.cycles_to_wait = count.into();
    }

    fn wait(&mut self) -> nb::Result<(), Void> {
        for _ in 0..self.cycles_to_wait {
            unsafe {
                asm!("nop");
            }
        }
        Ok(())
    }
}

impl Periodic for BusyWaitTimer {}
