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

fn get_sys_timestamp() -> u32 {
    let now = Utc::now();
    let now_timestamp = now.timestamp();
    now_timestamp.try_into().unwrap()
}

fn main() {

    // Initialize the I2C device
    let i2c = I2cdev::new("/dev/i2c-1")
        .expect("Failed to open I2C device");

    // Create a new instance of the RV3028 driver
    let mut rtc = RV3028::new(i2c);

    // Pull the current system time and synchronize RTC time to that
    let (_now_timestamp, now_year, now_month, now_date, now_hour, now_minute, now_second) =
        get_sys_date_time();
    let input_unix_time: u32 = get_sys_timestamp();
    rtc.set_unix_time(input_unix_time).expect("couldn't set unix time");
    let output_unix_time = rtc.get_unix_time().expect("couldn't get unix time");
    println!("unix timestamp in: {} out: {}", input_unix_time, output_unix_time);

    println!("sys date: {}-{:02}-{:02}", (2000u32 + now_year as u32), now_month, now_date);
    let (year, month, day) = rtc.get_year_month_day()
        .expect("Failed to get date");
    println!("rtc date: {}-{:02}-{:02}", (2000u32 + year as u32), month, day);

    let (hours, minutes, seconds) = rtc.get_time()
        .expect("Failed to get time");
    println!("rtc time: {:02}:{:02}:{:02}", hours, minutes, seconds);
    println!("sys time: {:02}:{:02}:{:02}", now_hour, now_minute, now_second);


    // check the drift over and over again
    loop {
        let rtc_unix_time = rtc.get_unix_time().expect("couldn't get unix time");
        let sys_unix_timestamp = get_sys_timestamp();
        println!("{}, {}", sys_unix_timestamp, rtc_unix_time);
        sleep(Duration::from_secs(60));
    }


}
