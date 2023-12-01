extern crate rv3028c7_rtc;

use std::ops::{Add};
use linux_embedded_hal::I2cdev;
use chrono::{Datelike, NaiveDateTime, Timelike, Utc, Weekday};
use rv3028c7_rtc::{RV3028};
use std::time::Duration;
use rtcc::DateTimeAccess;

use embedded_hal::blocking::i2c::{Write, Read, WriteRead};
// use direct linux gpio access using cdev rather than via constrained embedded_hal methods
use gpiocdev::{ line::{EdgeDetection} };

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

fn dump_edge_events(gpio_int_req: &gpiocdev::Request) {
    //    for edge_event in gpio_int_req.edge_events()
    while Ok(true) == gpio_int_req.has_edge_event() {
        if let Ok(inner_evt) = gpio_int_req.read_edge_event() {
            println!("{:?}", inner_evt);
        }
    }
}

const MUX_I2C_ADDRESS: u8 = 0x70;
const MUX_CHAN_FIRST:u8 = 0b0000_0001 ; //channel 0, LSB
const MUX_CHAN_SECOND:u8 = 0b1000_0000 ; // channel 7, MSB


fn main() {

    // Initialize the I2C device
    let i2c = I2cdev::new("/dev/i2c-1").expect("Failed to open I2C device");
    // Create a new instance of the RV3028 driver
    // let mut rtc = RV3028::new(i2c);
    let mut rtc =
      RV3028::new_with_mux(i2c, MUX_I2C_ADDRESS, MUX_CHAN_FIRST);

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

    // Try all date alarm variations
    verify_alarm_set(&mut rtc, &alarm_dt, None, true, true, true);
    verify_alarm_set(&mut rtc, &alarm_dt, None, true, true, false);
    verify_alarm_set(&mut rtc, &alarm_dt, None, true, false, false);
    verify_alarm_set(&mut rtc, &alarm_dt, None, false, false, false);
    verify_alarm_set(&mut rtc, &alarm_dt, None, false, false, true);
    verify_alarm_set(&mut rtc, &alarm_dt, None, false, true, true);
    verify_alarm_set(&mut rtc, &alarm_dt, None, false, true, false);
    verify_alarm_set(&mut rtc, &alarm_dt, None, true, false, true);

    // Try weekday alarm variations
    verify_alarm_set(&mut rtc, &alarm_dt, Some(Weekday::Mon), true, true, true);
    verify_alarm_set(&mut rtc, &alarm_dt, Some(Weekday::Tue), true, true, false);
    verify_alarm_set(&mut rtc, &alarm_dt, Some(Weekday::Wed), true, false, false);
    verify_alarm_set(&mut rtc, &alarm_dt, Some(Weekday::Thu), false, false, false);
    verify_alarm_set(&mut rtc, &alarm_dt, Some(Weekday::Fri), false, false, true);
    verify_alarm_set(&mut rtc, &alarm_dt, Some(Weekday::Sat), false, true, true);
    verify_alarm_set(&mut rtc, &alarm_dt, Some(Weekday::Sun), false, true, false);
    verify_alarm_set(&mut rtc, &alarm_dt, Some(Weekday::Mon), true, false, true);

    // Now, prep for alarm output on INT pin in (less than) 60 seconds
    let _ = rtc.clear_all_int_out_bits();

    // This is a specific configuration for Raspberry Pi -- YMMV
    let gpio_int_req = gpiocdev::Request::builder()
      .on_chip("/dev/gpiochip0")
      .with_line(17)
      //.with_line(27)
      // this pin is "active" when it is low, because we've attached a pull-up resistor of 2.2k
      .as_active_low()
      // PullUp bias doesn't appear to work on Rpi3
      // .with_bias(Bias::PullUp) // INT pulls down briefly when triggered
      .with_edge_detection(EdgeDetection::FallingEdge)
      // the debounce filter doesn't appear to work on Rpi3
      // .with_debounce_period(Duration::from_micros(1))
      .request().unwrap();

    verify_alarm_set(&mut rtc, &alarm_dt, None,
                     false, false, true);

    if let Ok(true) = gpio_int_req.has_edge_event() {
        println!("dump stale edge events");
        dump_edge_events(&gpio_int_req);
    }

    let cur_dt = rtc.datetime().unwrap();
    rtc.toggle_alarm_int_enable(true).unwrap();
    println!("wait for alarm to trigger..\r\n{} -> {}",cur_dt, alarm_dt);

    for _i in 0..20 {
        if let Ok(true) = gpio_int_req.wait_edge_event(Duration::from_secs(5)) {
            let cur_dt = rtc.datetime().unwrap();
            println!("Edge events at {}",cur_dt);
            dump_edge_events(&gpio_int_req);
        }
        else {
            let cur_dt = rtc.datetime().unwrap();
            println!("No edge events at {}",cur_dt);
        }

        let alarm_af = rtc.check_and_clear_alarm().unwrap();
        println!("{} alarm flag: {}", cur_dt, alarm_af);
        if alarm_af {
            // If we saw edge events, this should also be true
            println!("break on alarm_af true");
            break;
        }

        if cur_dt.minute() >= alarm_dt.minute() {
            println!("break on minute expired");
            break;
        }
    }
    _ = rtc.clear_all_int_out_bits();
    _ = rtc.check_and_clear_alarm();


}
