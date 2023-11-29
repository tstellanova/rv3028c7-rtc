extern crate rv3028c7_rtc;

use std::ops::{Add};
use linux_embedded_hal::I2cdev;
use chrono::{Datelike, Duration, NaiveDateTime, Timelike, Utc, Weekday};
use rv3028c7_rtc::{RV3028, TimerClockFreq};
use rtcc::DateTimeAccess;

use embedded_hal::blocking::i2c::{Write, Read, WriteRead};
// use direct linux gpio access using cdev rather than via constrained embedded_hal methods
use gpiocdev::{ line::{EdgeDetection} };

const MUX_I2C_ADDRESS: u8 = 0x70;
const MUX_CHAN_FIRST:u8 = 0b0000_0001 ; //channel 0, LSB
// const MUX_CHAN_SECOND:u8 = 0b1000_0000 ; // channel 7, MSB


fn test_one_shot_duration<I2C,E>(
  rtc: &mut RV3028<I2C>, freq: TimerClockFreq, ticks: u16)  -> Result<Duration, E>
  where
    I2C: Write<Error = E> + Read<Error = E> + WriteRead<Error = E>,
    E: std::fmt::Debug
{
  rtc.toggle_countdown_timer(false, false)?;
  rtc.configure_countdown_timer(ticks, freq)?;
  let start_time = Utc::now().naive_utc();
  rtc.toggle_countdown_timer(true, false)?;
  let delta = loop {
    if let Ok(true) = rtc.check_and_clear_countdown() {
      let end_time = Utc::now().naive_utc();
      let delta = end_time - start_time;
      break delta;
    }
  };
  match freq {
    TimerClockFreq::Hertz4096 => {
      println!("{} ticks at 4096 Hz finished in {:?} micros",ticks,
        delta.num_microseconds().unwrap());
    },
    TimerClockFreq::Hertz64 => {
      println!("{} ticks at 64 Hz finished in {:?} millis", ticks,
        delta.num_milliseconds());
    },
    TimerClockFreq::Hertz1 => {
      println!("{} ticks at 64 Hz finished in {:?} seconds", ticks,
        delta.num_seconds());
    },
    TimerClockFreq::HertzSixtieth => {
      println!("{} ticks at 1/60 Hz finished in {:?} minuts", ticks,
      delta.num_minutes());
    },
  }

  Ok(delta)
}

fn main() {
  // Initialize the I2C device
  let i2c = I2cdev::new("/dev/i2c-1").expect("Failed to open I2C device");
  // Create a new instance of the RV3028 driver
  // let mut rtc = RV3028::new(i2c);
  let mut rtc =
    RV3028::new_with_mux(i2c, MUX_I2C_ADDRESS, MUX_CHAN_FIRST);

  // use the set_datetime method to ensure all the timekeeping registers on
  // the rtc are aligned to the same values
  let dt_sys = Utc::now().naive_utc();
  rtc.set_datetime(&dt_sys).unwrap();
  let dt_rtc = rtc.datetime().unwrap();
  println!("start sys {} rtc {}  ", dt_sys, dt_rtc);

  // rtc.configure_countdown_timer(3, TimerClockFreq::Hertz4096);
  // let start_time = Utc::now().naive_utc();
  // rtc.toggle_countdown_timer(true, false).unwrap();
  //
  // loop {
  //   if let Ok(true) = rtc.check_and_clear_countdown() {
  //     let end_time = Utc::now().naive_utc();
  //     let delta = end_time - start_time;
  //     println!("countdown finished in {:?} micros", delta.num_microseconds().unwrap());
  //     break;
  //   }
  // }

  test_one_shot_duration(&mut rtc, TimerClockFreq::Hertz4096, 1).unwrap();
  test_one_shot_duration(&mut rtc, TimerClockFreq::Hertz4096, 10).unwrap();
  test_one_shot_duration(&mut rtc, TimerClockFreq::Hertz64, 1).unwrap();
  test_one_shot_duration(&mut rtc, TimerClockFreq::Hertz64, 10).unwrap();
  test_one_shot_duration(&mut rtc, TimerClockFreq::Hertz1, 1).unwrap();
  test_one_shot_duration(&mut rtc, TimerClockFreq::Hertz1, 10).unwrap();
  test_one_shot_duration(&mut rtc, TimerClockFreq::HertzSixtieth, 1).unwrap();


}