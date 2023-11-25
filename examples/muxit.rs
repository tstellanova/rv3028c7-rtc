extern crate rv3028c7_rtc;

use std::convert::TryInto;
use core::ops::Add;
use linux_embedded_hal::I2cdev;
use chrono::{NaiveDateTime, Timelike, Utc};
use rv3028c7_rtc::RV3028;
use std::time::{Duration };
use std::thread::sleep;
use rtcc::DateTimeAccess;


/**
Example testing muxing between two different RTCs
with the same i2c device address.

Assumptions:
- The i2c mux behaves like a Texas Instruments TCA9548A
- One RTC is attached to channel 0 on the mux; the other is attached to channel 7
- The platform this example runs on behaves like a Raspberry Pi 3+ running linux
- The mux is attached to i2c1

The following was tested by enabling i2c-1 on a Raspberry Pi 3+
using `sudo raspi-config`
and connecting the SDA, SCL, GND, and 3.3V pins from RPi to the mux
and relevant pins to the two RTCs
 */


fn get_sys_dt_and_subsec() -> (NaiveDateTime, u32) {
  let now = Utc::now();
  let dt = now.naive_utc();
  (dt,  now.timestamp_subsec_micros())
}

fn get_simple_dt_and_subsec() -> (NaiveDateTime, u32) {
  let now = Utc::now();
  let dt = now.naive_utc();
  let simple_dt = dt.with_nanosecond(0).unwrap();
  (simple_dt, now.timestamp_subsec_micros())
}

fn get_sys_timestamp_and_micros() -> (u32, u32) {
  let now = Utc::now();
  (
    now.timestamp().try_into().unwrap(),
    now.timestamp_subsec_micros(),
  )
}



const MUX_I2C_ADDRESS: u8 = 0x70;
const MUX_CHAN_FIRST:u8 = 0b0000_0001 ; //channel 0, LSB
const MUX_CHAN_SECOND:u8 = 0b1000_0000 ; // channel 7, MSB

fn main() {

  // Initialize the I2C bus (device)
  let i2c = I2cdev::new("/dev/i2c-1").expect("Failed to open I2C device");
  let i2c_bus = shared_bus::BusManagerSimple::new(i2c);

  // Create two instances of the RV3028 driver
  let mut rtc1 = RV3028::new_with_mux(i2c_bus.acquire_i2c(), MUX_I2C_ADDRESS, MUX_CHAN_FIRST);
  let mut rtc2 = RV3028::new_with_mux(i2c_bus.acquire_i2c(), MUX_I2C_ADDRESS, MUX_CHAN_SECOND);

  // setting RTC times and rechecking sys time seems to take ~3800 micros with my equipment
  const IDEAL_DELAY_US_LOW: u32 = 1_000_000 - 3800;
  const IDEAL_DELAY_US_HIGH: u32 = IDEAL_DELAY_US_LOW + 50;

  'session: loop {
    let next_dt =
      loop {
        // catch the next whole second transition
        let (simple_dt, subsec_micros) = get_simple_dt_and_subsec();
        if subsec_micros < IDEAL_DELAY_US_LOW || subsec_micros > IDEAL_DELAY_US_HIGH  {
          continue 'session;
        }
        break simple_dt.add(Duration::from_secs(1))
      };

    rtc1.set_datetime(&next_dt).unwrap();
    rtc2.set_datetime(&next_dt).unwrap();

    let (sys_dt, subsec) = get_sys_dt_and_subsec();
    // setting RTC times and rechecking sys time seems to take ~3800 micros with my equipment
    println!("set time {} at {} ({} µs) ", next_dt, sys_dt, subsec);
    if subsec > 100 { continue 'session }
    let sys_timestamp_start: u32 = sys_dt.timestamp().try_into().unwrap();

    // check the drift over and over again
    loop {
      let out1 = rtc1.get_unix_time().unwrap();
      let out2 = rtc2.get_unix_time().unwrap();
      let (sys_timestamp, subsec_micros) = get_sys_timestamp_and_micros();

      // adjust the check time so that we're checking as fast as we
      // can just after one minute has elapsed
      let fall_back =
        Duration::from_micros(((150*subsec_micros)/100) as u64);
      let wait_duration =
        Duration::from_secs(20).checked_sub(fall_back).unwrap();

      if sys_timestamp != out1 || sys_timestamp != out2 {
        let elapsed = sys_timestamp - sys_timestamp_start;
        println!("sys: {} rtc1: {} rtc2: {} µs: {}",
                 sys_timestamp,
                 out1,
                 out2,
                 subsec_micros);
        println!("time to drift: {}", sys_timestamp - sys_timestamp_start);
        if elapsed < 20 {
          continue 'session;
        }
        break;
      }
      else {
        println!("micros: {}", subsec_micros);
      }
      sleep(wait_duration);
    }
  }

}
