extern crate rv3028c7_rtc;

use std::convert::TryInto;
use linux_embedded_hal::I2cdev;
use chrono::{ Utc};
use rv3028c7_rtc::RV3028;
use std::time::{Duration };
use std::thread::sleep;



/**
Example testing muxing between two different RTCs
with the same i2c device address.

Assumptions:
- The i2c mux behaves like a Texas Instruments TCA9548A
- One RTC is attached to channel 0 on the mux; the other is attached to channel 7
- The platform this example runs on behaves like a Raspberry Pi 3+ running linux
- The mux is attached to i2c1

The following was tested by enabling i2c-1 on a Raspberry Pi 3+
using `sudo raspi-config`
and connecting the SDA, SCL, GND, and 3.3V pins from RPi to the mux
and relevant pins to the two RTCs
*/


fn get_sys_timestamp_and_nanos() -> (u32, u32) {
    let now = Utc::now();
    (
        now.timestamp().try_into().unwrap(),
        now.timestamp_subsec_nanos()
    )
}

fn get_sys_timestamp_and_micros() -> (u32, u32) {
    let now = Utc::now();
    (
        now.timestamp().try_into().unwrap(),
        now.timestamp_subsec_micros(),
    )
}

const MUX_I2C_ADDRESS: u8 = 0x70;
const MUX_CHAN_FIRST:u8 = 0b0000_0001 ; //channel 0, LSB
const MUX_CHAN_SECOND:u8 = 0b1000_0000 ; // channel 7, MSB

fn main() {

    // Initialize the I2C bus (device)
    let i2c = I2cdev::new("/dev/i2c-1").expect("Failed to open I2C device");
    let i2c_bus = shared_bus::BusManagerSimple::new(i2c);

    // Create two instances of the RV3028 driver
    let mut rtc1 = RV3028::new_with_mux(i2c_bus.acquire_i2c(), MUX_I2C_ADDRESS, MUX_CHAN_FIRST);
    rtc1.disable_trickle_charge().expect("unable to disable_trickle_charge");
    let mut rtc2 = RV3028::new_with_mux(i2c_bus.acquire_i2c(), MUX_I2C_ADDRESS, MUX_CHAN_SECOND);
    rtc2.disable_trickle_charge().expect("unable to disable_trickle_charge");

    // get the sys time and synchronize that onto the two RTCs
    let (sys_timestamp, subsec_nanos) = get_sys_timestamp_and_nanos();
    let next_timestamp = sys_timestamp + 1;
    let wait_nanos: u64 = (1_000_000_000 - subsec_nanos).try_into().unwrap();
    let wait_duration = Duration::from_nanos(wait_nanos);
    // sleep until the next second boundary to set the next second
    sleep(wait_duration);

    // the following should fail if the mux or child devices don't respond
    rtc1.set_unix_time(next_timestamp).expect("couldn't set rtc1");
    rtc2.set_unix_time(next_timestamp).expect("couldn't set rtc2");

    let (sys_timestamp_start, subsec) = get_sys_timestamp_and_micros();
    println!("set time {} at {} + {} us", next_timestamp, sys_timestamp_start, subsec );

    // check the drift over and over again
    loop {
        let (sys_timestamp,  subsec) = get_sys_timestamp_and_micros();
        let out1 = rtc1.get_unix_time().expect("couldn't get unix time");
        let out2 = rtc2.get_unix_time().expect("couldn't get unix time");

        // adjust the check time so that we're checking as fast as we
        // can just after one second has elapsed
        let fall_back =
            Duration::from_micros(subsec.into());
        let wait_duration =
            Duration::from_secs(60).checked_sub(fall_back).unwrap();

        if sys_timestamp != out1 || sys_timestamp != out2 {
            println!("sys: {} rtc1: {} rtc2: {} subsec: {}",
            sys_timestamp,
            out1,
            out2,
            subsec);
            println!("time to drift: {}", sys_timestamp - sys_timestamp_start);
            break;
        }
        sleep(wait_duration);
    }

}
