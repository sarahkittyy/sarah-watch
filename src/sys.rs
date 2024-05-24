use crate::qmi8658::Qmi8568;
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::{
    clock::{ClockControl, Clocks, CpuClock},
    delay::Delay,
    gpio::{AnyPin, Output, PushPull, IO, NO_PIN},
    i2c::I2C,
    peripherals::{Peripherals, I2C0, SPI2},
    prelude::*,
    spi::{master::Spi, FullDuplexMode, SpiMode},
    Blocking,
};
use gc9a01::{
    display::DisplayResolution240x240,
    mode::{BufferedGraphics, DisplayConfiguration},
    prelude::SPIInterface,
    rotation::DisplayRotation,
    Gc9a01, SPIDisplayInterface,
};

type GenericOutputPin = AnyPin<Output<PushPull>>;
type DisplayDriver<'d, SPI> = Gc9a01<
    SPIInterface<
        ExclusiveDevice<Spi<'d, SPI, FullDuplexMode>, GenericOutputPin, Delay>,
        GenericOutputPin,
    >,
    DisplayResolution240x240,
    BufferedGraphics<DisplayResolution240x240>,
>;

/// Wrapper for bare system config
pub struct Sys<'a> {
    pub display: DisplayDriver<'a, SPI2>,
    pub clocks: Clocks<'a>,
    pub gyro: Qmi8568<'a, I2C0>,
}

impl<'a> Sys<'a> {
    /// Initialize base code
    pub fn init() -> Self {
        // base init
        let ph = Peripherals::take();
        let system = ph.SYSTEM.split();

        let clocks = ClockControl::configure(system.clock_control, CpuClock::Clock240MHz).freeze();
        let mut delay = Delay::new(&clocks);

        // gpio pin io initialization
        let io = IO::new(ph.GPIO, ph.IO_MUX);
        let pins = io.pins;

        // gpio pin maps
        let sck = pins.gpio10;
        let mosi = pins.gpio11;
        let cs = pins.gpio9.into_push_pull_output();
        let dc = pins.gpio8.into_push_pull_output();
        let mut reset = pins.gpio14.into_push_pull_output();
        let mut backlight = pins.gpio2.into_push_pull_output();
        let i2c_sda = pins.gpio6;
        let i2c_scl = pins.gpio7;

        // backlight init
        backlight.set_output_high(true);
        log::info!("Set backlight high");

        // i2c for
        let i2c: I2C<I2C0, Blocking> = I2C::new(ph.I2C0, i2c_sda, i2c_scl, 1.MHz(), &clocks, None);

        log::info!("1");

        // screen spi interface initialization
        let spi = Spi::new(ph.SPI2, 30.MHz(), SpiMode::Mode0, &clocks).with_pins(
            Some(sck),
            Some(mosi),
            NO_PIN,
            NO_PIN,
        );
        log::info!("1");
        let spi_driver = ExclusiveDevice::new(spi, GenericOutputPin::from(cs), delay).unwrap();
        log::info!("1");
        let spi_interface = SPIDisplayInterface::new(spi_driver, GenericOutputPin::from(dc));
        log::info!("1");
        let mut display_driver: DisplayDriver<SPI2> = Gc9a01::new(
            spi_interface,
            DisplayResolution240x240,
            DisplayRotation::Rotate180,
        )
        .into_buffered_graphics();
        log::info!("1");

        display_driver.reset(&mut reset, &mut delay).ok();
        log::info!("1");
        display_driver.init(&mut delay).ok();

        log::info!("Display configured");

        // wifi
        let timer = esp_hal::timer::TimerGroup::new(ph.TIMG1, &clocks, None).timer0;
        let _init = esp_wifi::initialize(
            esp_wifi::EspWifiInitFor::Wifi,
            timer,
            esp_hal::rng::Rng::new(ph.RNG),
            system.radio_clock_control,
            &clocks,
        )
        .unwrap();

        log::info!("Wifi initialized");

        Sys {
            display: display_driver,
            clocks,
            gyro: Qmi8568::init(i2c),
        }
    }
}
