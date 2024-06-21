use core::{fmt::Debug, mem};

use esp_hal::{delay::Delay, i2c};

use crate::{
    sys::{TouchInterruptPin, TouchResetPin},
    with, P_I2C1,
};

const CST816_ADDRESS: u8 = 0x15;
const REG_VERSION: u8 = 0xA7;
const REG_INT_CONTROL: u8 = 0xFA;
const REG_MOTION_MASK: u8 = 0xEC;

const INT_MODE_TOUCH: u8 = 0x40;
const INT_MODE_CHANGE: u8 = 0x20;
const INT_MODE_MOTION: u8 = 0x10;
const INT_MODE_LONGPRESS: u8 = 0x01;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Gesture {
    None = 0x00,
    SwipeUp = 0x01,
    SwipeDown = 0x02,
    SwipeLeft = 0x03,
    SwipeRight = 0x04,
    SingleClick = 0x05,
    DoubleClick = 0x0B,
    LongPress = 0x0C,
}

#[derive(Debug)]
pub struct InvalidGestureType(u8);

impl TryFrom<u8> for Gesture {
    type Error = InvalidGestureType;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        use Gesture::*;
        match value {
            0x00 => Ok(None),
            0x01 => Ok(SwipeUp),
            0x02 => Ok(SwipeDown),
            0x03 => Ok(SwipeLeft),
            0x04 => Ok(SwipeRight),
            0x05 => Ok(SingleClick),
            0x0B => Ok(DoubleClick),
            0x0C => Ok(LongPress),
            v => Err(InvalidGestureType(v)),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
#[repr(u8)]
pub enum DeviceType {
    #[default]
    Unknown,
    Cst716 = 0x20,
    Cst816S = 0xB4,
    Cst816T = 0xB5,
    Cst816D = 0xB6,
}

impl From<u8> for DeviceType {
    fn from(value: u8) -> Self {
        match value {
            0x20 => Self::Cst716,
            0xB4 => Self::Cst816S,
            0xB5 => Self::Cst816T,
            0xB6 => Self::Cst816D,
            _ => Self::Unknown,
        }
    }
}

#[derive(Default)]
pub enum TouchMode {
    #[default]
    Touch, // interrupt every 10ms when pressed
    Change, // interrupt when finger changes position
    Fast,   // interrupt after: single click, swipe up, swipe down, swipe left, swipe right.
    Motion, // interrupt after: single click, double click, swipe up, swipe down, swipe left, swipe right, long press.
}

impl TouchMode {
    /// To write to REG_MOTION_MASK register
    pub fn motion_mask_byte(&self) -> u8 {
        match self {
            TouchMode::Motion => 0b001,
            _ => 0b000,
        }
    }
    /// To write to REG_INT_CONTROL register
    pub fn int_control_byte(&self) -> u8 {
        match self {
            TouchMode::Touch => INT_MODE_TOUCH,
            TouchMode::Change => INT_MODE_CHANGE,
            TouchMode::Fast => INT_MODE_MOTION,
            TouchMode::Motion => INT_MODE_MOTION | INT_MODE_LONGPRESS,
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum TouchEvent {
    Down = 0x00,
    Up = 0x01,
    Contact = 0x02,
}

#[derive(Default, Debug)]
pub struct VersionInfo {
    pub chip_id: DeviceType,
    pub proj_id: u8,
    pub fw_version: u8,
    pub factory_id: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct TouchData {
    pub gesture: Gesture,
    pub points: u8,
    pub event: TouchEvent,
    pub x: u16,
    pub y: u16,
}

impl Default for TouchData {
    fn default() -> Self {
        Self {
            gesture: Gesture::None,
            points: 0,
            event: TouchEvent::Down,
            x: 0,
            y: 0,
        }
    }
}

/// Cst816s lcd touch screen driver
pub struct Cst816<'d> {
    int: TouchInterruptPin<'d>,
    reset: TouchResetPin<'d>,
    has_event: bool,
    data: TouchData,
    mode: TouchMode,
}

impl<'d> Cst816<'d> {
    pub fn new(reset: TouchResetPin<'d>, int: TouchInterruptPin<'d>, mode: TouchMode) -> Self {
        Self {
            reset,
            int,
            has_event: false,
            data: TouchData::default(),
            mode,
        }
    }

    pub fn poll(&mut self) -> Option<TouchData> {
        if self.has_event {
            self.has_event = false;
            Some(self.data)
        } else {
            None
        }
    }

    pub fn read_version(&self) -> Result<VersionInfo, i2c::Error> {
        let mut version = [0u8; 4];
        with(&P_I2C1, |i2c| {
            i2c.write_read(CST816_ADDRESS, &[REG_VERSION], &mut version)?;
            Ok(unsafe { mem::transmute(version) })
        })
    }

    fn read_touch(&self) -> Result<TouchData, i2c::Error> {
        let mut buf = [0u8; 6];
        with(&P_I2C1, |i2c| {
            i2c.write_read(CST816_ADDRESS, &[0x01], &mut buf)?;

            Ok(TouchData {
                gesture: buf[0].try_into().unwrap(),
                points: buf[1],
                event: unsafe { mem::transmute(buf[2] >> 6) },
                x: (((buf[2] & 0xF) as u16) << 8) + buf[3] as u16,
                y: (((buf[4] & 0xF) as u16) << 8) + buf[5] as u16,
            })
        })
    }

    pub fn handle_interrupt(&mut self) {
        self.has_event = true;
        self.data = self.read_touch().unwrap();
        log::info!("Got Touch: {:?}", self.data);
        self.int.clear_interrupt();
    }

    pub fn reset(&mut self, delay: &Delay) {
        self.reset.set_low();
        delay.delay_millis(20);
        self.reset.set_high();
        delay.delay_millis(100);
    }

    pub fn begin(&mut self, delay: &Delay) {
        self.reset(delay);
        // write the touch mode
        with(&P_I2C1, |i2c| -> Result<_, _> {
            i2c.write(CST816_ADDRESS, &[REG_INT_CONTROL])?;
            i2c.write(CST816_ADDRESS, &[self.mode.int_control_byte()])?;
            i2c.write(CST816_ADDRESS, &[REG_MOTION_MASK])?;
            i2c.write(CST816_ADDRESS, &[self.mode.motion_mask_byte()])
        })
        .expect("Could not write mode to i2c");
    }
}
