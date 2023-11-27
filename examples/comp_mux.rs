extern crate rv3028c7_rtc;

use core::ops::Add;
use linux_embedded_hal::I2cdev;
use chrono::{NaiveDateTime, Timelike, Utc};
use rv3028c7_rtc::{RV3028, DateTimeAccess};
use std::time::{Duration };
use std::thread::sleep;
use ds323x::Ds323x;
use embedded_hal::blocking::i2c::Write;

/**
Example comparing set/get of date and time for two different models of RTC,
in this case the RV-3028-C7 and the DS3231.

Assumptions:
- RTCs are attached to an i2c mux to avoid i2c address conflicts
- The i2c mux behaves like a Texas Instruments TCA9548A
- One RV-3028-C7 RTC is attached to channel 0 on the mux; the other is attached to channel 7
- One DS3231 RTC is attached to channel 2 on the mux; the other is attached to channel 4
- The host platform this example runs on behaves like a Raspberry Pi 3+ running linux
- The mux is attached to i2c1 on the host platform

The following was tested by enabling i2c-1 on a Raspberry Pi 3+
using `sudo raspi-config`
and connecting the SDA, SCL, GND, and 3.3V pins from RPi to the mux
and relevant pins to the RTCs
*/


fn get_simple_dt_and_subsec() -> (NaiveDateTime, u32) {
    let now = Utc::now();
    let dt = now.naive_utc();
    let simple_dt = dt.with_nanosecond(0).unwrap();
    (simple_dt, now.timestamp_subsec_micros())
}

fn get_sys_dt_and_subsec() -> (NaiveDateTime, u32) {
    let now = Utc::now();
    let dt = now.naive_utc();
    (dt,  now.timestamp_subsec_micros())
}

fn get_sys_timestamp_and_micros() -> (i64, u32) {
    let now = Utc::now();
    (
        now.timestamp(),
        now.timestamp_subsec_micros(),
    )
}


const MUX_I2C_ADDRESS: u8 = 0x70;
const MUX_CHAN_ZERO:u8 = 0b0000_0001 ; //channel 0, LSB
const MUX_CHAN_SEVEN:u8 = 0b1000_0000 ; // channel 7, MSB

const MUX_CHAN_TWO:u8 = 0b0000_0100 ; // channel 2 
const MUX_CHAN_FOUR:u8 = 0b0001_0000 ; // channel 4

const IDEAL_RELOAD_US: u32 = 4866; // time it takes to set all RTCs and recheck sys time
const IDEAL_DELAY_US_LOW: u32 = 1_000_000 - IDEAL_RELOAD_US;
const IDEAL_DELAY_US_HIGH: u32 = IDEAL_DELAY_US_LOW + 100;
const IDEAL_HALF_DELAY_US: u32 = IDEAL_RELOAD_US/2;

fn main() {

    // Initialize the I2C bus (device)
    let i2c = I2cdev::new("/dev/i2c-1").expect("Failed to open I2C device");
    let i2c_bus = shared_bus::BusManagerSimple::new(i2c);

    let mut muxdev = i2c_bus.acquire_i2c();

    // Create two instances of the DS3231 driver
    let mut ds1 = Ds323x::new_ds3231(i2c_bus.acquire_i2c());
    let mut ds2 = Ds323x::new_ds3231(i2c_bus.acquire_i2c());

    // Create two instances of the RV3028 driver
    let mut rv1 = RV3028::new_with_mux(i2c_bus.acquire_i2c(), MUX_I2C_ADDRESS, MUX_CHAN_ZERO);
    let mut rv2 = RV3028::new_with_mux(i2c_bus.acquire_i2c(), MUX_I2C_ADDRESS, MUX_CHAN_SEVEN);

    'session: loop {
        // get the host system timestamp and synchronize that onto all RTCs
        let next_dt =
          loop {
              // catch the next whole second transition
              let (simple_dt, subsec_micros) = get_simple_dt_and_subsec();
              if subsec_micros < IDEAL_DELAY_US_LOW || subsec_micros > IDEAL_DELAY_US_HIGH  {
                  continue 'session;
              }
              break simple_dt.add(Duration::from_secs(1))
          };

        // the following should fail if the mux or child devices don't respond
        rv1.set_datetime(&next_dt).unwrap();
        muxdev.write(MUX_I2C_ADDRESS, &[MUX_CHAN_TWO]).expect("mux ch2 i2c err");
        ds1.set_datetime(&next_dt).unwrap();

        rv2.set_datetime(&next_dt).unwrap();
        muxdev.write(MUX_I2C_ADDRESS, &[MUX_CHAN_FOUR]).expect("mux ch4 i2c err");
        ds2.set_datetime(&next_dt).unwrap();

        let (sys_dt, subsec) = get_sys_dt_and_subsec();
        // setting RTC times and rechecking sys time seems to take ~3800 micros with my equipment
        println!("set time {} at {} ({} µs) ", next_dt, sys_dt, subsec);
        if subsec > IDEAL_HALF_DELAY_US { continue 'session }
        let sys_timestamp_start  = sys_dt.timestamp();


        // Read timestamps from all RTCs over and over again,
        // until we detect they are mismatched, which indicates clock drift.
        // Note that we read back from the clocks in the same order we wrote to them.
        loop {
            let rv1_out = rv1.datetime().unwrap().timestamp();
            muxdev.write(MUX_I2C_ADDRESS, &[MUX_CHAN_TWO]).expect("mux ch2 i2c err");
            let ds1_out = ds1.datetime().unwrap().timestamp();

            let rv2_out = rv2.datetime().unwrap().timestamp();
            muxdev.write(MUX_I2C_ADDRESS, &[MUX_CHAN_FOUR]).expect("mux ch4 i2c err");
            let ds2_out = ds2.datetime().unwrap().timestamp();

            let (sys_timestamp, subsec_micros) = get_sys_timestamp_and_micros();

            // adjust the check time so that we're checking as fast as we
            // can just after one second has elapsed
            let fall_back =
              Duration::from_micros(((150*subsec_micros)/100) as u64);
            let wait_duration =
              Duration::from_secs(20).checked_sub(fall_back).unwrap();

            let elapsed = sys_timestamp - sys_timestamp_start;

            if sys_timestamp != rv1_out || sys_timestamp != rv2_out ||
              sys_timestamp != ds1_out || sys_timestamp != ds2_out
            {
                println!("sys: {} µs:: {} rv1: {} rv2: {} ds1: {} ds2: {}",
                         sys_timestamp, subsec_micros,
                         rv1_out, rv2_out,
                         ds1_out, ds2_out);
                println!("=== Drift elapsed secs: {} mins: {} hours: {} days: {}",
                         elapsed, elapsed / 60, elapsed / 3600, elapsed / 62400);
                break;
            } //else if (sys_timestamp % 5) == 0 {
            else {
                println!("elapsed: {} µs: {}", elapsed, subsec_micros);
            }
            sleep(wait_duration);
        }
    }

}


