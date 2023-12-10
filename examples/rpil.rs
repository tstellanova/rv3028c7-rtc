extern crate rv3028c7_rtc;

use linux_embedded_hal::I2cdev;
use chrono::{Utc};
use rv3028c7_rtc::{RV3028, Datelike, DateTimeAccess, Duration, NaiveDateTime, Timelike};
use std::thread::sleep;

/// Example testing real RTC communications,
/// assuming linux environment (such as Raspberry Pi 3+)
/// with RV3028 attached to i2c1.
/// The following was tested by enabling i2c-1 on a Raspberry Pi 3+
/// using `sudo raspi-config`
/// and connecting the SDA, SCL, GND, and 3.3V pins from RPi to the RTC

fn get_sys_datetime_timestamp() -> (NaiveDateTime, u32) {
    let now = Utc::now();
    let now_timestamp = now.timestamp();
    (now.naive_utc(), now_timestamp.try_into().unwrap() )
}

fn main() {

    // Initialize the I2C device
    let i2c = I2cdev::new("/dev/i2c-1").expect("Failed to open I2C device");

    // Create a new instance of the RV3028 driver
    let mut rtc = RV3028::new(i2c);

    // Pull the current system time and synchronize RTC time to that
    let (sys_dt, sys_unix_timestamp) = get_sys_datetime_timestamp();
    // use the set_datetime method to ensure all the timekeeping registers on
    // the rtc are aligned to the same values
    rtc.set_datetime(&sys_dt).unwrap();

    // verify that the unix time and the
    // individual year, month, day registers are set correctly
    let rtc_unix_timestamp = rtc.get_unix_time().unwrap();
    println!("sys unix {}\r\nrtc unix {}  ", sys_unix_timestamp, rtc_unix_timestamp);

    let rtc_dt = rtc.datetime().unwrap();
    println!("sys dt: {}", sys_dt);
    println!("rtc dt: {}", rtc_dt);

    println!("sys Y-M-D {}-{:02}-{:02}",
             sys_dt.date().year(), sys_dt.date().month(), sys_dt.date().day());
    let (year, month, day) = rtc.get_ymd().unwrap();
    println!("rtc Y-M-D {}-{:02}-{:02}", year, month, day);

    println!("sys H:M:S {:02}:{:02}:{:02}",
             sys_dt.time().hour(), sys_dt.time().minute(), sys_dt.time().second());
    let (hours, minutes, seconds) = rtc.get_hms().unwrap();
    println!("rtc H:M:S {:02}:{:02}:{:02}",
             hours, minutes, seconds);

    // check the drift over and over again
    loop {
        let dt = rtc.datetime().unwrap();
        println!("sys {}\r\nrtc {}", Utc::now().naive_utc(), dt);
        sleep(Duration::seconds(5).to_std().unwrap());
    }


}
