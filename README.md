# rv3028c7-rtc

Rust `no_std` driver for RV-3028-C7 and similar Real Time Clocks (RTC)
manufactured by Micro Crystal AG, Switzerland. 
Based on the
[Application Manual](https://www.microcrystal.com/fileadmin/Media/Products/RTC/App.Manual/RV-3028-C7_App-Manual.pdf)
downloaded November 2023.

This driver provides many methods for reading and writing the i2c registers of the RTC,
but it does not fully exercise all the features of the RTC. 

## Running examples

All of the examples have been run and tested on a raspberry pi running linux, 
for example with the command line:
```
cargo run --example rpil
 ```
from the linux command line.

### Single RTC examples
These examples all assume there is a single RTC connected directly to a raspberry pi-like linux host.
- [`rpil`](./examples/rpil.rs) demonstrates some basic interactions with the RTC attached to eg a Raspberry Pi host.
- [`event_log`](./examples/event_log.rs) shows how to configure and enable event logging (for example, 
detecting interrupt signals on the EVI pin)
- [`alarm_int`](./examples/alarm_int.rs) shows how to set and get the alarm, and check for alarm trigger

### Multiple RTC examples
These examples all assume that there are multiple RTCs connected to the host
via an i2c mux (something like the TCA9548A)
- [`muxit`](./examples/muxit.rs) demonstrates talking to two RTCs (with the same i2c address) via TCA9548A-like i2c mux.
  See the [high-level wiring diagram schematic](./res/dual-rtc-schematic.pdf).
- [`comp_mux`](./examples/comp_mux.rs) compares the output of four independent RTCs (two each of RV-3028-C7 and DS3231)
  and detects when they drift apart (usually after multiple days).
  See the associated [quad RTC wiring diagram](./res/comp-quad-rtc-mux.pdf).
- [`trickle_mux`](./examples/trickle_mux.rs) shows how to configure and confirm the backup voltage supply trickle charger.

## Testing

```
cargo test --tests
```
This will build and run only the tests, on any host platform. 
These tests do not require a real hardware RTC to be connected to the host. 
Note that plain `cargo test` currently also builds all examples,
and if you're testing on a non-linux platform the `linux_embedded_hal`-based examples will fail to build. 



## Funstuff

Here's a breadboard with four RTCs connected via an i2c mux (for the RTC [comparison example](./examples/comp_mux.rs))
![](./res/quad-rtc-drift.jpg)