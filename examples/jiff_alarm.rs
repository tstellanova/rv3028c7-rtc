use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use linux_embedded_hal::I2cdev;
use rv3028c7_rtc::jiff::{civil, tz::Offset, Timestamp, ToSpan};
use rv3028c7_rtc::{JiffDateTimeAccess, RV3028};

/// Example using the `jiff` date/time types instead of `chrono`:
/// set the RTC from the host's system clock, then set an alarm one minute ahead
/// and wait for it to trigger.
///
/// Run with:
/// ```text
/// cargo run --example jiff_alarm --no-default-features --features jiff
/// ```
///
/// Assumes a linux environment (such as Raspberry Pi 3+)
/// with the RV3028 attached directly to i2c1:
/// enable i2c-1 using `sudo raspi-config`,
/// and connect the SDA, SCL, GND, and 3.3V pins from the RPi to the RTC.

fn get_sys_datetime() -> civil::DateTime {
  let unix_seconds = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
  Offset::UTC.to_datetime(Timestamp::from_second(unix_seconds as i64).unwrap())
}

fn main() {
  // Initialize the I2C device
  let i2c = I2cdev::new("/dev/i2c-1").expect("Failed to open I2C device");
  // Create a new instance of the RV3028 driver
  let mut rtc = RV3028::new(i2c);

  // use the set_datetime method to ensure all the timekeeping registers on
  // the rtc are aligned to the same values
  let sys_dt = get_sys_datetime();
  rtc.set_datetime(&sys_dt).unwrap();
  println!("sys {}\r\nrtc {}", sys_dt, rtc.datetime().unwrap());

  // disable alarm interrupts and clear any earlier alarm
  rtc.toggle_alarm_int_enable(false).unwrap();
  rtc.check_and_clear_alarm().unwrap();

  // set a date alarm one minute from now, matching only the hour and minute
  let alarm_dt = rtc.datetime().unwrap() + 1.minute();
  rtc.set_alarm(&alarm_dt, None, false, true, true).unwrap();

  let (dt, _weekday, match_day, match_hour, match_minute) =
    rtc.get_alarm_datetime_wday_matches().unwrap();
  println!("alarm at {:02}:{:02} (match day: {} hour: {} minute: {})",
           dt.hour(), dt.minute(), match_day, match_hour, match_minute);

  // The alarm triggers when the minute starts, so this waits up to two minutes
  for _ in 0..125 {
    if rtc.check_and_clear_alarm().unwrap() {
      println!("rtc {} alarm triggered", rtc.datetime().unwrap());
      return;
    }
    sleep(Duration::from_secs(1));
  }
  println!("alarm did not trigger");
}
