extern crate rv3028c7_rtc;

use linux_embedded_hal::I2cdev;
use chrono::{Duration, Utc};
use rv3028c7_rtc::{RV3028};
use rtcc::DateTimeAccess;

use embedded_hal::blocking::i2c::{Write, Read, WriteRead};


fn test_one_shot_duration<I2C,E>(rtc: &mut RV3028<I2C>, dur: &Duration)  -> Result<Duration, E>
  where
    I2C: Write<Error = E> + Read<Error = E> + WriteRead<Error = E>,
    E: std::fmt::Debug
{
  rtc.clear_all_int_out_bits()?;
  rtc.toggle_countdown_timer(false)?;
  rtc.setup_countdown_timer(dur, false)?;

  let expected_sleep = dur.to_std().unwrap();
  let start_time = Utc::now().naive_utc();

  println!("sleep {} ", dur);
  rtc.toggle_countdown_timer(true)?;
  spin_sleep::sleep(expected_sleep);
  // std::thread::sleep(expected_sleep);

  let mut last_remaining = 555;
  let actual = loop {
    if let Ok(true) = rtc.check_and_clear_countdown() {
      let end_time = Utc::now().naive_utc();
      let delta =  end_time - start_time ;
      break delta;
    }
    else {
      let val =     rtc.get_countdown_value()?;
      println!("remain: {}", val);
      if 0 == val {
        if 0 == last_remaining {
          let end_time = Utc::now().naive_utc();
          let delta = end_time - start_time;
          break delta;
        }
        last_remaining = 0;
      }
    }
  };

  println!("actual {} expected {}", actual, dur);
  Ok(actual)
}

fn main() {
  // Initialize the I2C device
  let i2c = I2cdev::new("/dev/i2c-1").expect("Failed to open I2C device");
  // Create a new instance of the RV3028 driver
  let mut rtc = RV3028::new(i2c);

  // use the set_datetime method to ensure all the timekeeping registers on
  // the rtc are aligned to the same values
  let dt_sys = Utc::now().naive_utc();
  rtc.set_datetime(&dt_sys).unwrap();
  let dt_rtc = rtc.datetime().unwrap();
  println!("sys {}\r\nrtc {}\r\n", dt_sys, dt_rtc);

  test_one_shot_duration(&mut rtc, &Duration::microseconds(244)).unwrap();
  test_one_shot_duration(&mut rtc, &Duration::microseconds(488)).unwrap();
  test_one_shot_duration(&mut rtc, &Duration::microseconds(976)).unwrap();
  test_one_shot_duration(&mut rtc, &Duration::microseconds(976)).unwrap();
  test_one_shot_duration(&mut rtc, &Duration::microseconds(999180)).unwrap();

  test_one_shot_duration(&mut rtc, &Duration::milliseconds(15)).unwrap();
  test_one_shot_duration(&mut rtc, &Duration::milliseconds(30)).unwrap();
  test_one_shot_duration(&mut rtc, &Duration::milliseconds(45)).unwrap();

  test_one_shot_duration(&mut rtc, &Duration::seconds(30)).unwrap();
  test_one_shot_duration(&mut rtc, &Duration::minutes(2)).unwrap();
  test_one_shot_duration(&mut rtc, &Duration::seconds(300)).unwrap();



}