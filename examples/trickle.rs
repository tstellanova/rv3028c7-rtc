extern crate rv3028c7_rtc;

use linux_embedded_hal::I2cdev;
use chrono::{Utc, Duration};
use rv3028c7_rtc::{RV3028, TrickleChargeCurrentLimiter};
use rtcc::DateTimeAccess;



/// Example enabling/disabling backup power supply trickle charging.
///  Assumptions:
///  - The host this example runs on behaves like a Raspberry Pi 3+ running linux
///  - The device is attached to i2c1 on the host
///
/// The following was tested by enabling i2c-1 on a Raspberry Pi 3+
///  using `sudo raspi-config`
///  and connecting the SDA, SCL, GND, and 3.3V pins from RPi to the RTC


// const MUX_I2C_ADDRESS: u8 = 0x70;
// const MUX_CHAN_FIRST:u8 = 0b0000_0001 ; //channel 0, LSB
// const MUX_CHAN_SECOND:u8 = 0b1000_0000 ; // channel 7, MSB

fn main() {
    // Initialize the I2C bus (device)
    let i2c_bus = I2cdev::new("/dev/i2c-1").expect("Failed to open I2C device");

    // Create instance of the RV3028 driver
    let mut rtc1 = RV3028::new(i2c_bus);
    // Alternate: connect via a mux
    // let mut rtc1 = RV3028::new_with_mux(i2c_bus, MUX_I2C_ADDRESS, MUX_CHAN_SECOND);

    let dt1 = rtc1.datetime().unwrap();
    let sys_dt = Utc::now().naive_utc();
    println!("start sys {}\r\nrtc1 {}\r\n", sys_dt, dt1);

    // enable trickle charging
    let one_enabled = rtc1.toggle_trickle_charge(
        true, TrickleChargeCurrentLimiter::Ohms15k).unwrap();
    println!("rtc1 trickle enabled: {}",one_enabled);

    // enable switchover to backup power (Vbackup)
    let bsm_enabled = rtc1.toggle_backup_switchover(true).unwrap();
    println!("rtc1 backup switchover enabled : {}",bsm_enabled);

    // charge for three seconds
    let dur = Duration::seconds(3);
    println!("charging backup for {}",dur);
    std::thread::sleep(dur.to_std().unwrap());

    // disable trickle charging
    let one_enabled = rtc1.toggle_trickle_charge(
        false, TrickleChargeCurrentLimiter::Ohms3k).unwrap();
    println!("rtc1 trickle enabled: {}",one_enabled);

}
