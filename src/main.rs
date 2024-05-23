#![no_std]
#![no_main]

use embedded_graphics::{
    geometry::Point,
    pixelcolor::{Rgb565, RgbColor},
    primitives::{Circle, Primitive, PrimitiveStyleBuilder},
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

//////////////////// ALLOC //////////
extern crate alloc;
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
    let ph = Peripherals::take();
    let system = ph.SYSTEM.split();

    let clocks = ClockControl::max(system.clock_control).freeze();
    let mut delay = Delay::new(&clocks);
    init_heap();

    esp_println::logger::init_logger_from_env();
    log::info!("Logged init'd, Booting...");

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

    backlight.set_output_high(true);
    log::info!("Set backlight high.");

    let i2c = I2C::new(ph.I2C0, i2c_sda, i2c_scl, 1.MHz(), &clocks, None);

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
        DisplayRotation::Rotate0,
    )
    .into_buffered_graphics();

    display_driver.reset(&mut reset, &mut delay).ok();
    display_driver.init(&mut delay).ok();

    log::info!("Display configured!");

    let style = PrimitiveStyleBuilder::new()
        .stroke_width(4)
        .stroke_color(Rgb565::new(9, 60, 31))
        .fill_color(Rgb565::BLUE)
        .build();

    let mut tick: u32 = 0;
    loop {
        log::info!("loop");
        display_driver.clear();
        Circle::new(Point::new(140, 140), 30)
            .into_styled(style)
            .draw(&mut display_driver)
            .unwrap();
        display_driver.flush().ok();
        tick += 1;
        delay.delay_millis(2000);
    }
}
