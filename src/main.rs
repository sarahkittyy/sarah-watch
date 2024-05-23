use embedded_graphics::{
    geometry::Point,
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Circle, Primitive, PrimitiveStyleBuilder},
    Drawable,
};
use esp_idf_svc::hal::{
    delay::{Delay, FreeRtos},
    gpio::{self, OutputPin, PinDriver},
    i2c,
    peripherals::Peripherals,
    prelude::*,
    spi::{
        self,
        config::{Mode, Phase, Polarity},
        SpiDeviceDriver,
    },
};

use gc9a01::{mode::BufferedGraphics, prelude::*, Gc9a01, SPIDisplayInterface};

type DisplayDriver<'a> = Gc9a01<
    SPIInterface<
        SpiDeviceDriver<'a, spi::SpiDriver<'a>>,
        PinDriver<'a, gpio::AnyOutputPin, gpio::Output>,
    >,
    DisplayResolution240x240,
    BufferedGraphics<DisplayResolution240x240>,
>;

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let pins = peripherals.pins;
    let mut delay = Delay::new_default();

    let sck = pins.gpio10;
    let mosi = pins.gpio11;
    let cs = pins.gpio9;
    let dc = pins.gpio8;
    let reset = pins.gpio14;
    let backlight = pins.gpio2;
    let i2c_sda = pins.gpio6;
    let i2c_scl = pins.gpio7;

    let cs_output = cs;
    let dc_output = PinDriver::output(dc.downgrade_output()).unwrap();
    let mut backlight_output = PinDriver::output(backlight.downgrade_output()).unwrap();
    let mut reset_output = PinDriver::output(reset.downgrade_output()).unwrap();

    backlight_output.set_high().unwrap();

    log::info!("Backlight on");

    let i2c =
        i2c::I2cDriver::new(peripherals.i2c0, i2c_sda, i2c_scl, &i2c::I2cConfig::new()).unwrap();

    let driver = spi::SpiDriver::new(
        peripherals.spi2,
        sck,
        mosi,
        None::<gpio::AnyIOPin>,
        &spi::SpiDriverConfig::new(),
    )
    .unwrap();

    let config = spi::config::Config::new()
        .baudrate(2.MHz().into())
        .data_mode(Mode {
            polarity: Polarity::IdleLow,
            phase: Phase::CaptureOnFirstTransition,
        });

    let spi_device = SpiDeviceDriver::new(driver, Some(cs_output), &config).unwrap();

    let interface = SPIDisplayInterface::new(spi_device, dc_output);

    let mut display_driver: Box<DisplayDriver> = Box::new(
        Gc9a01::new(
            interface,
            DisplayResolution240x240,
            DisplayRotation::Rotate0,
        )
        .into_buffered_graphics(),
    );

    display_driver.reset(&mut reset_output, &mut delay).ok();
    display_driver.init(&mut delay).ok();

    log::info!("Driver configured!");

    let style = PrimitiveStyleBuilder::new()
        .stroke_width(4)
        .stroke_color(Rgb565::new(80, 80, 180))
        .fill_color(Rgb565::BLUE)
        .build();

    let mut tick: u32 = 0;
    loop {
        display_driver.clear();
        Circle::new(Point::new(149, 149), 30)
            .into_styled(style)
            .draw::<DisplayDriver>(&mut display_driver)
            .unwrap();
        display_driver.flush().ok();
        tick += 1;
        FreeRtos::delay_ms(2000);
    }
}
