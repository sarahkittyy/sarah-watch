use esp_hal::{
    clock::{ClockControl, Clocks, CpuClock},
    delay::Delay,
    gpio::{GpioPin, Io, Level, Output},
    i2c::I2C,
    peripherals::{Peripherals, ADC1, I2C0, RADIO_CLK, SPI2, TIMG1},
    prelude::*,
    rng::Rng,
    system::SystemControl,
    Blocking,
};

use crate::qmi8658::Qmi8568;

pub struct Board<'d> {
    pub clocks: Clocks<'d>,
    pub i2c0: I2C0,
    pub spi2: SPI2,
    pub adc1: ADC1,
    pub rng: Rng,
    pub timg1: TIMG1,
    pub radio_clk: RADIO_CLK,
    // pins
    pub bat: GpioPin<1>,
    pub sck: GpioPin<10>,
    pub mosi: GpioPin<11>,
    pub cs: Output<'d, GpioPin<9>>,
    pub dc: Output<'d, GpioPin<8>>,
    pub reset: Output<'d, GpioPin<14>>,
    pub backlight: Output<'d, GpioPin<2>>,
    pub i2c_sda: GpioPin<6>,
    pub i2c_scl: GpioPin<7>,
}

impl<'d> Board<'d> {
    pub fn init() -> Self {
        let ph = Peripherals::take();
        let system = SystemControl::new(ph.SYSTEM);

        let io = Io::new(ph.GPIO, ph.IO_MUX);
        let pins = io.pins;

        Board {
            clocks: ClockControl::configure(system.clock_control, CpuClock::Clock240MHz).freeze(),
            spi2: ph.SPI2,
            adc1: ph.ADC1,
            rng: Rng::new(ph.RNG),
            timg1: ph.TIMG1,
            radio_clk: ph.RADIO_CLK,
            bat: pins.gpio1,
            sck: pins.gpio10,
            mosi: pins.gpio11,
            cs: Output::new(pins.gpio9, Level::Low),
            dc: Output::new(pins.gpio8, Level::Low),
            reset: Output::new(pins.gpio14, Level::Low),
            backlight: Output::new(pins.gpio2, Level::Low),
            i2c0: ph.I2C0,
            i2c_sda: pins.gpio6,
            i2c_scl: pins.gpio7,
        }
    }
}
