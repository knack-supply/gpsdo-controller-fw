use embedded_hal::digital::v2::{InputPin, OutputPin};
use embedded_hal::timer::CountDown;

use crate::hal;
use byteorder::ByteOrder;
use core::fmt::Debug;
use core::pin::Pin;
use crate::util::{OutputPinHold, FullDuplexTransfer};

#[derive(Debug)]
pub enum Error<E: Debug> {
    ConfigValidationMismatch {
        desired_config: ConfigRegister,
        actual_config: ConfigRegister,
    },
    InvalidConversionValue {
        raw_value: i16,
    },
    BusError {
        error: E,
    }
}

pub type ConversionResult<E> = core::result::Result<i16, Error<E>>;

#[derive(Debug, Copy, Clone)]
enum ReservedBit {
    Invalid,
    Valid,
}

impl From<u8> for ReservedBit {
    fn from(v: u8) -> Self {
        match v {
            1u8 => ReservedBit::Valid,
            _ => ReservedBit::Invalid,
        }
    }
}

impl Into<u8> for ReservedBit {
    fn into(self) -> u8 {
        match self {
            ReservedBit::Valid => 1,
            ReservedBit::Invalid => 0,
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum Nop {
    Invalid00,
    Valid,
    Invalid10,
    Invalid11,
}

impl From<u8> for Nop {
    fn from(v: u8) -> Self {
        match v {
            0b00u8 => Nop::Invalid00,
            0b01u8 => Nop::Valid,
            0b10u8 => Nop::Invalid10,
            _ => Nop::Invalid11,
        }
    }
}

impl Into<u8> for Nop {
    fn into(self) -> u8 {
        match self {
            Nop::Invalid00 => 0b00u8,
            Nop::Valid => 0b01u8,
            Nop::Invalid10 => 0b10u8,
            Nop::Invalid11 => 0b11u8,
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum Mode {
    Continuous,
    SingleShot,
}

impl From<u8> for Mode {
    fn from(v: u8) -> Self {
        match v {
            1u8 => Mode::SingleShot,
            _ => Mode::Continuous,
        }
    }
}

impl Into<u8> for Mode {
    fn into(self) -> u8 {
        match self {
            Mode::SingleShot => 1,
            Mode::Continuous => 0,
        }
    }
}

pub enum DataRate {
    _128SPS,
    _250SPS,
    _490SPS,
    _920SPS,
    _1600SPS,
    _2400SPS,
    _3300SPS,
}

#[derive(Debug, Copy, Clone)]
enum RawDataRate {
    _128SPS,
    _250SPS,
    _490SPS,
    _920SPS,
    _1600SPS,
    _2400SPS,
    _3300SPS,
    Invalid,
}

impl From<DataRate> for RawDataRate {
    fn from(dr: DataRate) -> Self {
        match dr {
            DataRate::_128SPS => RawDataRate::_128SPS,
            DataRate::_250SPS => RawDataRate::_250SPS,
            DataRate::_490SPS => RawDataRate::_490SPS,
            DataRate::_920SPS => RawDataRate::_920SPS,
            DataRate::_1600SPS => RawDataRate::_1600SPS,
            DataRate::_2400SPS => RawDataRate::_2400SPS,
            DataRate::_3300SPS => RawDataRate::_3300SPS,
        }
    }
}

impl From<u8> for RawDataRate {
    fn from(v: u8) -> Self {
        match v {
            0b000u8 => RawDataRate::_128SPS,
            0b001u8 => RawDataRate::_250SPS,
            0b010u8 => RawDataRate::_490SPS,
            0b011u8 => RawDataRate::_920SPS,
            0b100u8 => RawDataRate::_1600SPS,
            0b101u8 => RawDataRate::_2400SPS,
            0b110u8 => RawDataRate::_3300SPS,
            _ => RawDataRate::Invalid,
        }
    }
}

impl Into<u8> for RawDataRate {
    fn into(self) -> u8 {
        match self {
            RawDataRate::_128SPS => 0b000u8,
            RawDataRate::_250SPS => 0b001u8,
            RawDataRate::_490SPS => 0b010u8,
            RawDataRate::_920SPS => 0b011u8,
            RawDataRate::_1600SPS => 0b100u8,
            RawDataRate::_2400SPS => 0b101u8,
            RawDataRate::_3300SPS => 0b110u8,
            RawDataRate::Invalid => 0b111u8,
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum TsMode {
    ADC,
    TemperatureSensor,
}

impl From<u8> for TsMode {
    fn from(v: u8) -> Self {
        match v {
            1u8 => TsMode::TemperatureSensor,
            _ => TsMode::ADC,
        }
    }
}

impl Into<u8> for TsMode {
    fn into(self) -> u8 {
        match self {
            TsMode::TemperatureSensor => 1,
            TsMode::ADC => 0,
        }
    }
}

#[allow(non_camel_case_types)]
pub enum Gain {
    FSR_6_144V,
    FSR_4_096V,
    FSR_2_048V,
    FSR_1_024V,
    FSR_0_512V,
    FSR_0_256V,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone)]
enum PGA {
    FSR_6_144V,
    FSR_4_096V,
    FSR_2_048V,
    FSR_1_024V,
    FSR_0_512V,
    FSR_0_256V,
    FSR_0_256V_1,
    FSR_0_256V_2,
}

impl From<Gain> for PGA {
    fn from(gain: Gain) -> Self {
        match gain {
            Gain::FSR_6_144V => PGA::FSR_6_144V,
            Gain::FSR_4_096V => PGA::FSR_4_096V,
            Gain::FSR_2_048V => PGA::FSR_2_048V,
            Gain::FSR_1_024V => PGA::FSR_1_024V,
            Gain::FSR_0_512V => PGA::FSR_0_512V,
            Gain::FSR_0_256V => PGA::FSR_0_256V,
        }
    }
}

impl From<u8> for PGA {
    fn from(v: u8) -> Self {
        match v {
            0b000u8 => PGA::FSR_6_144V,
            0b001u8 => PGA::FSR_4_096V,
            0b010u8 => PGA::FSR_2_048V,
            0b011u8 => PGA::FSR_1_024V,
            0b100u8 => PGA::FSR_0_512V,
            0b101u8 => PGA::FSR_0_256V,
            0b110u8 => PGA::FSR_0_256V_1,
            _ => PGA::FSR_0_256V_2,
        }
    }
}

impl Into<u8> for PGA {
    fn into(self) -> u8 {
        match self {
            PGA::FSR_6_144V => 0b000u8,
            PGA::FSR_4_096V => 0b001u8,
            PGA::FSR_2_048V => 0b010u8,
            PGA::FSR_1_024V => 0b011u8,
            PGA::FSR_0_512V => 0b100u8,
            PGA::FSR_0_256V => 0b101u8,
            PGA::FSR_0_256V_1 => 0b110u8,
            PGA::FSR_0_256V_2 => 0b111u8,
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum Mux {
    P0N1,
    P0N3,
    P1N3,
    P2N3,
    P0,
    P1,
    P2,
    P3,
}

impl From<u8> for Mux {
    fn from(v: u8) -> Self {
        match v {
            0b000u8 => Mux::P0N1,
            0b001u8 => Mux::P0N3,
            0b010u8 => Mux::P1N3,
            0b011u8 => Mux::P2N3,
            0b100u8 => Mux::P0,
            0b101u8 => Mux::P1,
            0b110u8 => Mux::P2,
            _ => Mux::P3,
        }
    }
}

impl Into<u8> for Mux {
    fn into(self) -> u8 {
        match self {
            Mux::P0N1 => 0b000u8,
            Mux::P0N3 => 0b001u8,
            Mux::P1N3 => 0b010u8,
            Mux::P2N3 => 0b011u8,
            Mux::P0 => 0b100u8,
            Mux::P1 => 0b101u8,
            Mux::P2 => 0b110u8,
            Mux::P3 => 0b111u8,
        }
    }
}

bitfield!{
    pub struct ConfigRegister(u16);
    impl Debug;
    u8;
    from into ReservedBit, reserved, set_reserved: 0, 0;
    from into Nop, nop, set_nop: 2, 1;
    pullup_en, set_pullup_en: 3;
    from into TsMode, ts_mode, set_ts_mode: 4, 4;
    from into RawDataRate, data_rate, set_data_rate: 7, 5;
    from into Mode, mode, set_mode: 8, 8;
    from into PGA, pga, set_pga: 11, 9;
    from into Mux, mux, set_mux: 14, 12;
    singleshot_start, set_singleshot_start: 15;
}

impl Default for ConfigRegister {
    fn default() -> Self {
        Self(0)
    }
}

impl Clone for ConfigRegister {
    fn clone(&self) -> Self {
        ConfigRegister(self.0)
    }
}

impl ConfigRegister {
    pub fn new(channel: Channel, gain: Gain, data_rate: DataRate) -> ConfigRegister {
        let mut cfg_reg = ConfigRegister::default();
        cfg_reg.set_reserved(ReservedBit::Valid);
        cfg_reg.set_nop(Nop::Valid);
        cfg_reg.set_pullup_en(true);

        match channel {
            Channel::Channel0 => {
                cfg_reg.set_ts_mode(TsMode::ADC);
                cfg_reg.set_mux(Mux::P0);
            }
            Channel::Channel1 => {
                cfg_reg.set_ts_mode(TsMode::ADC);
                cfg_reg.set_mux(Mux::P1);
            }
            Channel::Channel2 => {
                cfg_reg.set_ts_mode(TsMode::ADC);
                cfg_reg.set_mux(Mux::P2);
            }
            Channel::Channel3 => {
                cfg_reg.set_ts_mode(TsMode::ADC);
                cfg_reg.set_mux(Mux::P3);
            }
            Channel::Channel0Against1 => {
                cfg_reg.set_ts_mode(TsMode::ADC);
                cfg_reg.set_mux(Mux::P0N1);
            }
            Channel::Channel0Against3 => {
                cfg_reg.set_ts_mode(TsMode::ADC);
                cfg_reg.set_mux(Mux::P0N3);
            }
            Channel::Channel1Against3 => {
                cfg_reg.set_ts_mode(TsMode::ADC);
                cfg_reg.set_mux(Mux::P1N3);
            }
            Channel::Channel2Against3 => {
                cfg_reg.set_ts_mode(TsMode::ADC);
                cfg_reg.set_mux(Mux::P2N3);
            }
            Channel::Temperature => {
                cfg_reg.set_ts_mode(TsMode::TemperatureSensor);
                cfg_reg.set_mux(Mux::P0N1);
            }
        }
        cfg_reg.set_data_rate(data_rate.into());
        cfg_reg.set_mode(Mode::SingleShot);
        cfg_reg.set_pga(gain.into());
        cfg_reg.set_singleshot_start(true);

        cfg_reg
    }
}

pub enum Channel {
    Channel0,
    Channel1,
    Channel2,
    Channel3,
    Channel0Against1,
    Channel0Against3,
    Channel1Against3,
    Channel2Against3,
    Temperature,
}

impl From<ExternalChannel> for Channel {
    fn from(c: ExternalChannel) -> Self {
        match c {
            ExternalChannel::Channel0 => Channel::Channel0,
            ExternalChannel::Channel1 => Channel::Channel1,
            ExternalChannel::Channel2 => Channel::Channel2,
            ExternalChannel::Channel3 => Channel::Channel3,
            ExternalChannel::Channel0Against1 => Channel::Channel0Against1,
            ExternalChannel::Channel0Against3 => Channel::Channel0Against3,
            ExternalChannel::Channel1Against3 => Channel::Channel1Against3,
            ExternalChannel::Channel2Against3 => Channel::Channel2Against3,
        }
    }
}

pub enum ExternalChannel {
    Channel0,
    Channel1,
    Channel2,
    Channel3,
    Channel0Against1,
    Channel0Against3,
    Channel1Against3,
    Channel2Against3,
}

pub struct ADS1018<SPI, CS: OutputPin, MISO: InputPin> where
    SPI: embedded_hal::blocking::spi::Transfer<u8>, SPI::Error: Debug
{
    spi: SPI,
    cs: CS,
    miso: MISO,
}

impl<SPI, CS: OutputPin, MISO: InputPin> ADS1018<SPI, CS, MISO>
    where SPI: embedded_hal::blocking::spi::Transfer<u8>, SPI::Error: Debug
{
    pub fn new(spi: SPI, cs: CS, miso: MISO) -> Self {
        Self { spi, cs, miso }
    }

    pub fn read_temperature(&mut self, data_rate: DataRate) -> ConversionResult<SPI::Error> {
        self.read_raw(Channel::Temperature, Gain::FSR_2_048V, data_rate)
    }

    pub fn read_channel(&mut self, channel: ExternalChannel, gain: Gain, data_rate: DataRate) -> ConversionResult<SPI::Error> {
        self.read_raw(channel.into(), gain, data_rate)
    }

    fn read_raw(&mut self, channel: Channel, gain: Gain, data_rate: DataRate) -> ConversionResult<SPI::Error> {
        let mut delay = hal::BusyWaitTimer::new(10);

        let cfg_reg = ConfigRegister::new(channel, gain, data_rate);

        let mut buf = [0u8, 0u8];
        byteorder::BE::write_u16(&mut buf, cfg_reg.0);

        self.cs.set_low().ok();
        delay.wait().ok();
        self.spi.transfer(&mut buf).ok();
        byteorder::BE::write_u16(&mut buf, 0);
        if let Ok(applied_config) = self.spi.transfer(&mut buf) {
            let applied_config = byteorder::BE::read_u16(applied_config) & 0x7fff | 0x0001;
            let expected_config = cfg_reg.0 & 0x7fff | 0x0001;
            if applied_config != expected_config {
                self.cs.set_high().ok();
                return Err(Error::ConfigValidationMismatch {
                    desired_config: ConfigRegister(expected_config),
                    actual_config: ConfigRegister(applied_config),
                });
            }
        }
        self.cs.set_high().ok();

        let mut large_delay = hal::BusyWaitTimer::new(100);

        loop {
            self.cs.set_low().ok();
            large_delay.wait().ok();
            if self.miso.is_low().unwrap_or(false) {
                self.cs.set_high().ok();
                delay.wait().ok();
                self.cs.set_low().ok();
                delay.wait().ok();
                byteorder::BE::write_u16(&mut buf, 0);
                let measurement = self.spi.transfer(&mut buf);
                self.cs.set_high().ok();
                return match measurement {
                    Ok(measurement) => {
                        let value = byteorder::BigEndian::read_i16(measurement);
                        let actual_value = value >> 4;
                        if value & 0b1111 != 0 {
                            Err(Error::InvalidConversionValue {
                                raw_value: value,
                            })
                        } else {
                            Ok(actual_value)
                        }
                    },
                    Err(error) => {
                        Err(Error::BusError { error })
                    }
                }
            } else {
                self.cs.set_high().ok();
            }
        }
    }
}

pub struct ADS1018Async<SPI, CS: OutputPin, MISO: InputPin> where
    SPI: embedded_hal::spi::FullDuplex<u8>, SPI::Error: Debug
{
    spi: SPI,
    cs: CS,
    miso: MISO,
}

impl<SPI, CS: OutputPin, MISO: InputPin> ADS1018Async<SPI, CS, MISO>
    where SPI: embedded_hal::spi::FullDuplex<u8>, SPI::Error: Debug
{
    pub fn new(spi: SPI, cs: CS, miso: MISO) -> Self {
        Self { spi, cs, miso }
    }

    pub async fn read_temperature(self: Pin<&mut Self>, data_rate: DataRate) -> ConversionResult<SPI::Error> {
        self.read_raw(Channel::Temperature, Gain::FSR_2_048V, data_rate).await
    }

    pub async fn read_channel(self: Pin<&mut Self>, channel: ExternalChannel, gain: Gain, data_rate: DataRate) -> ConversionResult<SPI::Error> {
        self.read_raw(channel.into(), gain, data_rate).await
    }

    fn pin_get_io<'a>(self: Pin<&'a mut Self>) -> (&'a mut SPI, &'a mut CS, &'a mut MISO) {
        unsafe {
            let self_mut = self.get_unchecked_mut();
            (
                &mut self_mut.spi,
                &mut self_mut.cs,
                &mut self_mut.miso,
            )
        }
    }

    async fn read_raw(self: Pin<&mut Self>, channel: Channel, gain: Gain, data_rate: DataRate) -> ConversionResult<SPI::Error> {
        let mut delay = hal::BusyWaitTimer::new(10);
        let (spi, cs, miso): (&mut SPI, &mut CS, &mut MISO) = self.pin_get_io();

        let cfg_reg = ConfigRegister::new(channel, gain, data_rate);

        let buf = [0u8, 0u8];
        pin_mut!(buf);

        byteorder::BE::write_u16(&mut *buf, cfg_reg.0);

        {
            let _ = cs.hold_low();
            for b in buf.iter() {
                nb_future!(spi.send(*b)).await.map_err(|error| Error::BusError { error })?;
            }

            byteorder::BE::write_u16(&mut *buf, 0);

            spi.transfer(&mut *buf).await.map_err(|error| Error::BusError { error })?;
        }

        let applied_config = byteorder::BE::read_u16(&*buf) & 0x7fff | 0x0001;
        let expected_config = cfg_reg.0 & 0x7fff | 0x0001;
        if applied_config != expected_config {
            return Err(Error::ConfigValidationMismatch {
                desired_config: ConfigRegister(expected_config),
                actual_config: ConfigRegister(applied_config),
            });
        }

        let mut large_delay = hal::BusyWaitTimer::new(100);

        loop {
            {
                let _ = cs.hold_low();
                large_delay.wait().ok();
                if !miso.is_low().unwrap_or(false) {
                    continue
                }
            }

            delay.wait().ok();

            {
                let cs_active = cs.hold_low();
                delay.wait().ok();
                byteorder::BE::write_u16(&mut *buf, 0);

                spi.transfer(&mut *buf).await.map_err(|error| Error::BusError { error })?;
                drop(cs_active);

                let value = byteorder::BigEndian::read_i16(&*buf);
                let actual_value = value >> 4;
                return if value & 0b1111 != 0 {
                    Err(Error::InvalidConversionValue {
                        raw_value: value,
                    })
                } else {
                    Ok(actual_value)
                }
            }
        }
    }
}

