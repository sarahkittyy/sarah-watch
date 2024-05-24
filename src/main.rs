#![no_std]
#![no_main]

use alloc::vec::Vec;
use critical_section::Mutex;
use embedded_graphics::{
    geometry::Point,
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::{Rgb565, RgbColor},
    primitives::{Circle, Primitive, PrimitiveStyleBuilder},
    text::Text,
    Drawable,
};
use esp_backtrace as _;
use esp_hal::{assist_debug::DebugAssist, interrupt, peripherals::Interrupt, prelude::*};
use micromath::F32Ext;

pub mod qmi8658;
pub mod sys;

//////////////////// ALLOC //////////
extern crate alloc;
use core::{cell::RefCell, mem::MaybeUninit, ptr::addr_of_mut};

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
///
///
pub fn recursive_stack_and_heap_allocation(depth: usize, allocation_size: usize) {
    // Allocate some data on the stack
    let mut stack_data = [0u8; 1024]; // 1 KB of data on the stack
    stack_data[0] = 1; // Access the array to ensure it's not optimized out

    // Heap allocation
    let mut test_vec: Vec<u8> = Vec::new();
    test_vec.resize(allocation_size, 0);
    test_vec[0] = 1; // Access the array to ensure it's not optimized out

    log::info!(
        "Depth: {}, Stack usage: {} bytes, Heap allocation: {}, Memory - used: {}; free: {}",
        depth,
        depth * 1024,
        allocation_size,
        ALLOCATOR.used(),
        ALLOCATOR.free()
    );

    if depth > 1000 {
        return; // Limit recursion depth to prevent stack overflow
    }

    recursive_stack_and_heap_allocation(depth + 1, allocation_size);
    // Keep allocation size the same, since variables are not going out of scope in recursion
}

////////////// STACK PROTECTION ////////////////////////
// Static variable to hold DebugAssist
static DA: Mutex<RefCell<Option<DebugAssist<'static>>>> = Mutex::new(RefCell::new(None));

fn install_stack_guard(mut da: DebugAssist<'static>, safe_area_size: u32) {
    extern "C" {
        static mut _stack_end: u32;
        static mut _stack_start: u32;
    }
    let stack_low = unsafe { addr_of_mut!(_stack_end) as u32 };
    let stack_high = unsafe { addr_of_mut!(_stack_start) as u32 };
    log::info!(
        "Safe stack {} bytes",
        stack_high - stack_low - safe_area_size
    );
    da.enable_region0_monitor(stack_low, stack_low + safe_area_size, true, true);

    critical_section::with(|cs| DA.borrow_ref_mut(cs).replace(da));
    interrupt::enable(Interrupt::ASSIST_DEBUG, interrupt::Priority::Priority1).unwrap();
}

#[handler]
fn assist_debug() {
    critical_section::with(|cs| {
        log::error!("\n\nPossible Stack Overflow Detected");
        let mut da = DA.borrow_ref_mut(cs);
        let da = da.as_mut().unwrap();
        if da.is_region0_monitor_interrupt_set() {
            let pc = da.get_region_monitor_pc();
            log::info!("PC = 0x{:x}", pc);
            da.clear_region0_monitor_interrupt();
            da.disable_region0_monitor();
            loop {}
        }
    });
}
/////////////////////////////////////////////////////

#[entry]
fn main() -> ! {
    init_heap();
    esp_println::logger::init_logger_from_env();
    log::info!("Hello, logger!");
    log::info!(
        "Memory - used: {}; free: {}",
        ALLOCATOR.used(),
        ALLOCATOR.free()
    );

    let mut sys = sys::Sys::init();

    // debugassist guards against stack overflows
    let debug = DebugAssist::new(sys.ph.ASSIST_DEBUG, Some(assist_debug));
    install_stack_guard(debug, 4096);

    log::info!("Sys init'd");

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
