extern crate rv3028c7_rtc;

use core::ops::{Add};
use linux_embedded_hal::I2cdev;
use chrono::{Datelike, NaiveDateTime, Timelike, Utc, Weekday};
use rv3028c7_rtc::{RV3028};
use std::time::Duration;
use std::thread::sleep;

use rtcc::DateTimeAccess;

use embedded_hal::blocking::i2c::{Write, Read, WriteRead};

fn get_sys_timestamp() -> (NaiveDateTime, u32) {
  let now = Utc::now();
  let now_timestamp = now.timestamp();
  (now.naive_utc(), now_timestamp.try_into().unwrap() )
}

const MUX_I2C_ADDRESS: u8 = 0x70;
const MUX_CHAN_FIRST:u8 = 0b0000_0001 ; //channel 0, LSB
const MUX_CHAN_SECOND:u8 = 0b1000_0000 ; // channel 7, MSB

fn main() {
  // This is a specific configuration for Raspberry Pi -- YMMV

  // Initialize the I2C device
  let i2c = I2cdev::new("/dev/i2c-1").expect("Failed to open I2C device");
  let i2c_bus = shared_bus::BusManagerSimple::new(i2c);

  // Create a new instance of the RV3028 driver
  // let mut rtc = RV3028::new(i2c);
  let mut rtc1 = RV3028::new_with_mux(i2c_bus.acquire_i2c(), MUX_I2C_ADDRESS, MUX_CHAN_FIRST);
  let mut rtc2 = RV3028::new_with_mux(i2c_bus.acquire_i2c(), MUX_I2C_ADDRESS, MUX_CHAN_SECOND);

  let (sys_datetime, sys_unix_timestamp) = get_sys_timestamp();
  // use the set_datetime method to ensure all the timekeeping registers on
  // the rtc are aligned to the same values
  rtc1.set_datetime(&sys_datetime).unwrap();
  rtc2.set_datetime(&sys_datetime).unwrap();
  let rtc_unix_time = rtc2.get_unix_time().unwrap();
  // verify that the individual year, month, day registers are set correctly
  let (year, month, day) = rtc2.get_ymd().unwrap();
  println!("start sys {} rtc {} ymd {} {} {} ", sys_unix_timestamp, rtc_unix_time, year, month, day);

  // disable alarm interrupts to begin with
  // rtc2.toggle_alarm_int_enable(false).unwrap();
  rtc1.clear_all_int_out_bits().unwrap();
  rtc2.clear_all_int_out_bits().unwrap();
  rtc1.toggle_clock_output(false).unwrap();
  rtc2.toggle_clock_output(false).unwrap();

  rtc1.check_and_clear_alarm().unwrap();
  rtc2.check_and_clear_alarm().unwrap();

  println!("INT disabled, pausing...");
  sleep(Duration::from_secs(3));

  let init_dt = rtc2.datetime().unwrap();
  let alarm_dt = init_dt.add(Duration::from_secs(60));
  println!("init_dt :  {}", init_dt);
  println!("alarm_dt: {}", alarm_dt);
  rtc1.set_alarm(&alarm_dt, None, false, false, true).unwrap();
  rtc2.set_alarm(&alarm_dt, None, false, false, true).unwrap();

  let (next_alarm_dt, _, _, _, _) =
    rtc1.get_alarm_datetime_wday_matches().unwrap();
  rtc1.toggle_alarm_int_enable(true).unwrap();
  println!("rtc1 alarm at {} ", next_alarm_dt);

  let (next_alarm_dt, _, _, _, _) =
    rtc2.get_alarm_datetime_wday_matches().unwrap();
  rtc2.toggle_alarm_int_enable(true).unwrap();
  println!("rtc2 alarm at {} ", next_alarm_dt);

}
