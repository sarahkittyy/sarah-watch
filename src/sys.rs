use esp_hal::{
    analog::adc::{Adc, AdcConfig, AdcPin, Attenuation},
    clock::{ClockControl, Clocks, CpuClock},
    gpio::{GpioPin, Input, Io, Level, Output, Pull},
    peripherals::{Peripherals, ADC1, I2C0, I2C1, RADIO_CLK, SPI2, TIMG1},
    prelude::*,
    rng::Rng,
    system::SystemControl,
};

use crate::gpio_interrupt_handler;

pub type BatteryPin = AdcPin<GpioPin<1>, ADC1, ()>;
pub type SdaPin = GpioPin<6>;
pub type SclPin = GpioPin<7>;
pub type TouchResetPin<'d> = Output<'d, GpioPin<13>>;
pub type TouchInterruptPin<'d> = Input<'d, GpioPin<5>>;
pub type LedResetPin<'d> = Output<'d, GpioPin<14>>;
pub type BacklightPin<'d> = Output<'d, GpioPin<2>>;

pub struct Board<'d> {
    pub clocks: Clocks<'d>,
    pub i2c0: I2C0,
    pub i2c1: I2C1,
    pub spi2: SPI2,
    pub rng: Rng,
    pub timg1: TIMG1,
    pub radio_clk: RADIO_CLK,
    // pins
    pub sck: GpioPin<10>,
    pub mosi: GpioPin<11>,
    pub cs: Output<'d, GpioPin<9>>,
    pub dc: Output<'d, GpioPin<8>>,
    pub touch_reset: TouchResetPin<'d>,
    pub touch_int: TouchInterruptPin<'d>,
    pub led_reset: LedResetPin<'d>,
    pub backlight: BacklightPin<'d>,
    pub i2c_sda: SdaPin,
    pub i2c_scl: SclPin,

    pub bat: BatteryPin,
    pub adc1: Adc<'d, ADC1>,
}

impl<'d> Board<'d> {
    pub fn init() -> Self {
        let ph = Peripherals::take();
        let system = SystemControl::new(ph.SYSTEM);

        let mut io = Io::new(ph.GPIO, ph.IO_MUX);
        io.set_interrupt_handler(gpio_interrupt_handler);
        let pins = io.pins;

        let mut adc1_config = AdcConfig::new();

        Board {
            clocks: ClockControl::configure(system.clock_control, CpuClock::Clock240MHz).freeze(),
            spi2: ph.SPI2,
            rng: Rng::new(ph.RNG),
            timg1: ph.TIMG1,
            radio_clk: ph.RADIO_CLK,
            sck: pins.gpio10,
            mosi: pins.gpio11,
            cs: Output::new(pins.gpio9, Level::Low),
            dc: Output::new(pins.gpio8, Level::Low),
            touch_int: Input::new(pins.gpio5, Pull::Up),
            touch_reset: Output::new(pins.gpio13, Level::High),
            led_reset: Output::new(pins.gpio14, Level::Low),
            backlight: Output::new(pins.gpio2, Level::Low),
            i2c0: ph.I2C0,
            i2c1: ph.I2C1,
            i2c_sda: pins.gpio6,
            i2c_scl: pins.gpio7,

            bat: adc1_config.enable_pin_with_cal(pins.gpio1, Attenuation::Attenuation11dB),
            adc1: Adc::new(ph.ADC1, adc1_config),
        }
    }
}

pub struct Battery<'d> {
    pin: BatteryPin,
    adc1: Adc<'d, ADC1>,
}

impl<'d> Battery<'d> {
    pub fn new(pin: BatteryPin, adc1: Adc<'d, ADC1>) -> Self {
        Self { pin, adc1 }
    }

    pub fn get_voltage(&mut self) -> f32 {
        let v: f32 = nb::block!(self.adc1.read_oneshot(&mut self.pin))
            .unwrap()
            .into();
        (3.7 / 4096.0) * v
    }
}
