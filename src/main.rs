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
use esp_hal::prelude::*;
use micromath::F32Ext;

pub mod qmi8658;
pub mod sys;

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
    init_heap();
    esp_println::logger::init_logger_from_env();
    let mut sys = sys::Sys::init();

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
        sys.display.clear();

        text.position.x = 120 + (30f32 * (tick as f32 / 10f32).sin()) as i32;
        text.draw(&mut sys.display).unwrap();
        circle.draw(&mut sys.display).unwrap();
        sys.display.flush().ok();
        tick += 1;
    }
}
