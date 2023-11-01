use linux_embedded_hal::I2cdev;
use chrono::{Datelike, Timelike, Utc};
use rv3028c7_rtc::RV3028;

/**
Example testing date and time access on RTC 
assuming linux environment (such as Raspberry Pi 3+)
with RV3028 attached to i2c1.
*/
fn main() {

    // Initialize the I2C device
    let i2c = I2cdev::new("/dev/i2c-1")
        .expect("Failed to open I2C device");

    // Create a new instance of the RV3028 driver
    let mut rtc = RV3028::new(i2c);

    if let Ok((year, month, day)) = rtc.get_year_month_day() {
      println!("rtc date: {:02}/{:02}/{:02}", year, month, day);
    } 

    let (hours, minutes, seconds) = rtc.get_time()
        .expect("Failed to get time");
    println!("rtc time: {:02}:{:02}:{:02}", hours, minutes, seconds);


    let now = Utc::now();
    let now_hour:u8 = now.hour().try_into().unwrap();
    let now_minute:u8 = now.minute().try_into().unwrap();
    let now_second: u8 = now.second().try_into().unwrap();
    let now_date:u8 = now.day().try_into().unwrap();
    let now_month:u8 = now.month().try_into().unwrap();
    let now_year:u8 = (now.year() - 2000).try_into().unwrap();
    println!("src time: {:02}:{:02}:{:02}", 
	now_hour, now_minute, now_second);
    println!("src date: {:02}/{:02}/{:02}",
	now_year, now_month, now_date);

    // Set the RTC time based on system time
    rtc.set_time(
	now_hour, now_minute, now_second)
	.expect("Failed to set time");
 
    // Get the current time
    let (hours, minutes, seconds) = rtc.get_time()
        .expect("Failed to get time");
    println!("rtc time: {:02}:{:02}:{:02}", hours, minutes, seconds);

    rtc.set_year_month_day(
      now_year, now_month, now_date).unwrap();

    if let Ok((year, month, day)) = rtc.get_year_month_day() {
      println!("rtc date: {:02}/{:02}/{:02}", year, month, day);
    }


}


