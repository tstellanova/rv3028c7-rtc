extern crate rv3028c7_rtc;

use anyhow::Result;
use linux_embedded_hal::I2cdev;
use chrono::{Utc};
use rv3028c7_rtc::{RV3028, DateTimeAccess, Duration, Weekday, NaiveDate, NaiveDateTime, NaiveTime, Datelike, Timelike, EventTimeStampLogger, TS_EVENT_SOURCE_BSF, ClockoutRate, TrickleChargeCurrentLimiter};
use std::thread::sleep;
use embedded_hal::blocking::i2c::{Write, Read, WriteRead};

/// Example testing real RTC communications,
/// assuming linux environment (such as Raspberry Pi 3+)
/// with RV3028 attached to i2c1.
/// The following was tested by enabling i2c-1 on a Raspberry Pi 3+
/// using `sudo raspi-config`
/// and connecting the SDA, SCL, GND, and 3.3V pins from RPi to the RTC
///
/// This example is designed to configure the RTC as a "Long Clock",
/// meant to run for many years and switch to backup power as needed.
/// It writes passwords to the clock settings so that the clock cannot be
/// unintentionally overwritten.

fn get_sys_datetime_timestamp() -> (NaiveDateTime, u32) {
    let now = Utc::now();
    let now_timestamp = now.timestamp();
    (now.naive_utc(), now_timestamp.try_into().unwrap() )
}

fn check_passwords_match<I2C,E>(rtc: &mut RV3028<I2C>) -> Result<(),E>
    where
      I2C: Write<Error = E> + Read<Error = E> + WriteRead<Error = E>,
      E: std::fmt::Debug
{
    if rtc.check_write_protect_enabled()? {
        println!("Write protection enabled-- entering password");
        rtc.enter_user_password(&LONG_CLOCK_PASSWORD)?;
        // read back the current write-protection password stored in EEPROM
        // this is only readable if wp is unlocked
        let (_wp_enabled, ur_wp_pass) = rtc.get_write_protect_settings()?;
        println!("wp password in eeprom is: {:?} expect: {:?}", ur_wp_pass, LONG_CLOCK_PASSWORD);
        // if ur_wp_pass.ne(&LONG_CLOCK_PASSWORD) {
            // // set the user pass to be the same
            // rtc.enter_user_password(&ur_wp_pass)?;
            // println!(">>> changing to LONG_CLOCK_PASSWORD");
            // rtc.set_write_protect_password(&LONG_CLOCK_PASSWORD, false)?;
            // let (_wp_enabled, new_wp_pass) = rtc.get_write_protect_settings()?;
            // println!("new_wp_pass is: {:?} expect: {:?}", new_wp_pass, LONG_CLOCK_PASSWORD);
            // rtc.enter_user_password(&LONG_CLOCK_PASSWORD)?;
        // }
    }
    else {
        println!("Write protect disabled");
    }

    Ok(())
}

fn setup_write_protection<I2C,E>(rtc: &mut RV3028<I2C>) -> Result<(),E>
    where
      I2C: Write<Error = E> + Read<Error = E> + WriteRead<Error = E>,
      E: std::fmt::Debug
{
    rtc.restore_eeprom_settings()?;

    let (wp_enabled, wp_pass) = rtc.get_write_protect_settings()?;
    println!("eeprom wp_enabled: {} wp_pass: {:?}", wp_enabled, wp_pass);

    if wp_enabled {
        println!("EEPROM WP already enabled..skipping");
        return Ok(())
    }

    println!("setting write protecct password...");
    // the WP password is not set in eeprom -- set it now
    rtc.enter_user_password(&LONG_CLOCK_PASSWORD)?;
    rtc.set_write_protect_password(&LONG_CLOCK_PASSWORD, true)?;
    // rtc.toggle_write_protect_enabled(true);

    //verify the WP password is as expected
    check_passwords_match(rtc)?;

    Ok(())
}

fn verify_write_protection<I2C,E>(rtc: &mut RV3028<I2C>) -> Result<(),E>
    where
      I2C: Write<Error = E> + Read<Error = E> + WriteRead<Error = E>,
      E: std::fmt::Debug
{
    // We test the configuration with User Ram, which is innocuous
    let initial_uram = rtc.get_user_ram()?;
    println!("initial_uram {:?}", initial_uram);

    const INIT_USER_RAM: [u8; 2] = [55u8,77];
    const CHECK_USER_RAM: [u8; 2] = [44u8, 66];
    const FINAL_USER_RAM: [u8; 2] = [0u8;2];

    rtc.set_user_ram(&INIT_USER_RAM)?;
    let pre_protect_uram = rtc.get_user_ram()?;
    println!("pre_protect_uram {:?}", pre_protect_uram);
    assert_eq!(pre_protect_uram, INIT_USER_RAM);

    if !rtc.check_write_protect_enabled()? {
        // now ensure that the password is set and enabled in EEPROM
        rtc.set_write_protect_password(&LONG_CLOCK_PASSWORD, true)?;
    }

    //verify the WP password is as expected
    check_passwords_match(rtc)?;

    // this write to user RAM register should fail because password mismatches EEPROM
    rtc.enter_user_password(&BOGUS_PASSWORD)?;
    rtc.set_user_ram(&CHECK_USER_RAM)?;
    let post_protect_uram = rtc.get_user_ram()?;
    println!("post_protect_uram {:?}", post_protect_uram);
    assert_ne!(post_protect_uram, CHECK_USER_RAM);

    // this write to user RAM register should pass because password matches EEPROM
    rtc.enter_user_password(&LONG_CLOCK_PASSWORD)?;
    rtc.set_user_ram(&FINAL_USER_RAM)?;
    let post_unlocked_uram = rtc.get_user_ram()?;
    println!("post_unlocked_uram {:?}", post_unlocked_uram);
    assert_eq!(post_unlocked_uram, FINAL_USER_RAM);

    // leave the user password in RAM as bogus,
    // effectively locking the write-protect registers
    rtc.enter_user_password(&BOGUS_PASSWORD)?;
    Ok(())
}

// Set the alarm alarm set, and verify the value is set
fn verify_alarm_set<I2C,E>(rtc: &mut RV3028<I2C>, alarm_dt: &NaiveDateTime,
                           weekday: Option<Weekday>,
                           match_day: bool, match_hour: bool, match_minute: bool)
    where
      I2C: Write<Error = E> + Read<Error = E> + WriteRead<Error = E>,
      E: std::fmt::Debug
{
    rtc.set_alarm( &alarm_dt, weekday,
                   match_day, match_hour, match_minute).unwrap();

    let (dt, out_weekday, out_match_day, out_match_hour, out_match_minute) =
      rtc.get_alarm_datetime_wday_matches().unwrap();
    if let Some(inner_weekday) = weekday {
        println!("weekday alarm dt: {} wd: {} match_day: {} match_hour: {} match_minute; {}",
                 dt, inner_weekday, out_match_day, out_match_hour, out_match_minute
        );
    }
    else {
        println!("date alarm dt: {} match_day: {} match_hour: {} match_minute: {}",
                 dt, out_match_day, out_match_hour, out_match_minute
        );
    }

    assert!(!rtc.check_and_clear_alarm().unwrap());// alarm should not trigger

    assert_eq!(match_day, out_match_day);
    assert_eq!(match_hour, out_match_hour);
    assert_eq!(match_minute, out_match_minute);

    if weekday.is_some() {
        // weekday-based alarm
        assert_eq!(out_weekday, weekday);
    }
    else {
        // date-based alarm
        assert_eq!(dt.date().day(), alarm_dt.date().day());
    }

    assert_eq!(dt.time().hour(), alarm_dt.time().hour());
    assert_eq!(dt.time().minute(), alarm_dt.time().minute());

}

//[255, 254, 237, 208
// const ANCIENT_PASSWORD: [u8; 4] = [187, 254, 237, 208];
// const LESS_ANCIENT_PASS : [u8; 4] = [255, 254, 237, 208];
// const EVEN_LESS_ANCIENT_PASS: [u8;4] = [0, 254, 237, 208];

const LONG_CLOCK_PASSWORD: [u8; 4] = [0xFE,0xED,0xD0,0xBB];
const BOGUS_PASSWORD: [u8; 4] = [0x0A,0x0C,0x0E,0x0D];

fn main() -> Result<()> {

    // Initialize the I2C device
    let i2c = I2cdev::new("/dev/i2c-1").expect("Failed to open I2C device");

    // Create a new instance of the RV3028 driver
    let mut rtc = RV3028::new(i2c);

    setup_write_protection(&mut rtc)?;

    // preemptively set the user password in case it has already been written to EEPROM
    check_passwords_match(&mut rtc)?;

    // Pull the current system time and synchronize RTC time to that
    let (sys_dt, sys_unix_timestamp) = get_sys_datetime_timestamp();
    // use the set_datetime method to ensure all the timekeeping registers on
    // the rtc are aligned to the same values
    rtc.set_datetime(&sys_dt)?;

    // verify that the unix time and the
    // individual year, month, day registers are set correctly
    let rtc_unix_timestamp = rtc.get_unix_time()?;
    println!("sys unix {}\r\nrtc unix {}  ", sys_unix_timestamp, rtc_unix_timestamp);

    let rtc_dt = rtc.datetime().unwrap();
    println!("sys dt: {}", sys_dt);
    println!("rtc dt: {}", rtc_dt);

    // disable any INT output
    rtc.clear_all_int_out_bits()?;
    // disable any CLKOUT output on internal interrupt flags
    rtc.clear_all_int_clockout_bits()?;

    // enable switchover to backup power supply (on Vbackup)
    let backup_set = rtc.toggle_backup_switchover(true)?;
    println!("Vbackup switchover enabled:  {}", backup_set);

    // disable trickle charging. We assume a long-lived Vbackup source,
    // and/or a reliable Vdd supply
    let trickle_enabled= rtc.config_trickle_charge(
        false, TrickleChargeCurrentLimiter::Ohms15k)?;
    println!("trickle charge enabled: {}", trickle_enabled);

    // Configure a recurring alarm to go off every year at the turn of the new year
    let alarm_date = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
    let alarm_time = NaiveTime::from_num_seconds_from_midnight_opt(0,0).unwrap();
    let alarm_dt = NaiveDateTime::new(alarm_date, alarm_time);
    println!("alarm_dt: {}", alarm_dt);
    verify_alarm_set(
        &mut rtc, &alarm_dt, None, true, true, true);
    // set INT pin high when alarm goes off
    rtc.toggle_alarm_int_enable(true)?;
    // drive CLKOUT when alarm goes off
    rtc.toggle_clockout_on_alarm(true)?;
    rtc.config_clockout(true, ClockoutRate::Clkout1Hz)?;

    // if you don't care about logging when Vbackup switchovers take place, you can disable:
    // rtc.toggle_timestamp_logging(false);
    // But here we want to collect some stats on when the RTC switches
    // from the primary input power supply to the Vbackup power source
    rtc.config_timestamp_logging(TS_EVENT_SOURCE_BSF, false, true)?;

    let sleep_duration = Duration::seconds(10).to_std()?;
    // check the drift relative to system host time, and check for Vbackup switchover events
    for _i in 0..1 {
        if let Ok(dt) = rtc.datetime() {
            let bsf = rtc.check_and_clear_backup_event()?;
            println!("sys {}\r\nrtc {} bsf: {}", Utc::now().naive_utc(), dt, bsf);
            if bsf {
                let (evt_count, backup_evt_odt) = rtc.get_event_count_and_datetime()?;
                if let Some(backup_evt) = backup_evt_odt {
                    println!("num Vbackup switchovers: {} first: {}", evt_count, backup_evt);
                    // clear the log and continue logging
                    rtc.reset_timestamp_log()?;
                }
            }
        }
        sleep(sleep_duration);
    }

    // === Password-protect write protect (WP) registers ===
    verify_write_protection(&mut rtc)?;

    Ok(())

}
