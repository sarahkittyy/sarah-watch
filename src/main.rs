#![no_std]
#![no_main]

use alloc::format;
use embedded_graphics::{
    geometry::Point,
    mono_font::{iso_8859_15::FONT_10X20, MonoTextStyle},
    pixelcolor::{Rgb565, RgbColor},
    primitives::{Circle, Primitive, PrimitiveStyleBuilder},
    text::Text,
    Drawable,
};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_backtrace as _;
use esp_hal::{
    analog::adc::{Adc, AdcConfig, Attenuation},
    delay::Delay,
    gpio::{GpioPin, Output, NO_PIN},
    i2c::I2C,
    peripherals::{I2C0, SPI2},
    prelude::*,
    spi::{master::Spi, FullDuplexMode, SpiMode},
    time,
    timer::timg::TimerGroup,
    Blocking,
};
use gc9a01::{
    display::DisplayResolution240x240,
    mode::{BufferedGraphics, DisplayConfiguration},
    prelude::SPIInterface,
    rotation::DisplayRotation,
    Gc9a01, SPIDisplayInterface,
};
use micromath::F32Ext;

pub mod qmi8658;
pub mod sys;

//////////////////// ALLOC //////////
extern crate alloc;
use core::{mem::MaybeUninit, ptr::addr_of_mut};

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

fn init_heap() {
    const HEAP_SIZE: usize = 8 * 1024;
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
type DisplayDriver<'d, SPI> = Gc9a01<
    SPIInterface<
        ExclusiveDevice<Spi<'d, SPI, FullDuplexMode>, Output<'d, GpioPin<9>>, Delay>,
        Output<'d, GpioPin<8>>,
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

    let mut board = sys::Board::init();
    log::info!("Board init'd");
    let mut delay = Delay::new(&board.clocks);

    // backlight init
    board.backlight.set_high();
    log::info!("Set backlight high");

    // i2c for
    let i2c: I2C<I2C0, Blocking> = I2C::new(
        board.i2c0,
        board.i2c_sda,
        board.i2c_scl,
        1.MHz(),
        &board.clocks,
        None,
    );

    // screen spi interface initialization
    let spi = Spi::new(board.spi2, 40.MHz(), SpiMode::Mode0, &board.clocks).with_pins(
        Some(board.sck),
        Some(board.mosi),
        NO_PIN,
        NO_PIN,
    );
    let spi_driver = ExclusiveDevice::new(spi, board.cs, delay).unwrap();
    let spi_interface = SPIDisplayInterface::new(spi_driver, board.dc);
    let mut display: DisplayDriver<SPI2> = Gc9a01::new(
        spi_interface,
        DisplayResolution240x240,
        DisplayRotation::Rotate180,
    )
    .into_buffered_graphics();

    display.reset(&mut board.reset, &mut delay).ok();
    display.init(&mut delay).ok();

    log::info!("Display configured");

    let timer = TimerGroup::new(board.timg1, &board.clocks, None);
    let wifi = esp_wifi::initialize(
        esp_wifi::EspWifiInitFor::Wifi,
        timer.timer0,
        board.rng,
        board.radio_clk,
        &board.clocks,
    )
    .unwrap();

    log::info!("Wifi initialized");

    // battery adc
    let mut adc1_config = AdcConfig::new();
    let mut adc1_pin = adc1_config.enable_pin(board.bat, Attenuation::Attenuation11dB);
    let mut adc1 = Adc::new(board.adc1, adc1_config);

    // main code loop
    let style = PrimitiveStyleBuilder::new()
        .stroke_width(4)
        .stroke_color(Rgb565::new(9, 60, 31))
        .fill_color(Rgb565::BLUE)
        .build();
    let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);

    let circle = Circle::new(Point::new(120 - 15, 120 - 15), 30).into_styled(style);

    let mut tick: u32 = 0;
    loop {
        display.clear();

        // update
        let x = 90 + (30f32 * (tick as f32 / 10f32).sin()) as i32;
        let text = Text::new("hiii <3", Point::new(x, 30), text_style);
        let t = format!(
            "bat: {}",
            nb::block!(adc1.read_oneshot(&mut adc1_pin)).unwrap()
        );
        let text2 = Text::new(&t, Point::new(120, 190), text_style);

        // draw
        text.draw(&mut display).unwrap();
        circle.draw(&mut display).unwrap();
        text2.draw(&mut display).unwrap();

        display.flush().ok();

        tick += 1;

        delay.delay_millis(10);
    }
}
