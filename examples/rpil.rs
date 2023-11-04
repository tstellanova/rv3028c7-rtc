extern crate rv3028c7_rtc;

use linux_embedded_hal::I2cdev;
use chrono::{Datelike, Timelike, Utc};
use rv3028c7_rtc::RV3028;
use std::time::Duration;
use std::thread::sleep;



/**
Example testing real RTC communications,
assuming linux environment (such as Raspberry Pi 3+)
with RV3028 attached to i2c1.
The following was tested by enabling i2c-1 on a Raspberry Pi 3+
using `sudo raspi-config`
and connecting the SDA, SCL, GND, and 3.3V pins from RPi to the RTC
*/

fn get_sys_date_time() -> (i64, u8, u8, u8, u8, u8, u8)
{
    let now = Utc::now();
    let now_timestamp = now.timestamp();
    let now_hour:u8 = now.hour().try_into().unwrap();
    let now_minute:u8 = now.minute().try_into().unwrap();
    let now_second: u8 = now.second().try_into().unwrap();
    let now_date:u8 = now.day().try_into().unwrap();
    let now_month:u8 = now.month().try_into().unwrap();
    // the RTC only handles years in the 0..99 range (corresponding with 2000 to 2099)
    let now_year: u32 = now.year().try_into().unwrap();
    //println!("src time: {:02}:{:02}:{:02}", now_hour, now_minute, now_second);
    //println!("src date: {}-{:02}-{:02}", now_year, now_month, now_date);
    let now_year:u8 = (now_year - 2000u32).try_into().unwrap();
    (now_timestamp, now_year, now_month, now_date, now_hour, now_minute, now_second)
}


fn main() {

    // Initialize the I2C device
    let i2c = I2cdev::new("/dev/i2c-1")
        .expect("Failed to open I2C device");

    // Create a new instance of the RV3028 driver
    let mut rtc = RV3028::new(i2c);

    let (year, month, day) = rtc.get_year_month_day()
        .expect("Failed to get date");
    println!("rtc date: {}-{:02}-{:02}", (2000u32 + year as u32), month, day);

    let (hours, minutes, seconds) = rtc.get_time()
        .expect("Failed to get time");
    println!("rtc time: {:02}:{:02}:{:02}", hours, minutes, seconds);

    // Pull the current system time
    let (_now_timestamp, now_year, now_month, now_date, now_hour, now_minute, now_second) =
        get_sys_date_time();

    // Set the RTC time and date based on system time
    rtc.set_year_month_day( now_year, now_month, now_date).expect("Failed to set date");
    rtc.set_time(now_hour, now_minute, now_second).expect("Failed to set time");

    // check the drift over and over again
    loop {
        let (hours, minutes, seconds) = rtc.get_time().expect("Failed to get time");
        let (year, month, date) = rtc.get_year_month_day().expect("Failed to get date");
        let (now_timestamp, now_year, now_month, now_date, now_hour, now_minute, now_second) =
            get_sys_date_time();
        println!("{}, {}-{:02}-{:02}, {:02}:{:02}:{:02}",
                 now_timestamp,
                 now_year - year, now_month - month, now_date - date,
                 now_hour - hours, now_minute - minutes, now_second - seconds);

        sleep(Duration::from_secs(60));
    }


}


