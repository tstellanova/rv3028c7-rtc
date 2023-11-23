extern crate rv3028c7_rtc;

use linux_embedded_hal::I2cdev;
use chrono::{ Utc};
use rv3028c7_rtc::{RV3028, EventTimeStampLogger};
use std::time::Duration;
use std::thread::sleep;
// use sysfs_gpio::{Pin, Direction};
use linux_embedded_hal::{CdevPin,
                         gpio_cdev::{Chip, LineRequestFlags}};
use embedded_hal::digital::v2::OutputPin;



/// Example testing real RTC communications,
/// assuming linux environment (such as Raspberry Pi 3+)
/// with RV3028 attached to i2c1.
/// The following was tested by enabling i2c-1 on a Raspberry Pi 3+
/// using `sudo raspi-config`
/// and connecting the SDA, SCL, GND, and 3.3V pins from RPi to the RTC
/// and connecting a gpio pin (Pin 17 in this case) from rpi to the EVI pin of the RTC

fn get_sys_timestamp() -> u32 {
    let now = Utc::now();
    let now_timestamp = now.timestamp();
    now_timestamp.try_into().unwrap()
}


fn main() {
    // Grab a GPIO output pin on the host for sending digital signals to RTC
    // This is a specific configuration for Raspberry Pi -- YMMV
    let mut gpiochip = Chip::new("/dev/gpiochip0").unwrap();
    let line = gpiochip.get_line(17).unwrap();
    let handle = line.request(LineRequestFlags::OUTPUT, 1, "gpio_evi").unwrap();
    let mut evi_pin = CdevPin::new(handle).expect("new CdevPin");
    evi_pin.set_low().expect("set low");

    // Initialize the I2C device
    let i2c = I2cdev::new("/dev/i2c-1")
        .expect("Failed to open I2C device");
    let i2c_bus = shared_bus::BusManagerSimple::new(i2c);

    // Create a new instance of the RV3028 driver
    let mut rtc = RV3028::new(i2c_bus.acquire_i2c());

    let sys_unix_timestamp = get_sys_timestamp();
    rtc.set_unix_time(sys_unix_timestamp).expect("set_unix_time");
    let rtc_unix_time = rtc.get_unix_time().expect("couldn't get unix time");
    println!("start sys {} rtc {} ", sys_unix_timestamp, rtc_unix_time);

    // clear any existing event logging
    rtc.toggle_event_log(false).unwrap();
    sleep(Duration::from_millis(100));
    rtc.toggle_event_log(true).unwrap();

    let (event_count, dt) =
      rtc.get_event_count_and_datetime().expect("get_event_count_and_datetime");
    if 0 != event_count {
        println!("init count: {} dt: {}", event_count, dt);
    }

    let mut toggle_count: u32 = 0;
    for _i in 0..10 {
        evi_pin.set_high().expect("EVI set high");
        sleep(Duration::from_millis(100));
        evi_pin.set_low().expect("EVI set low");
        sleep(Duration::from_millis(100));
        toggle_count += 1;
        let (event_count, dt) =
          rtc.get_event_count_and_datetime().expect("get_event_count_and_datetime");
        if 0 != event_count {
            println!("toggles: {} count: {} dt: {}", toggle_count, event_count, dt);
        }
    }


}
