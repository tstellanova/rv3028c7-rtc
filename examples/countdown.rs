use linux_embedded_hal::I2cdev;
use chrono::Utc;
use std::time::{Duration, Instant};
use rv3028c7_rtc::{RV3028};
use rtcc::DateTimeAccess;

use embedded_hal::i2c::I2c;

// Signed difference in microseconds, since `Duration` itself cannot be negative
fn delta_micros(actual: Duration, expected: Duration) -> i128 {
  actual.as_micros() as i128 - expected.as_micros() as i128
}

fn test_one_shot_duration<I2C>(rtc: &mut RV3028<I2C>, duration: &Duration) -> Result<Duration, rv3028c7_rtc::Error<I2C::Error>>
  where
    I2C: I2c,
    I2C::Error: std::fmt::Debug,
{
  rtc.clear_all_int_out_bits()?;
  rtc.toggle_countdown_timer(false)?;
  rtc.check_and_clear_countdown()?;
  let estimated_duration = rtc.config_countdown_timer(duration, false, false)?;
  // account for supposed maximum uncertainty in RTC durations
  let expected_sleep =
    if estimated_duration <= Duration::from_micros(4096 * 245){
      estimated_duration + Duration::from_micros(245)
    }
    else {
      estimated_duration + Duration::from_millis(16)
    };

  println!("> oneshot {:?} sleep {:?} ", duration, expected_sleep);

  let start_time = Instant::now();
  rtc.toggle_countdown_timer(true)?;
  if estimated_duration < Duration::from_secs(1) {
    std::thread::sleep(expected_sleep);
  }
  else {
    spin_sleep::sleep(expected_sleep);
  }

  let actual = loop {
    let remain = rtc.get_countdown_value()?;
    if 0 == remain {
      let triggered = rtc.check_and_clear_countdown()?;
      if !triggered { println!("Counter zero but PERIODIC_TIMER_FLAG untriggered!!")}
      let end_time = Instant::now();
      let delta = end_time - start_time;
      break delta;
    }
    else {
      // println!("remain: {}", remain);
      // 15.625 ms uncertainty
      std::thread::sleep(Duration::from_millis(1));
    }
  };

  let delta_total = delta_micros(actual, estimated_duration);
  println!("< actual {:?} expected {:?} requested {:?} delta {} µs", actual, estimated_duration, duration, delta_total);
  Ok(actual)
}

fn test_periodic_duration<I2C>(rtc: &mut RV3028<I2C>, duration: &Duration) -> Result<(), rv3028c7_rtc::Error<I2C::Error>>
  where
    I2C: I2c,
    I2C::Error: std::fmt::Debug,
{
  rtc.clear_all_int_out_bits()?;
  rtc.toggle_countdown_timer(false)?;
  rtc.check_and_clear_countdown()?;

  let estimated_duration = rtc.config_countdown_timer(duration, true, false)?;
  // we don't adjust for the first duration uncertainty, assuming it will average out
  let expected_sleep =    estimated_duration;
  println!("> periodic {:?} sleep {:?} ", duration, expected_sleep);

  // place to store the sum of all measured countdown durations
  let mut sum_actual = Duration::ZERO;
  const NUM_ITERATIONS: usize = 10;

  // start the countdown repeating
  let mut start_time = Instant::now();
  rtc.toggle_countdown_timer(true)?;
  for _i in 0..NUM_ITERATIONS {
    if estimated_duration < Duration::from_secs(1) {
      std::thread::sleep(expected_sleep);
    }
    else {
      spin_sleep::sleep(expected_sleep);
    }

    let actual = loop {
      let triggered = rtc.check_and_clear_countdown()?;
      if triggered {
        let end_time = Instant::now();
        let delta = end_time - start_time;
        start_time = end_time; //reset timer for next event
        break delta;
      }
    };

    // println!("actual {} expected {}", actual, duration);
    sum_actual = sum_actual + actual;
  }

  let avg_actual = sum_actual / NUM_ITERATIONS as u32;
  let delta_total = delta_micros(avg_actual, estimated_duration);
  println!("< avg_actual {:?} expected {:?} requested {:?} delta: {} µs",
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
  test_one_shot_duration(&mut rtc, &Duration::from_micros(488)).unwrap();
  test_one_shot_duration(&mut rtc, &Duration::from_millis(15 * (1000/15))).unwrap();
  test_one_shot_duration(&mut rtc, &Duration::from_secs(1)).unwrap();
  test_one_shot_duration(&mut rtc, &Duration::from_secs(2)).unwrap();
  test_one_shot_duration(&mut rtc, &Duration::from_secs(3)).unwrap();
  // test_one_shot_duration(&mut rtc, &Duration::minutes(1)).unwrap();

  println!("\r\n==== PERIODICS ====");
  test_periodic_duration(&mut rtc, &Duration::from_micros(488)).unwrap();
  test_periodic_duration(&mut rtc, &Duration::from_millis(15 * (1000/15))).unwrap();
  test_one_shot_duration(&mut rtc, &Duration::from_secs(1)).unwrap();
  test_one_shot_duration(&mut rtc, &Duration::from_secs(2)).unwrap();
  test_periodic_duration(&mut rtc, &Duration::from_secs(3)).unwrap();
  // test_periodic_duration(&mut rtc, &Duration::minutes(1)).unwrap();



}