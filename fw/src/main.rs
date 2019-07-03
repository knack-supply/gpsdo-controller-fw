#![no_std]
#![no_main]
#![feature(proc_macro_hygiene)]
#![feature(asm)]
#![feature(clamp)]
#![feature(try_blocks)]
#![allow(deprecated)]

#[cfg(not(test))]
extern crate panic_halt;
#[macro_use]
extern crate ufmt;

use core::fmt::Write;
use embedded_hal::digital::v1_compat::OldOutputPin;
use embedded_hal::digital::v2::OutputPin;
use embedded_hal::digital::InputPin;
use embedded_hal::spi::{FullDuplex, MODE_0};
use embedded_hal::timer::{CountDown, Periodic};
use ks_gpsdo::freq_counter::{
    FrequencyCounter, FrequencyCounters, FrequencyCountersToleranceCheck,
};
use ks_gpsdo::max5216::MAX5216;
use ks_gpsdo::picorv32::*;
use riscv_minimal_rt::entry;
use typenum::consts::*;
use ufmt::uWrite;
use void::Void;
use ks_gpsdo::filter::{UniformAverageFilter, ExponentialAverageFilter};

struct GPIODummyIn {}

impl InputPin for GPIODummyIn {
    fn is_high(&self) -> bool {
        false
    }

    fn is_low(&self) -> bool {
        true
    }
}

#[derive(Clone, Copy)]
struct BusyWaitTimer {
    cycles_to_wait: u32,
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

struct ControlLoop<SPI: FullDuplex<u8>, CS: OutputPin, CONSOLE: uWrite + Write> {
    console: CONSOLE,
    tolerance_check: FrequencyCountersToleranceCheck,
    dac: MAX5216<SPI, CS>,
    last_epoch: Option<u8>,
    error_flag: bool,
    output_flag: bool,
}

impl<SPI: FullDuplex<u8>, CS: OutputPin, CONSOLE: uWrite + Write> ControlLoop<SPI, CS, CONSOLE> {
    pub fn new(spi: SPI, cs: CS, console: CONSOLE) -> Self {
        Self {
            console,
            tolerance_check: FrequencyCountersToleranceCheck {
                target_sig_cnt: 10_000_000,
                sig_cnt_tolerance: 200,
                #[cfg(feature = "hx8k")]
                target_clk: 201_000_000,
                #[cfg(feature = "up5k")]
                target_clk: 100_500_000,
                clk_tolerance: 10_000,
            },
            dac: MAX5216::new(spi, cs),
            last_epoch: None,
            error_flag: false,
            output_flag: false,
        }
    }

    pub fn get_counters(&mut self) -> Result<FrequencyCounters, ()> {
        'try_again: loop {
            let counters = FrequencyCounter::get_counters()?;
            if let Some(last_epoch) = self.last_epoch {
                if last_epoch == counters.epoch {
                    continue 'try_again;
                }
                if (last_epoch + 1) & 0b11 != counters.epoch {
                    writeln!(self.console, "Missed counter update").ok();
                    return Err(());
                }
            }
            self.last_epoch = Some(counters.epoch);
            //            writeln!(self.console, "Counters: {}, {}, {}", counters.ref_sys, counters.ref_sig, counters.sig_sys);
            return Ok(counters);
        }
    }

    pub fn stabilize(&mut self) -> Result<(), ()> {
        let mut stable_samples = 0;
        loop {
            if self.get_counters().is_ok() {
                break;
            }
        }
        loop {
            let counters = self.get_counters()?;
            let is_valid = self.tolerance_check.check_tolerance(&counters);
            if is_valid {
                stable_samples += 1;
            }
            if stable_samples > 5 {
                return Ok(());
            }
        }
    }

    fn frequency_at_v(&mut self, samples: u8, v: u16) -> Result<f64, ()> {
        let mut f_sum = 0.0;
        self.dac.set_v(v);
        self.get_counters()?;

        for _ in 0..samples {
            let counters = self.get_counters()?;
            f_sum += counters.get_frequency(1.0);
        }

        Ok(f_sum / (samples as f64))
    }

    fn find_operating_point(&mut self) -> Result<u16, ()> {
        let mut min = 0u16;
        let mut max = 0xffffu16;

        let target_freq = self.tolerance_check.target_sig_cnt as f64;

        loop {
            if max < min {
                core::mem::swap(&mut min, &mut max);
            }
            writeln!(self.console, "Hunting: [{}, {}]", min, max).ok();

            let samples = if max - min < 4 { 10 } else { 1 };

            let mut v_limits_adjusted = false;

            let min_freq = self.frequency_at_v(samples, min)?;
            if min_freq > target_freq {
                min = min.saturating_sub(100);
                v_limits_adjusted = true;
            }

            let max_freq = self.frequency_at_v(samples, max)?;
            if max_freq < target_freq {
                max = max.saturating_add(100);
                v_limits_adjusted = true;
            }

            if v_limits_adjusted {
                writeln!(self.console, "DAC codes adjusted").ok();
                continue;
            }

            writeln!(self.console, "Frequencies: [{}, {}]", min_freq, max_freq).ok();

            let test_v = (((self.tolerance_check.target_sig_cnt as f64) - min_freq)
                / (max_freq - min_freq)
                * (max - min) as f64) as u16
                + min;

            if max - min <= 16 {
                return Ok(test_v);
            }
            if (max_freq - min_freq) <= 0.1 {
                return Ok(test_v);
            }

            min = ((min as u32 + test_v as u32) / 2) as u16;
            max = ((max as u32 + test_v as u32) / 2) as u16;
        }
    }

    fn control_sensitivity(&mut self, op_point: u16) -> Result<f64, ()> {
        let min = op_point.saturating_sub(10000);
        let max = op_point.saturating_add(10000);

        let min_f = self.frequency_at_v(5, min)?;
        let max_f = self.frequency_at_v(5, max)?;

        Ok((max_f - min_f) / ((max - min) as f64))
    }

    pub fn run_servo_loop(&mut self, init_op_point: u16, sensitivity: f64) -> Result<(), ()> {
        self.dac.set_v(init_op_point);
        self.get_counters()?;

        let i_factor = 0.01;
        let p_factor = 0.2;
        let min_quick_change = 20;

        let mut op_point = init_op_point;

        writeln!(self.console, "Establishing an initial filter value").ok();
        let initial_filter_value = loop {
            let mut filter = UniformAverageFilter::<U60>::new();
            while !filter.is_full() {
                let counters = self.get_counters()?;
                let raw_freq = counters.get_frequency(1.0);
                filter.add(raw_freq);
            }

            let freq = filter.get();
            let p_error = self.tolerance_check.target_sig_cnt as f64 - freq;

            let new_op_point = (op_point as i32
                + (p_error / sensitivity) as i32)
                .clamp(0, 0xffff) as u16;
            let adj = new_op_point as i32 - op_point as i32;

            if adj != 0 {
                self.dac.set_v(new_op_point);
                op_point = new_op_point;
            }

            if adj.abs() <= 10 {
                break freq;
            } else {
                writeln!(self.console, "Adjusted the DAC by {}, repeating", adj).ok();
            }
        };

        self.output_flag = true;
        writeln!(self.console, "Frequency is in spec, running a slow control loop now").ok();
        let mut filter = ExponentialAverageFilter::new(3600, initial_filter_value);
        let mut i_error = 0.0;

        loop {
            if let Some(counters) = self.get_counters().ok() {
                let raw_freq = counters.get_frequency(1.0);
                filter.add(raw_freq);

                let freq = filter.get();

                let p_error = self.tolerance_check.target_sig_cnt as f64 - freq;
                let raw_p_error = self.tolerance_check.target_sig_cnt as f64 - raw_freq;
                i_error += raw_p_error;

                let new_op_point = (op_point as i32
                    + (((i_error * i_factor + p_error * p_factor) / sensitivity) as i32))
                    .clamp(0, 0xffff) as u16;
                let adj = new_op_point as i32 - op_point as i32;
                let actual_adj = if adj.abs() >= min_quick_change {
                    adj - min_quick_change + adj / 10
                } else {
                    adj / 10
                };
                writeln!(self.console, "freq: {:.03},\traw_freq: {:.03},\terr_i: {:.03}cycles,\terr_p: {:.03},\tadj: {},\tactual: {}",
                         freq, raw_freq, i_error, p_error, adj, actual_adj).ok();
                if actual_adj != 0 {
                    self.dac.set_v(new_op_point);
                    op_point = new_op_point;
                    let filter_adj = actual_adj as f64 * sensitivity;
                    filter.apply_adjustment(filter_adj);
                }
            } else {
                self.error_flag = true;
            }
        }
    }
}

#[entry]
fn main() -> ! {
    let uart = UART::new();
    uart.set_speed(115_200);

    let mut console = ConsoleDevice {};

    uwriteln!(&mut console, "").ok();
    uwriteln!(&mut console, "Starting").ok();

    loop {
        let sck = GPIO0 {};
        let miso = GPIODummyIn {};
        let mosi = GPIO1 {};

        let dac_cs = GPIO2 {};

        let spi_timer = BusyWaitTimer { cycles_to_wait: 2 };
        let spi = bitbang_hal::spi::SPI::new(
            MODE_0,
            miso,
            OldOutputPin::new(mosi),
            OldOutputPin::new(sck),
            spi_timer,
        );

        let mut control_loop = ControlLoop::new(spi, dac_cs, console);
        let _result: Result<(), ()> = try {
            uwriteln!(&mut console, "Stabilizing").ok();
            control_loop.stabilize()?;
            uwriteln!(&mut console, "Finding operating point").ok();
            let v = control_loop.find_operating_point()?;
            uwriteln!(&mut console, "Estimating control response").ok();
            let sensitivity = control_loop.control_sensitivity(v)?;
            writeln!(&mut console, "Starting control loop with initial op {} and control response of {}Hz per 1 LSB code",
                     v, sensitivity).ok();

            control_loop.run_servo_loop(v, sensitivity)?;
        };

        uwriteln!(&mut console, "Restarting").ok();
    }
}
