extern crate rv3028c7_rtc;

use linux_embedded_hal::I2cdev;
use chrono::{Datelike, NaiveDateTime, Timelike, Utc};
use rv3028c7_rtc::RV3028;
use std::time::Duration;
use std::thread::sleep;
use rtcc::DateTimeAccess;


/// Example testing real RTC communications,
/// assuming linux environment (such as Raspberry Pi 3+)
/// with RV3028 attached to i2c1.
/// The following was tested by enabling i2c-1 on a Raspberry Pi 3+
/// using `sudo raspi-config`
/// and connecting the SDA, SCL, GND, and 3.3V pins from RPi to the RTC


fn get_sys_timestamp() -> (NaiveDateTime, u32) {
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
    let (sys_datetime, sys_unix_timestamp) = get_sys_timestamp();
    // use the set_datetime method to ensure all the timekeeping registers on
    // the rtc are aligned to the same values
    rtc.set_datetime(&sys_datetime).unwrap();

    // verify that the unix time and the
    // individual year, month, day registers are set correctly
    let rtc_unix_time = rtc.get_unix_time().unwrap();
    println!("unix sys {} rtc {}  ", sys_unix_timestamp, rtc_unix_time);

    println!("sys date: {}-{:02}-{:02}",
             sys_datetime.date().year(), sys_datetime.date().month(), sys_datetime.date().day());
    let (year, month, day) = rtc.get_ymd().unwrap();
    println!("rtc date: {}-{:02}-{:02}", year, month, day);

    let (hours, minutes, seconds) = rtc.get_hms().unwrap();
    println!("rtc time: {:02}:{:02}:{:02}",
             hours, minutes, seconds);
    println!("sys time: {:02}:{:02}:{:02}",
             sys_datetime.time().hour(), sys_datetime.time().minute(), sys_datetime.time().second());


    // check the drift over and over again
    loop {
        let dt = rtc.datetime().unwrap();
        println!("sys {}\r\nrtc {}", Utc::now().naive_utc(), dt);
        sleep(Duration::from_secs(60));
    }


}
