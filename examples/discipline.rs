extern crate rv3028c7_rtc;

use linux_embedded_hal::I2cdev;
use chrono::{ Duration,  Utc};
use rv3028c7_rtc::RV3028;
use std::thread::sleep;
//use time_series_filter::{EwmaFilter, IntSeriesEwmaFilter};

/**
Example that sets the RTC clock to the same value as the origin system clock.
It attempts to measure how long the process of reading the system time 
and then setting the RTC time takes, then uses that to predict when to
call the RTC set time method, so that the seconds boundary aligns. 

The following was tested by enabling i2c-1 on a Raspberry Pi 3+
using `sudo raspi-config`
and connecting the SDA, SCL, GND, and 3.3V pins from RPi to the RTC
*/

/// An estimate of the time to perform the RTC clock update
const ESTIMATED_WORK_NANOS: u32 = 1_055_000; // 1_036_866 

/**
Returns measured number of nanoseconds to perform the RTC set time
@param work_nanos: Estimated number of nanoseconds to perform the RTC set time
*/
fn inner_set_time(rtc: &mut RV3028<I2cdev>, work_nanos: u32) -> u32{
      let now = Utc::now(); 
      let now_timestamp:u32 = now.timestamp().try_into().unwrap();
      // how many nanoseconds have elapsed after now_timestamp?
      let now_nanos = now.timestamp_subsec_nanos();
      // how many nanos remain in the now_timestamp second?
      let nano_remain = 1_000_000_000 - now_nanos;
      if nano_remain < 5 { return work_nanos; }
      let sleep_nanos = nano_remain - work_nanos;
      let sleep_duration = Duration::nanoseconds(sleep_nanos.try_into().unwrap()).to_std().unwrap();
      let next_timestamp = now_timestamp + 1; //the next second
      sleep(sleep_duration);
      
      rtc.set_unix_time(next_timestamp).expect("couldn't set unix time");
      let test_end = Utc::now();
      let elapsed = test_end.signed_duration_since(now);
      let elapsed_nanos:u32 = elapsed.num_nanoseconds().unwrap().try_into().unwrap();
      let actual_work_nanos = 
        if elapsed_nanos > sleep_nanos {elapsed_nanos - sleep_nanos}
        else {ESTIMATED_WORK_NANOS };

      actual_work_nanos
}

fn main() {

    // Initialize the I2C device
    let i2c = I2cdev::new("/dev/i2c-1")
        .expect("Failed to open I2C device");

    // Create a new instance of the RV3028 driver
    let mut rtc = RV3028::new(i2c);

    // Pull the current system time and synchronize RTC time to that
    let mut work_nanos = ESTIMATED_WORK_NANOS/3;
    work_nanos = inner_set_time(&mut rtc, work_nanos);

    /*
    let mut tracking:u32 = 0;
    for i in 1..120 {
      work_nanos = inner_set_time(&mut rtc, work_nanos);
      tracking += work_nanos;
      let avg_work_nanos = tracking / i;
      println!("{} {}",work_nanos, avg_work_nanos);
      work_nanos = avg_work_nanos;
    }
    */
   
    
    loop {
      let rtc_unix_time = rtc.get_unix_time().expect("couldn't get unix time");
      let now = Utc::now();
      let now_timestamp:u32 = now.timestamp().try_into().unwrap();
      let now_nanos = now.timestamp_subsec_nanos();

      if now_timestamp != rtc_unix_time {
        println!("rtc: {} sys: {} nanos: {}",rtc_unix_time, now_timestamp, now_nanos);
        inner_set_time(&mut rtc, 100000);
      }
      else {
        sleep(Duration::microseconds(100).to_std().unwrap());
        //let sleep_nanos:i64 = (rand::random::<u32>() % now_nanos ).try_into().unwrap();
        //sleep(Duration::nanoseconds(sleep_nanos).to_std().unwrap());
      }
    }
    
}
