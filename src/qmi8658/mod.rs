use esp_hal::{
    i2c::{Instance, I2C},
    Blocking,
};

/// Qmi8568 3 axis gyroscope
pub struct Qmi8568<'d, T: Instance> {
    i2c: I2C<'d, T, Blocking>,
}

impl<'a, T: Instance> Qmi8568<'a, T> {
    pub fn init(i2c: I2C<'a, T, Blocking>) -> Self {
        Self { i2c }
    }
}
