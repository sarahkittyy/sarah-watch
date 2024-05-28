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
    clock::{ClockControl, CpuClock},
    delay::Delay,
    gpio::{AnyPin, Output, PushPull, IO, NO_PIN},
    i2c::I2C,
    peripherals::{Peripherals, I2C0, SPI2},
    prelude::*,
    spi::{master::Spi, FullDuplexMode, SpiMode},
    Blocking,
};
use micromath::F32Ext;

use embedded_hal_bus::spi::ExclusiveDevice;
use gc9a01::{
    display::DisplayResolution240x240,
    mode::{BufferedGraphics, DisplayConfiguration},
    prelude::SPIInterface,
    rotation::DisplayRotation,
    Gc9a01, SPIDisplayInterface,
};

pub mod qmi8658;

//////////////////// ALLOC //////////
extern crate alloc;
use core::{mem::MaybeUninit, ptr::addr_of_mut};

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

fn init_heap() {
    const HEAP_SIZE: usize = 128 * 1024;
    static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();

    unsafe {
        ALLOCATOR.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE);
    }
}

fn print_stack_size() {
    extern "C" {
        static mut _stack_end: u32;
        static mut _stack_start: u32;
    }
    let stack_low = unsafe { addr_of_mut!(_stack_end) as u32 };
    let stack_high = unsafe { addr_of_mut!(_stack_start) as u32 };
    log::info!(
        "Stack: {:#08x} + {} bytes",
        stack_low,
        stack_high - stack_low
    );
}
///////////////////////////////////
type GenericOutputPin = AnyPin<Output<PushPull>>;
type DisplayDriver<'d, SPI> = Gc9a01<
    SPIInterface<
        ExclusiveDevice<Spi<'d, SPI, FullDuplexMode>, GenericOutputPin, Delay>,
        GenericOutputPin,
    >,
    DisplayResolution240x240,
    BufferedGraphics<DisplayResolution240x240>,
>;

#[entry]
fn main() -> ! {
    init_heap();
    esp_println::logger::init_logger(log::LevelFilter::Info);

    log::info!("Hello, logger!");
    log::info!(
        "Memory - used: {}; free: {}",
        ALLOCATOR.used(),
        ALLOCATOR.free()
    );
    print_stack_size();

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

    // screen spi interface initialization
    let spi = Spi::new(ph.SPI2, 30.MHz(), SpiMode::Mode0, &clocks).with_pins(
        Some(sck),
        Some(mosi),
        NO_PIN,
        NO_PIN,
    );
    let spi_driver = ExclusiveDevice::new(spi, GenericOutputPin::from(cs), delay).unwrap();
    let spi_interface = SPIDisplayInterface::new(spi_driver, GenericOutputPin::from(dc));
    let mut display: DisplayDriver<SPI2> = Gc9a01::new(
        spi_interface,
        DisplayResolution240x240,
        DisplayRotation::Rotate180,
    )
    .into_buffered_graphics();

    display.reset(&mut reset, &mut delay).ok();
    display.init(&mut delay).ok();

    log::info!("Display configured");

    // main code loop
    let style = PrimitiveStyleBuilder::new()
        .stroke_width(4)
        .stroke_color(Rgb565::new(9, 60, 31))
        .fill_color(Rgb565::BLUE)
        .build();
    let text_style = MonoTextStyle::new(&FONT_6X10, Rgb565::WHITE);

    let mut text = Text::new("hello", Point::new(120, 30), text_style);
    let circle = Circle::new(Point::new(120 - 15, 120 - 15), 30).into_styled(style);

	Batt

    let mut tick: u32 = 0;
    loop {
        display.clear();

        text.position.x = 120 + (30f32 * (tick as f32 / 10f32).sin()) as i32;
        text.draw(&mut display).unwrap();
        circle.draw(&mut display).unwrap();
        display.flush().ok();
        tick += 1;
    }
}
