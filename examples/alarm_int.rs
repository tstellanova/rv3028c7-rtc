extern crate rv3028c7_rtc;

use std::ops::{Add};
use linux_embedded_hal::I2cdev;
use chrono::{Datelike, NaiveDateTime, Timelike, Utc, Weekday};
use rv3028c7_rtc::{RV3028};
use std::time::Duration;
use std::thread::sleep;
// use linux_embedded_hal::{CdevPin, gpio_cdev::{Chip, LineRequestFlags}};
// use embedded_hal::digital::v2::{InputPin};
use rtcc::DateTimeAccess;

use embedded_hal::blocking::i2c::{Write, Read, WriteRead};

/// Example testing real RTC interaction for alarm set/get,
/// assuming linux environment (such as Raspberry Pi 3+)
/// with RV3028 attached to i2c1.
/// The following was tested by enabling i2c-1 on a Raspberry Pi 3+
/// using `sudo raspi-config`
/// and connecting:
/// - SDA, SCL, GND, and 3.3V pins from rpi to the RTC
/// - GPIO 27 (physical pin 13) from rpi to the INT pin of the RTC

fn get_sys_timestamp() -> (NaiveDateTime, u32) {
    let now = Utc::now();
    let now_timestamp = now.timestamp();
    (now.naive_utc(), now_timestamp.try_into().unwrap() )
}

// run through a single iteration of alarm set, and verify the value is set
fn run_iteration<I2C,E>(rtc: &mut RV3028<I2C>, alarm_dt: &NaiveDateTime,
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

fn main() {
    // This is a specific configuration for Raspberry Pi -- YMMV

    // let mut gpiochip = Chip::new("/dev/gpiochip0").unwrap();
    //
    // // Grab a GPIO input pin on the host for receiving INT signals from RTC
    // let int_line = gpiochip.get_line(27).unwrap();
    // let handle = int_line.request(LineRequestFlags::INPUT, 1, "gpio_int").unwrap();
    // let int_pin = CdevPin::new(handle).expect("new int_pin");

    // Initialize the I2C device
    let i2c = I2cdev::new("/dev/i2c-1").expect("Failed to open I2C device");
    // Create a new instance of the RV3028 driver
    let mut rtc = RV3028::new(i2c);

    let (sys_datetime, sys_unix_timestamp) = get_sys_timestamp();
    // use the set_datetime method to ensure all the timekeeping registers on
    // the rtc are aligned to the same values
    rtc.set_datetime(&sys_datetime).unwrap();
    let rtc_unix_time = rtc.get_unix_time().unwrap();
    // verify that the individual year, month, day registers are set correctly
    let (year, month, day) = rtc.get_ymd().unwrap();
    println!("start sys {} rtc {} ymd {} {} {} ", sys_unix_timestamp, rtc_unix_time, year,month,day);

    // disable alarm interrupts to begin with
    rtc.toggle_alarm_int_enable(false).unwrap();

    let (first_alarm_dt, _out_weekday, _out_match_day, _out_match_hour, _out_match_minute) =
      rtc.get_alarm_datetime_wday_matches().unwrap();
    println!("first_alarm_dt {} ", first_alarm_dt);

    let init_dt = rtc.datetime().unwrap();
    let alarm_dt = init_dt.add(Duration::from_secs(60));
    println!("init_dt:  {}", init_dt);
    println!("alarm_dt: {}", alarm_dt);

    // date alarm variations
    run_iteration(&mut rtc, &alarm_dt, None, true, true, true);
    run_iteration(&mut rtc, &alarm_dt, None, true, true, false);
    run_iteration(&mut rtc, &alarm_dt, None, true, false, false);
    run_iteration(&mut rtc, &alarm_dt, None, false, false, false);
    run_iteration(&mut rtc, &alarm_dt, None, false, false, true);
    run_iteration(&mut rtc, &alarm_dt, None, false, true, true);
    run_iteration(&mut rtc, &alarm_dt, None, false, true, false);
    run_iteration(&mut rtc, &alarm_dt, None, true, false, true);

    // weekday alarm variations
    run_iteration(&mut rtc, &alarm_dt, Some(Weekday::Mon), true, true, true);
    run_iteration(&mut rtc, &alarm_dt, Some(Weekday::Tue), true, true, false);
    run_iteration(&mut rtc, &alarm_dt, Some(Weekday::Wed) , true, false, false);
    run_iteration(&mut rtc, &alarm_dt, Some(Weekday::Thu), false, false, false);
    run_iteration(&mut rtc, &alarm_dt, Some(Weekday::Fri), false, false, true);
    run_iteration(&mut rtc, &alarm_dt, Some(Weekday::Sat), false, true, true);
    run_iteration(&mut rtc, &alarm_dt, Some(Weekday::Sun), false, true, false);
    run_iteration(&mut rtc, &alarm_dt, Some(Weekday::Mon), true, false, true);

    // println!("int pin low? {} ", int_pin.is_low().unwrap());

    // prep for alarm output on INT pin
    run_iteration(&mut rtc, &alarm_dt, Some(alarm_dt.weekday()),
                  false, false, true);
    rtc.toggle_alarm_int_enable(true).unwrap();
    let cur_dt = rtc.datetime().unwrap();
    println!("wait for alarm to trigger..\r\n{} {}",cur_dt, alarm_dt);

    // let  last_low = int_pin.is_low().unwrap();
    // let  last_high = int_pin.is_high().unwrap();
    for _i in 0..10 {
        sleep(Duration::from_secs(10));

        let alarm_af = rtc.check_and_clear_alarm().unwrap();
        let cur_dt = rtc.datetime().unwrap();
        println!("{} alarm flag: {}", cur_dt, alarm_af);

        // let pin_low = int_pin.is_low().unwrap();
        // let pin_high = int_pin.is_high().unwrap();
        // if pin_low != last_low || pin_high != last_high {
        //     println!("pin high {} low {}", pin_high, pin_low);
        //     break;
        // }
        if alarm_af { break; }

        if cur_dt.minute() >= alarm_dt.minute() {
            break;
        }
    }

}
