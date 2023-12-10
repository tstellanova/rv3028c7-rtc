
extern crate rv3028c7_rtc;

use anyhow::Result;
use linux_embedded_hal::I2cdev;
use chrono::{Utc};
use rv3028c7_rtc::{
  RV3028,
  DateTimeAccess,
  EventTimeStampLogger,
  NaiveDateTime
};
use embedded_hal::blocking::i2c::{Write, Read, WriteRead};

/// This example is meant to probe a "Long Clock" that has
/// already been setup (see the `long_clock_setup.rs` example)
/// for running undisturbed for many years, switching to backup
/// power as needed to maintain its state.
///
/// This example takes a two-stage approach:
/// - First it reads back values of interest in read-only mode.
/// - Then it provides the writer (write protection) password and
/// resets counters and interrupts ONLY. It must not overwrite any
/// time counters or other configuration
///
/// This example assumes:
/// - The host connecting to the RTC is running a linux environment (such as Raspberry Pi)
/// - The RTC is attached to i2c1.
/// - The RTC has previously been configured with write-protection
/// with an app such as `long_clock_setup`.
///
/// The following was tested by enabling i2c-1 on a Raspberry Pi 3+
/// using `sudo raspi-config`
/// and connecting the SDA, SCL, GND, and 3.3V pins from RPi to the RTC
///

const BOGUS_PASSWORD: [u8; 4] = [0xDE,0xED,0xFA,0xDE];
const LONG_CLOCK_PASSWORD: [u8; 4] = [0xFE,0xED,0xD0,0xBB];


fn get_sys_datetime_timestamp() -> (NaiveDateTime, u32) {
  let now = Utc::now();
  let now_timestamp = now.timestamp();
  (now.naive_utc(), now_timestamp.try_into().unwrap() )
}

/// ==== Read-only (safe) section ====
/// Read the RTC configuration, status,
/// and any flags that have triggered during operation.
/// This method does not write to any write-protected registers.
///
fn read_rtc_status_and_config<I2C,E>(rtc: &mut RV3028<I2C>) -> Result<(),E>
  where
    I2C: Write<Error = E> + Read<Error = E> + WriteRead<Error = E>,
    E: std::fmt::Debug
{
  let wp_enabled = rtc.check_write_protect_enabled()?;
  println!("write protect enabled: {}", wp_enabled);

  let uram_val = rtc.get_user_ram()?;
  let user_val: u16 = u16::from_le_bytes(uram_val);
  println!("user RAM value: {}", user_val);

  // Pull the current system time and compare RTC time to that
  let (sys_dt, sys_unix_timestamp) = get_sys_datetime_timestamp();

  let rtc_unix_timestamp = rtc.get_unix_time()?;
  println!("sys unix {}", sys_unix_timestamp);
  println!("rtc unix {}", rtc_unix_timestamp);

  let rtc_dt = rtc.datetime()?;
  println!("sys dt: {}", sys_dt);
  println!("rtc dt: {}", rtc_dt);

  // check alarm flag set
  let af_set = rtc.check_alarm_flag()?;
  println!("Alarm flag set? {}", af_set);
  // read alarm setting
  let (alarm_dt, out_weekday, out_match_day, out_match_hour, out_match_minute) =
    rtc.get_alarm_datetime_wday_matches()?;
  println!("alarm dt: {} weekday: {:?} matches dhm: {} {} {}",
           alarm_dt, out_weekday, out_match_day, out_match_hour, out_match_minute);

  // Read status of Vbackup switchovers
  let bsf = rtc.check_backup_event_flag()?;
  println!("Vbackup switchover flag set? {}", bsf);
  if bsf {
    let (evt_count, backup_evt_odt) = rtc.get_event_count_and_datetime()?;
    if let Some(backup_evt) = backup_evt_odt {
      println!("num Vbackup switchovers: {} first: {}", evt_count, backup_evt);
    }
  }

  Ok(())
}

///
/// === DANGEROUS SECTION: includes write-modifications ===
///
fn cleanup_status_and_logs<I2C,E>(rtc: &mut RV3028<I2C>) -> Result<(),E>
  where
    I2C: Write<Error = E> + Read<Error = E> + WriteRead<Error = E>,
    E: std::fmt::Debug
{

  let wp_enabled = rtc.check_write_protect_enabled()?;
  if !wp_enabled {
    println!("write protect should be enabled -- exiting");
    return Ok(())
  }

  // Enter the known valid password to unlock write protection
  rtc.enter_user_password(&LONG_CLOCK_PASSWORD)?;
  // disable write protection temporarily
  rtc.toggle_write_protect_enabled(false)?;

  // read back the current write-protection password stored in EEPROM
  // this is only readable if wp is unlocked
  let ur_wp_pass = rtc.get_write_protect_password()?;
  println!("wp password in eeprom is: {:?}", ur_wp_pass);

  // clear all status flags that may have triggered
  rtc.clear_all_status_flags()?;

  // clear out the event timestamp logs
  rtc.reset_timestamp_log()?;


  let uram_val = rtc.get_user_ram()?;
  let user_val: u16 = u16::from_le_bytes(uram_val);
  let new_user_val = user_val + 1;
  let new_uram_val = new_user_val.to_le_bytes();
  rtc.set_user_ram(&new_uram_val)?;
  println!("user RAM value: old {} new {} ", user_val, new_user_val);

  // done with dangerous changes -- lock the write-protected areas
  let wp_reenabled = rtc.toggle_write_protect_enabled(true)?;
  println!("wp_reenabled: {}", wp_reenabled);

  println!("Clearing user password");
  rtc.enter_user_password(&BOGUS_PASSWORD)?;

  println!("Write protection enabled: {}", rtc.check_write_protect_enabled()?);


  Ok(())

}

fn main() -> Result<()> {
  // Initialize the I2C device
  let i2c = I2cdev::new("/dev/i2c-1").expect("Failed to open I2C device");

  // Create a new instance of the RV3028 driver
  let mut rtc = RV3028::new(i2c);

  // === Safe section: only read operations ===
  read_rtc_status_and_config(&mut rtc)?;

  // === Dangerous section: some write/update operations ===
  cleanup_status_and_logs(&mut rtc)?;

  Ok(())

}
