extern crate rv3028c7_rtc;

use std::convert::TryInto;
use linux_embedded_hal::I2cdev;
use chrono::{ Utc};
use rv3028c7_rtc::RV3028;
use std::time::{Duration };
use std::thread::sleep;
use ds323x::{DateTimeAccess, Ds323x  };
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

fn main() {

    // Initialize the I2C bus (device)
    let i2c = I2cdev::new("/dev/i2c-1").expect("Failed to open I2C device");
    let i2c_bus = shared_bus::BusManagerSimple::new(i2c);

    // Create two instances of the RV3028 driver
    let mut rtc1 = RV3028::new_with_mux(i2c_bus.acquire_i2c(), MUX_I2C_ADDRESS, MUX_CHAN_ZERO);
    rtc1.disable_trickle_charge().expect("unable to disable_trickle_charge");
    let mut rtc2 = RV3028::new_with_mux(i2c_bus.acquire_i2c(), MUX_I2C_ADDRESS, MUX_CHAN_SEVEN);
    rtc2.disable_trickle_charge().expect("unable to disable_trickle_charge");

    let mut dsrtc1 = Ds323x::new_ds3231(i2c_bus.acquire_i2c());
    let mut dsrtc2 = Ds323x::new_ds3231(i2c_bus.acquire_i2c());
    let mut muxdev = i2c_bus.acquire_i2c();

    // get the sys time and synchronize that onto the two RTCs
    let now = Utc::now();
    let sys_timestamp_64 = now.timestamp();
    let sys_timestamp_32:u32 = sys_timestamp_64.try_into().unwrap();
    let subsec_nanos = now.timestamp_subsec_nanos();

    let next_timestamp = sys_timestamp_32 + 1;
    let wait_nanos: u64 = (1_000_000_000 - subsec_nanos).try_into().unwrap();
    let wait_duration = Duration::from_nanos(wait_nanos);
    // sleep until the next second boundary to set the next second
    sleep(wait_duration);

    // the following should fail if the mux or child devices don't respond
    rtc1.set_unix_time(next_timestamp).expect("couldn't set rtc1");
    rtc2.set_unix_time(next_timestamp).expect("couldn't set rtc2");

    let (sys_timestamp_start, subsec) = get_sys_timestamp_and_micros();
    println!("set time {} at {} + {} us", next_timestamp, sys_timestamp_start, subsec );

    let datetime = ds323x::NaiveDateTime::from_timestamp_opt(next_timestamp.into(), 0).unwrap();
    muxdev.write(MUX_I2C_ADDRESS, &[MUX_CHAN_TWO]).expect("mux ch2 i2c err");
    dsrtc1.set_datetime(&datetime).unwrap();
    muxdev.write(MUX_I2C_ADDRESS, &[MUX_CHAN_FOUR]).expect("mux ch4 i2c err");
    dsrtc2.set_datetime(&datetime).unwrap();

    // check the drift over and over again
    loop {
        let (sys_timestamp,  subsec) = get_sys_timestamp_and_micros();
        let out1:i64 = rtc1.get_unix_time().expect("couldn't get RV unix time").into();
        let out2:i64 = rtc2.get_unix_time().expect("couldn't get RV unix time").into();
        muxdev.write(MUX_I2C_ADDRESS, &[MUX_CHAN_TWO]).expect("mux ch2 i2c err");
        let dsout1 = dsrtc1.datetime().expect("couldn't get DS datetime ").timestamp();
        muxdev.write(MUX_I2C_ADDRESS, &[MUX_CHAN_FOUR]).expect("mux ch4 i2c err");
        let dsout2 = dsrtc2.datetime().expect("couldn't get DS datetime").timestamp();

        // adjust the check time so that we're checking as fast as we
        // can just after one second has elapsed
        let fall_back =
            Duration::from_micros(subsec.into());
        let wait_duration =
            Duration::from_secs(21).checked_sub(fall_back).unwrap();

        if sys_timestamp != out1 || sys_timestamp != out2  ||
          sys_timestamp != dsout1 || sys_timestamp != dsout2 {
            println!("sys: {} us: {} rtc1: {} rtc2: {} ds1: {} ds2: {}",
            sys_timestamp, subsec,
            out1, out2,
            dsout1, dsout2);
            println!("time to drift: {}", sys_timestamp - sys_timestamp_start);
            break;
        }
        else if (sys_timestamp % 5) == 0 {
          println!("sys: {} duration: {}", sys_timestamp, sys_timestamp - sys_timestamp_start);
        }
        sleep(wait_duration);
    }

}


