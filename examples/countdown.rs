extern crate rv3028c7_rtc;

use linux_embedded_hal::I2cdev;
use chrono::{Duration, Utc};
use rv3028c7_rtc::{RV3028, TimerClockFreq};
use rtcc::DateTimeAccess;

use embedded_hal::blocking::i2c::{Write, Read, WriteRead};


fn ticks_and_rate_for_duration(dur: &Duration) -> (u16, TimerClockFreq)
{
  const MAX_TICKS: u16 = 0x0FFF;
  const MAX_COUNT_VAL:i64 = MAX_TICKS as i64;
  const MILLIS_FACTOR:i64 = 15; // 15.625 ms period
  const MICROS_FACTOR:i64 = 244; // 244.14 μs period
  const MAX_MILLIS_COUNT:i64 = MAX_COUNT_VAL * MILLIS_FACTOR;
  const MAX_MICROS_COUNT:i64 = MAX_COUNT_VAL * MICROS_FACTOR;

  let whole_minutes = dur.num_minutes();
  let whole_seconds = dur.num_seconds();
  let whole_milliseconds = dur.num_milliseconds();
  let whole_microseconds = dur.num_microseconds().unwrap();

  return if whole_minutes > MAX_COUNT_VAL {
    (MAX_TICKS, TimerClockFreq::HertzSixtieth)
  } else if whole_seconds >= MAX_COUNT_VAL {
    // use minutes
    let minutes = (whole_minutes & MAX_COUNT_VAL) as u16;
    (minutes, TimerClockFreq::HertzSixtieth)
  } else if whole_milliseconds >= MAX_MILLIS_COUNT {
    // use seconds
    let seconds = (whole_seconds & MAX_COUNT_VAL) as u16;
    (seconds, TimerClockFreq::Hertz1)
  } else if whole_microseconds >= MAX_MICROS_COUNT {
    // use milliseconds
    let millis = whole_milliseconds % MAX_MILLIS_COUNT;
    let ticks = (millis / MILLIS_FACTOR) as u16;
    (ticks, TimerClockFreq::Hertz64)
  } else {
    // use microseconds
    let micros = whole_microseconds % MAX_MICROS_COUNT;
    let ticks = (micros / MICROS_FACTOR) as u16;
    (ticks, TimerClockFreq::Hertz4096)
  }

}

fn dump_actual_vs_expected(
  ticks: u16, freq: TimerClockFreq, expected: &Duration, actual: &Duration) {
  match freq {
    TimerClockFreq::Hertz4096 => {
      println!("{} ticks at 4096 Hz finished in {} micros (expected {})",
               ticks, actual.num_microseconds().unwrap(), expected.num_microseconds().unwrap()
      );
    },
    TimerClockFreq::Hertz64 => {
      println!("{} ticks at 64 Hz finished in {} millis (expected {})",
               ticks, actual.num_milliseconds(), expected.num_milliseconds()
      );
    },
    TimerClockFreq::Hertz1 => {
      println!("{} ticks at 1 Hz finished in {:?} seconds (expected {})",
               ticks, actual.num_seconds(), expected.num_seconds()
      );
    },
    TimerClockFreq::HertzSixtieth => {
      println!("{} ticks at 1/60 Hz finished in {:?} minutes (expected {})",
               ticks, actual.num_minutes(), expected.num_minutes());
    },
  }
}

fn calc_countdown_period(ticks: u16, freq: TimerClockFreq) -> Duration {
  let clean_ticks: i64 = ticks as i64;
  match freq {
    TimerClockFreq::Hertz4096 => {
      let micros = (clean_ticks * 1_000_000) / 4096;
        Duration::microseconds( micros)
    },
    TimerClockFreq::Hertz64 => {
      let millis = (clean_ticks * 1_000) / 64;
      Duration::milliseconds(millis)
    },
    TimerClockFreq::Hertz1 => {
      Duration::seconds(clean_ticks)
    },
    TimerClockFreq::HertzSixtieth => {
      Duration::seconds(clean_ticks*60)
    },
  }
}

fn test_one_shot_duration<I2C,E>(
  rtc: &mut RV3028<I2C>, dur: &Duration)  -> Result<Duration, E>
  where
    I2C: Write<Error = E> + Read<Error = E> + WriteRead<Error = E>,
    E: std::fmt::Debug
{
  rtc.clear_all_int_out_bits()?;
  rtc.toggle_countdown_timer(false)?;
  let (ticks, freq) = ticks_and_rate_for_duration(dur);
  let expected = calc_countdown_period(ticks, freq);
  println!("ticks {} freq {} expected {} source {} \r\n", ticks, freq as u8, expected, dur);

  rtc.config_countdown_timer(ticks, freq, false)?;

  let expected_sleep = expected.to_std().unwrap();
  let start_time = Utc::now().naive_utc();

  rtc.toggle_countdown_timer(true)?;
  std::thread::sleep(expected_sleep);

  let actual = loop {
    if let Ok(true) = rtc.check_and_clear_countdown() {
      let end_time = Utc::now().naive_utc();
      let delta = (end_time - start_time) - Duration::microseconds(1956) ;
      break delta;
    }
  };

  dump_actual_vs_expected(ticks, freq, &dur, &actual);
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

  // test_one_shot_duration(&mut rtc, &Duration::seconds(300)).unwrap();
  // test_one_shot_duration(&mut rtc, &Duration::seconds(3000)).unwrap();
  //
  // test_one_shot_duration(&mut rtc, &Duration::seconds(5000)).unwrap();
  // test_one_shot_duration(&mut rtc, &Duration::minutes(100)).unwrap();


}