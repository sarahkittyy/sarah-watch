#![no_std]
#![no_main]

use embedded_graphics::{
    geometry::Point,
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::{Rgb565, RgbColor},
    primitives::{Circle, Primitive, PrimitiveStyleBuilder},
    text::Text,
    Drawable,
};
use esp_backtrace as _;
use esp_hal::{
    clock::ClockControl,
    delay::Delay,
    gpio::{IO, NO_PIN},
    i2c::I2C,
    peripherals::Peripherals,
    prelude::*,
    spi::{self, SpiMode},
};
use gc9a01::{
    display::DisplayResolution240x240, mode::DisplayConfiguration, rotation::DisplayRotation,
    Gc9a01, SPIDisplayInterface,
};
use micromath::F32Ext;

//////////////////// ALLOC //////////
use core::mem::MaybeUninit;

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

fn init_heap() {
    const HEAP_SIZE: usize = 32 * 1024;
    static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();

    unsafe {
        ALLOCATOR.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE);
    }
}
///////////////////////////////////

#[entry]
fn main() -> ! {
    // base init
    let ph = Peripherals::take();
    let system = ph.SYSTEM.split();

    let clocks = ClockControl::max(system.clock_control).freeze();
    let mut delay = Delay::new(&clocks);
    init_heap();

    esp_println::logger::init_logger_from_env();
    log::info!("Logged init'd, Booting...");

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
    let i2c = I2C::new(ph.I2C0, i2c_sda, i2c_scl, 1.MHz(), &clocks, None);

    // screen spi interface initialization
    let spi = spi::master::Spi::new(ph.SPI2, 2.MHz(), SpiMode::Mode0, &clocks).with_pins(
        Some(sck),
        Some(mosi),
        NO_PIN,
        NO_PIN,
    );
    let spi_driver = embedded_hal_bus::spi::ExclusiveDevice::new(spi, cs, delay).unwrap();
    let spi_interface = SPIDisplayInterface::new(spi_driver, dc);
    let mut display_driver = Gc9a01::new(
        spi_interface,
        DisplayResolution240x240,
        DisplayRotation::Rotate180,
    )
    .into_buffered_graphics();

    display_driver.reset(&mut reset, &mut delay).ok();
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

    // main code loop
    let style = PrimitiveStyleBuilder::new()
        .stroke_width(4)
        .stroke_color(Rgb565::new(9, 60, 31))
        .fill_color(Rgb565::BLUE)
        .build();
    let text_style = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);

    let mut text = Text::new("hello", Point::new(120, 30), text_style);
    let circle = Circle::new(Point::new(120 - 15, 120 - 15), 30).into_styled(style);

    let mut tick: u32 = 0;
    loop {
        display_driver.clear();

        text.position.x = 120 + (30f32 * (tick as f32 / 10f32).sin()) as i32;
        text.draw(&mut display_driver).unwrap();
        circle.draw(&mut display_driver).unwrap();
        display_driver.flush().ok();
        tick += 1;
    }
}
