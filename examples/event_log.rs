extern crate rv3028c7_rtc;

use linux_embedded_hal::I2cdev;
use chrono::{Duration, Utc};
use rv3028c7_rtc::{RV3028, EventTimeStampLogger, TS_EVENT_SOURCE_EVI};
use std::thread::sleep;
use gpiocdev::{ Request, line::{Value} };

// use linux_embedded_hal::{CdevPin, gpio_cdev::{Chip, LineRequestFlags}};
use rtcc::DateTimeAccess;


const MUX_I2C_ADDRESS: u8 = 0x70;
// const MUX_CHAN_ZERO:u8 = 0b0000_0001 ; //channel 0, LSB
const MUX_CHAN_SEVEN:u8 = 0b1000_0000 ; // channel 7, MSB

// GPIO output pin for the host to send events to the RTC's EVI pin
const GPIO_OUT_PIN: u32 = 27;

/// Example testing real RTC communications,
/// assuming linux environment (such as Raspberry Pi 3+)
/// with RV3028 attached to i2c1.
/// The following was tested by enabling i2c-1 on a Raspberry Pi 3+
/// using `sudo raspi-config`
/// and connecting the SDA, SCL, GND, and 3.3V pins from RPi to the RTC
/// and connecting a gpio output pin  from rpi to the EVI pin of the RTC


fn main() {

    // Initialize the I2C device
    let i2c = I2cdev::new("/dev/i2c-1").expect("Failed to open I2C device");

    // Create a new instance of the RV3028 driver
    // let mut rtc = RV3028::new(i2c);
    // alternate: connect via MUX
    let mut rtc =
      RV3028::new_with_mux(i2c, MUX_I2C_ADDRESS, MUX_CHAN_SEVEN);

    // note the state of the Power On Reset flag and clear it
    let por_state = rtc.check_and_clear_power_on_reset().unwrap();
    println!("Power On Reset was: {}", por_state);

    let sys_dt = Utc::now().naive_utc();
    // use the set_datetime method to ensure all the timekeeping registers on
    // the rtc are aligned to the same values
    rtc.set_datetime(&sys_dt).unwrap();
    let init_dt = rtc.datetime().unwrap();
    println!("sys: {}\r\nrtc: {}", sys_dt, init_dt);

    rtc.configure_event_logging(
        TS_EVENT_SOURCE_EVI, true, true, false, true).unwrap();
    let (event_count, odt) =
      rtc.get_event_count_and_datetime().unwrap();
    if 0 != event_count {
        println!("init count: {} dt: {}", event_count, odt.unwrap());
    }

    // Grab a GPIO output pin on the host for sending digital signals to RTC
    // This is a specific configuration for Raspberry Pi -- YMMV
    let gpio_req = Request::builder()
      .on_chip("/dev/gpiochip0")
      .with_line(GPIO_OUT_PIN)
      // initially inactive (low)
      .as_output(Value::Inactive)
      .request().unwrap();

    let pulse_active_time = Duration::milliseconds(50);
    let pulse_inactive_time = Duration::milliseconds(300);
    let mut pulse_count: u32 = 0;
    for _i in 0..10 {
        println!("Pulse active for {}",pulse_active_time);
        let _ = gpio_req.set_value(GPIO_OUT_PIN, Value::Active);
        sleep(pulse_active_time.to_std().unwrap());
        let _ = gpio_req.set_value(GPIO_OUT_PIN, Value::Inactive);
        println!("Pulse inactive for {}",pulse_inactive_time);
        sleep(pulse_inactive_time.to_std().unwrap());
        pulse_count += 1;
    }

    let (event_count, odt) = rtc.get_event_count_and_datetime().unwrap();
    println!("pulses: {} event_count: {}", pulse_count, event_count);
    if 0 != event_count {
        let dt = odt.unwrap();
        let now = Utc::now().naive_utc();
        println!("event dt: {} sys: {}", dt, now);
    }


}
