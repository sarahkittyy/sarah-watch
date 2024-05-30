#![no_std]
#![no_main]

use alloc::format;
use critical_section::Mutex;
use cst816s::Cst816s;
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
    delay::Delay,
    gpio::{Event, GpioPin, Output, NO_PIN},
    i2c::I2C,
    peripherals::{I2C1, SPI2},
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
use micromath::F32Ext;
use sys::Battery;

pub mod cst816s;
pub mod qmi8658;
pub mod sys;

//////////////////// ALLOC //////////
extern crate alloc;
use core::{cell::RefCell, mem::MaybeUninit, ptr::addr_of_mut};

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

pub static P_I2C1: Mutex<RefCell<Option<I2C<'static, I2C1, Blocking>>>> =
    Mutex::new(RefCell::new(None));
pub static P_TOUCH: Mutex<RefCell<Option<Cst816s<'static>>>> = Mutex::new(RefCell::new(None));

pub fn with<G, R>(peripheral: &Mutex<RefCell<Option<G>>>, f: impl FnOnce(&mut G) -> R) -> R {
    critical_section::with(|cs| {
        f(unsafe { peripheral.borrow_ref_mut(cs).as_mut().unwrap_unchecked() })
    })
}

#[handler]
#[ram]
pub fn gpio_interrupt_handler() {
    critical_section::with(|cs| {
        P_TOUCH
            .borrow_ref_mut(cs)
            .as_mut()
            .unwrap()
            .handle_interrupt();
    });
}

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

    // configure i2c0
    let i2c: I2C<I2C1, Blocking> = I2C::new(
        board.i2c1,
        board.i2c_sda,
        board.i2c_scl,
        100.kHz(),
        &board.clocks,
        None,
    );
    critical_section::with(|cs| {
        // store i2c0 as global
        P_I2C1.borrow_ref_mut(cs).replace(i2c);
        // configure touch screen interrupt pin
        board.touch_int.listen(Event::FallingEdge);
        // init touch screen
        let mut touch = Cst816s::new(
            board.touch_reset,
            board.touch_int,
            cst816s::TouchMode::Change,
        );
        touch.begin(&delay);
        log::info!("Touch Screen {:?}", touch.read_version().unwrap());
        P_TOUCH.borrow_ref_mut(cs).replace(touch);
    });

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

    display.reset(&mut board.led_reset, &mut delay).ok();
    display.init(&mut delay).ok();

    log::info!("Display configured");

    /*let timer = TimerGroup::new(board.timg1, &board.clocks, None);
    let wifi = esp_wifi::initialize(
        esp_wifi::EspWifiInitFor::Wifi,
        timer.timer0,
        board.rng,
        board.radio_clk,
        &board.clocks,
    )
    .unwrap();

    log::info!("Wifi initialized");*/

    let mut battery = Battery::new(board.bat, board.adc1);

    // main code loop
    let style = PrimitiveStyleBuilder::new()
        .stroke_width(4)
        .stroke_color(Rgb565::new(9, 60, 31))
        .fill_color(Rgb565::BLUE)
        .build();
    let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);

    let mut pos = Point::new(120, 120);
    let mut circle = Circle::new(pos, 30).into_styled(style);

    let mut tick: u32 = 0;
    loop {
        display.clear();

        // update
        let x = 90 + (30f32 * (tick as f32 / 10f32).sin()) as i32;
        let text = Text::new("hiii <3", Point::new(x, 30), text_style);
        let t = format!("bat: {:.2}", battery.get_voltage());
        let text2 = Text::new(&t, Point::new(120, 190), text_style);
        circle.primitive.top_left = pos - Point::new_equal(15);

        // draw
        text.draw(&mut display).unwrap();
        circle.draw(&mut display).unwrap();
        text2.draw(&mut display).unwrap();

        display.flush().ok();

        tick += 1;

        if let Some(data) = with(&P_TOUCH, Cst816s::poll) {
            pos.x = data.x.into();
            pos.y = data.y.into();
        }

        delay.delay_millis(10);
    }
}
