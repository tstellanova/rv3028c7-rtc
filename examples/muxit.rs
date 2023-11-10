extern crate rv3028c7_rtc;

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


fn get_sys_timestamp() -> u32 {
    let now = Utc::now();
    let now_timestamp = now.timestamp();
    now_timestamp.try_into().unwrap()
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
    let mut rtc2 = RV3028::new_with_mux(i2c_bus.acquire_i2c(), MUX_I2C_ADDRESS, MUX_CHAN_SECOND);

    let sys_timestamp = get_sys_timestamp();
    // the following should fail if the mux or child devices don't respond
    rtc1.set_unix_time(sys_timestamp).expect("couldn't set rtc1");
    rtc2.set_unix_time(sys_timestamp).expect("couldn't set rtc2");

    // check the drift over and over again
    loop {
        let sys_timestamp = get_sys_timestamp();
        let out1 = rtc1.get_unix_time().expect("couldn't get unix time");
        let out2 = rtc2.get_unix_time().expect("couldn't get unix time");
        println!("sys: {} rtc1: {} rtc2: {}", sys_timestamp, out1, out2);
        sleep(Duration::from_secs(10));
    }

}
