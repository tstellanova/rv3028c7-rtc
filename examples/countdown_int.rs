extern crate rv3028c7_rtc;

use std::ops::{Add};
use linux_embedded_hal::I2cdev;
use chrono::{Datelike, Duration, NaiveDateTime, Timelike, Utc, Weekday};
use rv3028c7_rtc::{RV3028};
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
/// - GPIO 17 from rpi to the INT pin of the RTC

fn get_sys_timestamp() -> (NaiveDateTime, u32) {
    let now = Utc::now();
    let now_timestamp = now.timestamp();
    (now.naive_utc(), now_timestamp.try_into().unwrap() )
}


fn dump_gpio_events(gpio_int_req: &gpiocdev::Request) {
    //    for edge_event in gpio_int_req.edge_events()
    while Ok(true) == gpio_int_req.has_edge_event() {
        if let Ok(inner_evt) = gpio_int_req.read_edge_event() {
            println!("{:?}", inner_evt);
        }
    }
}

const MUX_I2C_ADDRESS: u8 = 0x70;
const MUX_CHAN_ZERO:u8 = 0b0000_0001 ; //channel 0, LSB
// const MUX_CHAN_SEVEN:u8 = 0b1000_0000 ; // channel 7, MSB


fn main() {

    // Initialize the I2C device
    let i2c = I2cdev::new("/dev/i2c-1").expect("Failed to open I2C device");
    // Create a new instance of the RV3028 driver
    // let mut rtc = RV3028::new(i2c);
    // alternate: connect via MUX
    let mut rtc =
      RV3028::new_with_mux(i2c, MUX_I2C_ADDRESS, MUX_CHAN_ZERO);

    let (sys_datetime, sys_unix_timestamp) = get_sys_timestamp();
    // use the set_datetime method to ensure all the timekeeping registers on
    // the rtc are aligned to the same values
    rtc.set_datetime(&sys_datetime).unwrap();
    let rtc_unix_time = rtc.get_unix_time().unwrap();
    // verify that the individual year, month, day registers are set correctly
    let (year, month, day) = rtc.get_ymd().unwrap();
    println!("start sys {} rtc {} ymd {} {} {} ", sys_unix_timestamp, rtc_unix_time, year,month,day);

    // disable PCT interrupts to begin with
    // rtc.toggle_countdown_int_enable(false).unwrap();
    rtc.clear_all_int_out_bits().unwrap();
    rtc.toggle_countdown_timer(false).unwrap();
    let _ = rtc.check_and_clear_countdown();

    let test_duration = Duration::milliseconds(1500);
    let _estimated_duration = rtc.setup_countdown_timer(&test_duration, true).unwrap();

    let init_dt = rtc.datetime().unwrap();
    let alarm_dt = init_dt.add(Duration::seconds(60));
    println!("init_dt:  {}", init_dt);
    println!("alarm_dt: {}", alarm_dt);


    // Now, prep for alarm output on INT pin in (less than) 60 seconds
    let _ = rtc.clear_all_int_out_bits();

    // This is a specific configuration for Raspberry Pi -- YMMV
    let gpio_int_req = gpiocdev::Request::builder()
      .on_chip("/dev/gpiochip0")
      .with_line(17)
      // this pin is "active" when it is low, because we've attached a pull-up resistor of 2.2..10k
      .as_active_low()
      // PullUp bias doesn't appear to work on Rpi3
      // .with_bias(Bias::PullUp) // INT pulls down briefly when triggered
      .with_edge_detection(EdgeDetection::FallingEdge)
      // the debounce filter doesn't appear to work on Rpi3
      // .with_debounce_period(Duration::from_micros(1))
      .request().unwrap();

    dump_gpio_events(&gpio_int_req);

    let start_time = Utc::now().naive_utc();
    rtc.toggle_countdown_timer(true);
    rtc.toggle_countdown_int_enable(true).unwrap();

    let cur_dt = rtc.datetime().unwrap();
    println!("wait for countdown INT to trigger..\r\n{} -> {}",cur_dt, alarm_dt);

    for _i in 0..20 {
        if let Ok(true) = gpio_int_req.wait_edge_event(Duration::seconds(30).to_std().unwrap()) {
            let cur_dt = Utc::now().naive_utc();
            println!(" {} gpio events:",cur_dt);
            dump_gpio_events(&gpio_int_req);
            let ptc_af = rtc.check_and_clear_countdown().unwrap();
            let delta  = cur_dt - start_time;
            println!("{} ptc_af: {} delta: {} expected: {}", cur_dt, ptc_af, delta, test_duration);
            println!("break on gpio_events");
            break;
        }
        else {
            let cur_dt = Utc::now().naive_utc();
            println!("{} No gpio events",cur_dt);
        }

        // there's a bit of a race condition where the PTC  flag can switch high
        // after we've already checked for gpio events
        let alarm_af = rtc.check_and_clear_countdown().unwrap();
        if alarm_af  {
            let cur_dt = Utc::now().naive_utc();
            println!("{} alarm flag: {}", cur_dt, alarm_af);
            dump_gpio_events(&gpio_int_req);
            println!("break on alarm_af true");
            break;
        }

        if cur_dt.minute() > alarm_dt.minute() {
            println!("{} break on minute expired", cur_dt);
            break;
        }
    }
    _ = rtc.clear_all_int_out_bits();
    _ = rtc.check_and_clear_countdown();


}
