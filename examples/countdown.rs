extern crate rv3028c7_rtc;

use std::ops::Add;
use linux_embedded_hal::I2cdev;
use chrono::{Duration, Utc};
use rv3028c7_rtc::{RV3028};
use rtcc::DateTimeAccess;

use embedded_hal::blocking::i2c::{Write, Read, WriteRead};



fn test_one_shot_duration<I2C,E>(rtc: &mut RV3028<I2C>, duration: &Duration) -> Result<Duration, E>
  where
    I2C: Write<Error = E> + Read<Error = E> + WriteRead<Error = E>,
    E: std::fmt::Debug
{
  rtc.clear_all_int_out_bits()?;
  rtc.toggle_countdown_timer(false)?;
  rtc.check_and_clear_countdown()?;
  let estimated_duration = rtc.config_countdown_timer(duration, false, false)?;
  // account for supposed maximum uncertainty in RTC durations
  let expected_sleep =
    if estimated_duration.le(&Duration::microseconds(4096 * 245)){
      estimated_duration.add(Duration::microseconds(245) )
    }
    else {
      estimated_duration.add(Duration::milliseconds(16) )
    };

  println!("> oneshot {} sleep {} ", duration, expected_sleep);

  let start_time = Utc::now().naive_utc();
  rtc.toggle_countdown_timer(true)?;
  if estimated_duration.lt(&Duration::seconds(1)) {
    std::thread::sleep(expected_sleep.to_std().unwrap());
  }
  else {
    spin_sleep::sleep(expected_sleep.to_std().unwrap());
  }

  let actual = loop {
    let remain = rtc.get_countdown_value()?;
    if 0 == remain {
      let triggered = rtc.check_and_clear_countdown()?;
      if !triggered { println!("Counter zero but PERIODIC_TIMER_FLAG untriggered!!")}
      let end_time = Utc::now().naive_utc();
      let delta = end_time - start_time;
      break delta;
    }
    else {
      // println!("remain: {}", remain);
      // 15.625 ms uncertainty
      std::thread::sleep(Duration::milliseconds(1).to_std().unwrap());
    }
  };

  let delta_total = actual - estimated_duration ;
  println!("< actual {} expected {} requested {} delta {}", actual, estimated_duration, duration, delta_total);
  Ok(actual)
}

fn test_periodic_duration<I2C,E>(rtc: &mut RV3028<I2C>, duration: &Duration) -> Result<(), E>
  where
    I2C: Write<Error = E> + Read<Error = E> + WriteRead<Error = E>,
    E: std::fmt::Debug
{
  rtc.clear_all_int_out_bits()?;
  rtc.toggle_countdown_timer(false)?;
  rtc.check_and_clear_countdown()?;

  let estimated_duration = rtc.config_countdown_timer(duration, true, false)?;
  // we don't adjust for the first duration uncertainty, assuming it will average out
  let expected_sleep =    estimated_duration;
  println!("> periodic {} sleep {} ", duration, expected_sleep);

  // place to store the sum of all measured countdown durations
  let mut sum_actual = Duration::zero();
  const NUM_ITERATIONS: usize = 10;

  // start the countdown repeating
  let mut start_time = Utc::now().naive_utc();
  rtc.toggle_countdown_timer(true)?;
  for _i in 0..NUM_ITERATIONS {
    if estimated_duration.lt(&Duration::seconds(1)) {
      std::thread::sleep(expected_sleep.to_std().unwrap());
    }
    else {
      spin_sleep::sleep(expected_sleep.to_std().unwrap());
    }

    let actual = loop {
      let triggered = rtc.check_and_clear_countdown()?;
      if triggered {
        let end_time = Utc::now().naive_utc();
        let delta = end_time - start_time;
        start_time = end_time; //reset timer for next event
        break delta;
      }
    };

    // println!("actual {} expected {}", actual, duration);
    sum_actual = sum_actual + actual;
  }

  let avg_actual = sum_actual / NUM_ITERATIONS as i32;
  let delta_total = avg_actual - estimated_duration ;
  println!("< avg_actual {} expected {} requested {} delta: {}",
           avg_actual, estimated_duration, duration, delta_total);

  Ok(())
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


  // Note that all RTC durations have a minimum uncertainty in the
  // first period duration: this is between 244 microseconds and 15 milliseconds.
  // Add to that some variation in how the host (linux) platform
  // polls for RTC countdown timer completion.

  println!("\r\n==== ONE SHOTS ====");
  test_one_shot_duration(&mut rtc, &Duration::microseconds(488)).unwrap();
  test_one_shot_duration(&mut rtc, &Duration::milliseconds(15 * (1000/15))).unwrap();
  test_one_shot_duration(&mut rtc, &Duration::seconds(1)).unwrap();
  test_one_shot_duration(&mut rtc, &Duration::seconds(2)).unwrap();
  test_one_shot_duration(&mut rtc, &Duration::seconds(3)).unwrap();
  // test_one_shot_duration(&mut rtc, &Duration::minutes(1)).unwrap();

  println!("\r\n==== PERIODICS ====");
  test_periodic_duration(&mut rtc, &Duration::microseconds(488)).unwrap();
  test_periodic_duration(&mut rtc, &Duration::milliseconds(15 * (1000/15))).unwrap();
  test_one_shot_duration(&mut rtc, &Duration::seconds(1)).unwrap();
  test_one_shot_duration(&mut rtc, &Duration::seconds(2)).unwrap();
  test_periodic_duration(&mut rtc, &Duration::seconds(3)).unwrap();
  // test_periodic_duration(&mut rtc, &Duration::minutes(1)).unwrap();



}