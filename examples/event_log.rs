extern crate rv3028c7_rtc;

use linux_embedded_hal::I2cdev;
use chrono::{NaiveDateTime, NaiveTime, Utc};
use rv3028c7_rtc::{RV3028, EventTimeStampLogger, TS_EVENT_SOURCE_EVI};
use std::time::Duration;
use std::thread::sleep;
// use sysfs_gpio::{Pin, Direction};
use linux_embedded_hal::{CdevPin,
                         gpio_cdev::{Chip, LineRequestFlags}};
use embedded_hal::digital::v2::OutputPin;
use rtcc::DateTimeAccess;


/// Example testing real RTC communications,
/// assuming linux environment (such as Raspberry Pi 3+)
/// with RV3028 attached to i2c1.
/// The following was tested by enabling i2c-1 on a Raspberry Pi 3+
/// using `sudo raspi-config`
/// and connecting the SDA, SCL, GND, and 3.3V pins from RPi to the RTC
/// and connecting a gpio pin (Pin 17 in this case) from rpi to the EVI pin of the RTC

fn get_sys_timestamp() -> (NaiveDateTime, u32) {
    let now = Utc::now();
    let now_timestamp = now.timestamp();
    (now.naive_utc(), now_timestamp.try_into().unwrap() )
}


fn main() {
    // Grab a GPIO output pin on the host for sending digital signals to RTC
    // This is a specific configuration for Raspberry Pi -- YMMV
    let mut gpiochip = Chip::new("/dev/gpiochip0").unwrap();
    let line = gpiochip.get_line(17).unwrap();
    let handle = line.request(LineRequestFlags::OUTPUT, 1, "gpio_evi").unwrap();
    let mut evi_pin = CdevPin::new(handle).unwrap();
    evi_pin.set_low().unwrap();

    // Initialize the I2C device
    let i2c = I2cdev::new("/dev/i2c-1")
        .expect("Failed to open I2C device");
    let i2c_bus = shared_bus::BusManagerSimple::new(i2c);

    // Create a new instance of the RV3028 driver
    let mut rtc = RV3028::new(i2c_bus.acquire_i2c());

    let (sys_datetime, sys_unix_timestamp) = get_sys_timestamp();
    // use the set_datetime method to ensure all the timekeeping registers on
    // the rtc are aligned to the same values
    rtc.set_datetime(&sys_datetime).unwrap();
    let rtc_unix_time = rtc.get_unix_time().unwrap();
    // verify that the individual year, month, day registers are set correctly
    let (year, month, day) = rtc.get_ymd().unwrap();
    println!("start sys {} rtc {} ymd {} {} {} ", sys_unix_timestamp, rtc_unix_time, year,month,day);


    let init_dt = rtc.datetime().unwrap();
    println!("init_dt: {}", init_dt);

    // clear any existing event logging
    rtc.toggle_event_log(false).unwrap();
    rtc.set_event_source(TS_EVENT_SOURCE_EVI).unwrap();
    rtc.toggle_event_high_low(true).unwrap();
    // allow saving of the latest event time stamp
    rtc.toggle_time_stamp_overwrite(true).unwrap();
    sleep(Duration::from_millis(100));
    rtc.toggle_event_log(true).unwrap();

    let (event_count, odt) =
      rtc.get_event_count_and_datetime().unwrap();
    if 0 != event_count {
        println!("init count: {} dt: {}", event_count, odt.unwrap());
    }

    let mut toggle_count: u32 = 0;
    for _i in 0..90 {
        evi_pin.set_high().unwrap();
        sleep(Duration::from_micros(100));
        evi_pin.set_low().unwrap();
        toggle_count += 1;
        let (event_count, odt) =
          rtc.get_event_count_and_datetime().unwrap();
        if 0 != event_count {
            let dt = odt.unwrap();
            let now = Utc::now();
            println!("toggles: {} count: {} dt: {} sys: {}", toggle_count, event_count, dt, now);
        }
        // wait another second before sending a pulse
        sleep(Duration::from_secs(1));
    }


}
