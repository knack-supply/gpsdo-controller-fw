#![cfg(not(test))]
#![cfg_attr(not(test), no_std)]
#![no_main]
#![feature(proc_macro_hygiene)]
//#![feature(asm)]
#![feature(clamp)]
#![feature(try_blocks)]
#![cfg_attr(not(test), feature(alloc_error_handler))]
#![allow(deprecated)]

#[cfg(not(test))]
extern crate panic_halt;
#[macro_use]
extern crate ufmt;
extern crate ks_gpsdo;

#[macro_use]
extern crate picorv32_rt;

#[cfg(not(test))]
extern crate alloc;
#[cfg(test)]
extern crate std;

use core::fmt::Write;
use embedded_hal::digital::v1_compat::{OldOutputPin, OldInputPin};
use embedded_hal::digital::v2::OutputPin;
use embedded_hal::spi::MODE_1;
use embedded_hal::timer::CountDown;
use ks_gpsdo::freq_counter::{FrequencyCounters, FrequencyCountersToleranceCheck, FrequencyCountersFuture};
use ks_gpsdo::max5216::MAX5216;
use ks_gpsdo::picosoc::*;
use picorv32_rt::entry;
use typenum::consts::*;
use ufmt::uWrite;
use ks_gpsdo::filter::UniformAverageFilter;
use ks_gpsdo::bus::SharedBusManager;
use core::sync::atomic;
use core::sync::atomic::Ordering;
use ks_gpsdo::control::FeedbackControl;
#[cfg(not(test))]
use ks_gpsdo::allocator::RISCVHeap;
use ks_gpsdo::ads1018::ADS1018;
use ks_gpsdo::ads1018;
use ks_gpsdo::hal::BusyWaitTimer;

#[cfg(not(test))]
#[allow(non_upper_case_globals)]
extern "C" {
    static _sheap: u8;
    static _eheap: u8;
}

#[cfg(not(test))]
#[global_allocator]
static ALLOCATOR: RISCVHeap = RISCVHeap::empty();

#[cfg(not(test))]
#[alloc_error_handler]
fn alloc_error(_: core::alloc::Layout) -> ! {
    panic!("Allocation failure");
}

struct ControlLoop<SPI: embedded_hal::blocking::spi::Write<u8>, CS: OutputPin, CONSOLE: uWrite + Write> {
    console: CONSOLE,
    tolerance_check: FrequencyCountersToleranceCheck,
    dac: MAX5216<SPI, CS>,
    last_epoch: Option<u8>,
    error_flag: bool,
    output_flag: bool,
}

impl<SPI: embedded_hal::blocking::spi::Write<u8>, CS: OutputPin, CONSOLE: uWrite + Write> ControlLoop<SPI, CS, CONSOLE> {
    pub fn new(spi: SPI, cs: CS, console: CONSOLE) -> Self {
        Self {
            console,
            tolerance_check: FrequencyCountersToleranceCheck {
                target_sig_cnt: 10_000_000,
                sig_cnt_tolerance: 200,
                target_clk: if cfg!(feature = "hx8k") {
                    201_000_000
                } else if cfg!(feature = "up5k") {
                    100_500_000
                } else {
                    unreachable!()
                },
                clk_tolerance: 10_000,
            },
            dac: MAX5216::new(spi, cs),
            last_epoch: None,
            error_flag: false,
            output_flag: false,
        }
    }

    pub fn get_counters(&mut self) -> Result<FrequencyCounters, ()> {
        writeln!(self.console, "Getting counters").ok();
        let r = ks_gpsdo::futures::block_on(FrequencyCountersFuture::new());
        writeln!(self.console, "Counters: {:?}", r).ok();

        if let (Ok(counters), Some(last_epoch)) = (r, self.last_epoch) {
            if (last_epoch + 1) & 0b11 != counters.epoch {
                writeln!(self.console, "Missed counter update").ok();
                return Err(());
            }
        }
        self.last_epoch = r.ok().map(|c| c.epoch);

        r
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
        writeln!(self.console, "DAC code: {}", v).ok();
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

            let samples = if max - min < 1024 { 10 } else { 1 };

            let mut v_limits_adjusted = false;

            let min_freq = self.frequency_at_v(samples, min)?;
            if min_freq > target_freq {
                min = min.saturating_sub(1000);
                v_limits_adjusted = true;
            }

            let max_freq = self.frequency_at_v(samples, max)?;
            if max_freq < target_freq {
                max = max.saturating_add(1000);
                v_limits_adjusted = true;
            }

            writeln!(self.console, "Frequencies: [{}, {}]", min_freq, max_freq).ok();

            if v_limits_adjusted {
                writeln!(self.console, "DAC codes adjusted").ok();
                continue;
            }

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

            let new_op_point = (op_point as i32 + (p_error / sensitivity) as i32)
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

        let mut feedback_control = FeedbackControl::new(
            op_point,
            initial_filter_value,
            self.tolerance_check.target_sig_cnt as f64,
            sensitivity,
            0.001,
            0.1,
            0.05,
            0.01,
        );

        loop {
            if let Some(counters) = self.get_counters().ok() {
                let raw_freq = counters.get_frequency(1.0);
                feedback_control.set_frequency(raw_freq);
                feedback_control.tick();

                let new_op_point = feedback_control.get_dac_code();

                writeln!(self.console, "freq: {:.03},\traw_freq: {:.03},\terr_i: {:.03}cycles,\tadj: {}",
                         feedback_control.get_filtered_frequency(), raw_freq,
                         feedback_control.get_i_error(), new_op_point as i32 - op_point as i32).ok();
                if new_op_point != op_point {
                    self.dac.set_v(new_op_point);
                    op_point = new_op_point;
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

    unsafe {
        picorv32::interrupt::enable();
    }

    let mut console = ConsoleDevice {};

    uwriteln!(&mut console, "").ok();
    uwriteln!(&mut console, "Starting").ok();

    #[cfg(not(test))]
    unsafe {
        let heap_start = &_sheap as *const u8 as usize;
        let heap_end = &_eheap as *const u8 as usize;
        writeln!(&mut console, "Initializing heap at {:08x}, of size {:08x}", heap_start, heap_end - heap_start).ok();
        ALLOCATOR.init(heap_start, heap_end - heap_start);
    }

    let sck = GPIO0 {};
    let miso = GPIO4 {};
    let mosi = GPIO1 {};

    let mut dac_cs = GPIO2 {};
    let mut adc_cs = GPIO3 {};
    dac_cs.set_high().ok();
    adc_cs.set_high().ok();

    let spi = {
        let spi_timer = BusyWaitTimer::new(20);
        let spi = bitbang_hal::spi::SPI::new(
            MODE_1,
            OldInputPin::from(miso),
            OldOutputPin::from(mosi),
            OldOutputPin::from(sck),
            spi_timer,
        );

        SharedBusManager::new(spi)
    };

    BusyWaitTimer::new(100000).wait().ok();
    let mut adc = ADS1018::new(spi.acquire(), adc_cs, GPIO4 {});

    loop {
        match adc.read_temperature(ads1018::DataRate::_128SPS) {
            Ok(reading) => {
                writeln!(console, "ADC temperature:           \t{:04x}\t{:.03}⁰C", reading,
                         reading as f64 * 0.125).ok();
            }
            Err(e) => {
                writeln!(console, "ADC temperature error: {:?}", e).ok();
            }
        }

        match adc.read_channel(ads1018::ExternalChannel::Channel0,
                                                ads1018::Gain::FSR_2_048V,
                                                ads1018::DataRate::_128SPS) {
            Ok(reading) => {
                writeln!(console, "ADC channel 0 (VCOCXO I):  \t{:04x}\t{:.03}A", reading,
                        reading as f64 * 0.001 * 0.5).ok();
            }
            Err(e) => {
                writeln!(console, "ADC channel 0 error: {:?}", e).ok();
            }
        }
        match adc.read_channel(ads1018::ExternalChannel::Channel1,
                                                ads1018::Gain::FSR_4_096V,
                                                ads1018::DataRate::_128SPS) {
            Ok(reading) => {
                writeln!(console, "ADC channel 1 (VCOCXO Vcc):\t{:04x}\t{:.03}V", reading,
                         reading as f64 * 0.002 * (12.0 / 5.0)).ok();
            }
            Err(e) => {
                writeln!(console, "ADC channel 1 error: {:?}", e).ok();
            }
        }
        match adc.read_channel(ads1018::ExternalChannel::Channel2,
                                                ads1018::Gain::FSR_1_024V,
                                                ads1018::DataRate::_128SPS) {
            Ok(reading) => {
                writeln!(console, "ADC channel 2 (VCOCXO t):  \t{:04x}\t{:.03}⁰C", reading,
                         ((reading as f64 * 0.0005) - 0.5) * 100.0).ok();
            }
            Err(e) => {
                writeln!(console, "ADC channel 2 error: {:?}", e).ok();
            }
        }
        match adc.read_channel(ads1018::ExternalChannel::Channel3,
                                                ads1018::Gain::FSR_1_024V,
                                                ads1018::DataRate::_128SPS) {
            Ok(reading) => {
                writeln!(console, "ADC channel 3 (ambient t): \t{:04x}\t{:.03}⁰C", reading,
                         ((reading as f64 * 0.0005) - 0.5) * 100.0).ok();
            }
            Err(e) => {
                writeln!(console, "ADC channel 3 error: {:?}", e).ok();
            }
        }
        break
    }

    loop {
        let mut control_loop = ControlLoop::new(spi.acquire(), dac_cs, console);
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

pub fn timer(_regs: &picorv32_rt::PicoRV32StoredRegisters) {
    let mut console = ConsoleDevice {};
    uwriteln!(&mut console, "IRQ: Timer").ok();
}

pub fn illegal_instruction(regs: &picorv32_rt::PicoRV32StoredRegisters) {
    let mut console = ConsoleDevice {};
    uwriteln!(&mut console, "IRQ: Illegal instruction").ok();
    writeln!(&mut console, "{:?}", regs).ok();
    loop {
        atomic::compiler_fence(Ordering::SeqCst);
    }
}

pub fn bus_error(regs: &picorv32_rt::PicoRV32StoredRegisters) {
    let mut console = ConsoleDevice {};
    uwriteln!(&mut console, "IRQ: Bus error").ok();
    writeln!(&mut console, "{:?}", regs).ok();
    loop {
        atomic::compiler_fence(Ordering::SeqCst);
    }
}

pub fn frequency_counter_ready(_regs: &picorv32_rt::PicoRV32StoredRegisters) {
//    let mut console = ConsoleDevice {};
//    uwriteln!(&mut console, "IRQ: Frequency counter ready").ok();

    unsafe { ks_gpsdo::freq_counter::FrequencyCounterInterruptHandler::handle_interrupt(); }
}

picorv32_interrupts!(
    0: timer,
    1: illegal_instruction,
    2: bus_error,
    5: frequency_counter_ready
);
