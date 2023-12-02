extern crate rv3028c7_rtc;

use linux_embedded_hal::I2cdev;
use chrono::{Utc};
use rv3028c7_rtc::{RV3028, TrickleChargeCurrentLimiter};
use rtcc::DateTimeAccess;



/// Example enabling/disabling backup power supply trickle charging.
/// For this example we are muxing between two different RTCs
/// with the same i2c device address.
///
///  Assumptions:
///  - The i2c mux behaves like a Texas Instruments TCA9548A
///  - One RTC is attached to channel 0 on the mux; the other is attached to channel 7
///  - The host this example runs on behaves like a Raspberry Pi 3+ running linux
/// - The mux is attached to i2c1 on the host
///
/// The following was tested by enabling i2c-1 on a Raspberry Pi 3+
///  using `sudo raspi-config`
///  and connecting the SDA, SCL, GND, and 3.3V pins from RPi to the mux
///  and relevant pins to the two RTCs


const MUX_I2C_ADDRESS: u8 = 0x70;
const MUX_CHAN_FIRST:u8 = 0b0000_0001 ; //channel 0, LSB
const MUX_CHAN_SECOND:u8 = 0b1000_0000 ; // channel 7, MSB

fn main() {
    // Initialize the I2C bus (device)
    let i2c = I2cdev::new("/dev/i2c-1").expect("Failed to open I2C device");
    let i2c_bus = shared_bus::BusManagerSimple::new(i2c);

    // Create two instances of the RV3028 driver
    let mut rtc1 = RV3028::new_with_mux(i2c_bus.acquire_i2c(), MUX_I2C_ADDRESS, MUX_CHAN_FIRST);
    let mut rtc2 = RV3028::new_with_mux(i2c_bus.acquire_i2c(), MUX_I2C_ADDRESS, MUX_CHAN_SECOND);

    let dt1 = rtc1.datetime().unwrap();
    let dt2 = rtc2.datetime().unwrap();
    let sys_dt = Utc::now().naive_utc();
    println!("start sys {}\r\nrtc1 {}\r\nrtc2 {}", sys_dt, dt1, dt2);

    // enable trickle charging on both
    let one_enabled = rtc1.toggle_trickle_charge(
        true, TrickleChargeCurrentLimiter::Ohms15k).unwrap();
    let two_enabled = rtc2.toggle_trickle_charge(
        true, TrickleChargeCurrentLimiter::Ohms15k).unwrap();

    println!("rtc1 enabled: {}\r\nrtc2 enabled: {}",one_enabled, two_enabled);

    // disable on both
    let one_enabled = rtc1.toggle_trickle_charge(
        false, TrickleChargeCurrentLimiter::Ohms3k).unwrap();
    let two_enabled = rtc2.toggle_trickle_charge(
        false, TrickleChargeCurrentLimiter::Ohms3k).unwrap();
    println!("rtc1 enabled: {}\r\nrtc2 enabled: {}",one_enabled, two_enabled);

}
