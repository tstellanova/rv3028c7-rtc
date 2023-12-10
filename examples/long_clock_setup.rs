extern crate rv3028c7_rtc;

use anyhow::Result;
use linux_embedded_hal::I2cdev;
use chrono::{Duration, Utc};
use rv3028c7_rtc::{
    RV3028,
    DateTimeAccess,
    NaiveDateTime, EventTimeStampLogger, TS_EVENT_SOURCE_BSF
};
use std::thread::sleep;

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

const LONG_CLOCK_PASSWORD: [u8; 4] = [0xFE,0xED,0xD0,0xBB];
const BOGUS_PASSWORD: [u8; 4] = [0xFE,0xED,0xFA,0xDE];

fn main() -> Result<()> {

    // Initialize the I2C device
    let i2c = I2cdev::new("/dev/i2c-1").expect("Failed to open I2C device");

    // Create a new instance of the RV3028 driver
    let mut rtc = RV3028::new(i2c);

    // preemptively set the user password in case it has already been written to EEPROM
    if rtc.check_writer_password_enabled()? {
        println!("Write protection enabled-- entering password");
        rtc.enter_user_password(&LONG_CLOCK_PASSWORD)?;
    }

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

    // enable switchover to backup power supply (on Vbackup)
    rtc.clear_all_int_out_bits().unwrap();
    if let Ok(backup_set) = rtc.toggle_backup_switchover(true) {
        println!("backup_set:  {}", backup_set);
    }

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

    // TODO password-protect WP registers
    let initial_user_ram = rtc.get_user_ram()?;
    println!("initial_user_ram {:?}",initial_user_ram);

    rtc.set_user_ram(&[55u8,77])?;
    let pre_user_ram = rtc.get_user_ram()?;
    println!("pre_user_ram {:?}",pre_user_ram);

    rtc.set_writer_password(&LONG_CLOCK_PASSWORD, true)?;

    // this write to user RAM register should fail because password mismatches EEPROM
    rtc.enter_user_password(&BOGUS_PASSWORD)?;
    rtc.set_user_ram(&[44u8,66])?;
    let post_user_ram = rtc.get_user_ram()?;
    println!("post_user_ram {:?}",post_user_ram);

    // this write to user RAM register should pass because password matches EEPROM
    rtc.enter_user_password(&LONG_CLOCK_PASSWORD)?;
    rtc.set_user_ram(&[44u8,66])?;
    let post_user_ram = rtc.get_user_ram()?;
    println!("post_user_ram {:?}",post_user_ram);


    Ok(())

}
