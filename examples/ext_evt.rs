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


fn send_pulses(num_pulses: u32, out_pin: u32) {
  // Grab a GPIO output pin on the host for sending digital signals to RTC
  // This is a specific configuration for Raspberry Pi -- YMMV
  let gpio_req = Request::builder()
    .on_chip("/dev/gpiochip0")
    .with_line(out_pin)
    // initially inactive (low)
    .as_output(Value::Inactive)
    .request().unwrap();

  let pulse_active_time = Duration::milliseconds(50);
  let pulse_inactive_time = Duration::milliseconds(300);
  for _i in 0..num_pulses {
    println!("Pulse active for {}",pulse_active_time);
    let _ = gpio_req.set_value(out_pin, Value::Active);
    sleep(pulse_active_time.to_std().unwrap());
    let _ = gpio_req.set_value(out_pin, Value::Inactive);
    println!("Pulse inactive for {}",pulse_inactive_time);
    sleep(pulse_inactive_time.to_std().unwrap());
  }

}
fn main() {

  // Initialize the I2C device
  let i2c = I2cdev::new("/dev/i2c-1").expect("Failed to open I2C device");

  // Create a new instance of the RV3028 driver
  // let mut rtc = RV3028::new(i2c);
  // Alternate: connect via MUX
  let mut rtc =
    RV3028::new_with_mux(i2c, MUX_I2C_ADDRESS, MUX_CHAN_SEVEN);

  let sys_dt = Utc::now().naive_utc();
  // use the set_datetime method to ensure all the timekeeping registers on
  // the rtc are aligned to the same values
  rtc.set_datetime(&sys_dt).unwrap();
  let init_dt = rtc.datetime().unwrap();
  println!("sys: {}\r\nrtc: {}", sys_dt, init_dt);

  rtc.clear_all_int_out_bits().unwrap();
  rtc.clear_all_int_clockout_bits().unwrap();
  rtc.clear_all_status_flags().unwrap();

  // Configure the RTC for external events on EVI pin
  rtc.config_ext_event_detection(
    true, false, 0u8, false).unwrap();

  // Start the time stamp function recording
  rtc.config_timestamp_logging(
    TS_EVENT_SOURCE_EVI, true, true).unwrap();

  // send a series of pulses on the hosts GPIO output pin
  let num_pulses = 10;
  send_pulses(num_pulses, GPIO_OUT_PIN);

  // find out how many pulses the RTC observed on its EVI pin
  let (event_count, odt) =
    rtc.get_event_count_and_datetime().unwrap();
  println!("pulses: {} event_count: {}", num_pulses, event_count);
  if 0 != event_count {
    let dt = odt.unwrap();
    let now = Utc::now().naive_utc();
    println!("event dt: {} sys: {}", dt, now);
  }

}