use embedded_hal::digital::v2::OutputPin;

pub struct MAX5216<SPI: embedded_hal::blocking::spi::Write<u8>, CS: OutputPin> {
    spi: SPI,
    cs: CS,
}

impl<SPI: embedded_hal::blocking::spi::Write<u8>, CS: OutputPin> MAX5216<SPI, CS> {
    pub fn new(spi: SPI, cs: CS) -> Self {
        Self { spi, cs }
    }

    pub fn set_v(&mut self, v: u16) {
        self.cs.set_low().ok();
        self.spi
            .write(&[
                0b0100_0000u8 | ((v & 0b1111_1100_0000_0000u16) >> 10) as u8,
                (v >> 2) as u8,
                (v << 6) as u8 & 0b1100_0000u8,
            ])
            .ok();
        self.cs.set_high().ok();
    }
}
