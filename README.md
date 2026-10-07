# rv3028c7-rtc

Rust `no_std` driver for RV-3028-C7 and similar Real Time Clocks (RTC)
manufactured by Micro Crystal AG, Switzerland. 
Based on the
[Application Manual](https://www.microcrystal.com/fileadmin/Media/Products/RTC/App.Manual/RV-3028-C7_App-Manual.pdf)
downloaded November 2023.

This driver provides many methods for reading and writing the i2c registers of the RTC,
but it does not fully exercise all the features of the RTC. 

The driver works with any I2C implementation of the [`embedded-hal`](https://crates.io/crates/embedded-hal)
1.0 `I2c` trait. The minimum supported Rust version is 1.85.

## Features

Dates and times can be handled with [`chrono`](https://crates.io/crates/chrono),
[`jiff`](https://crates.io/crates/jiff), or without any date/time library.

| Feature | Default | Provides |
|---|---|---|
| `chrono` | yes | `DateTimeAccess` (from [`rtcc`](https://crates.io/crates/rtcc)) and `ChronoAccess`, using `chrono::NaiveDateTime` |
| `jiff` | no | `JiffDateTimeAccess`, using `jiff::civil::DateTime` |
| (none) | | `DateTimeParts` and `AlarmParts` with `set_datetime_parts`, `set_alarm_parts` and friends |

Everything that doesn't involve a date or time, such as the countdown timer
(which takes a `core::time::Duration`), interrupts, trickle charging and the unix time counter,
is always available. Both features can be enabled at the same time.

## Usage

With `chrono` (the default):

```rust
use linux_embedded_hal::I2cdev;
use rv3028c7_rtc::{ChronoAccess, DateTimeAccess, RV3028};

let i2c = I2cdev::new("/dev/i2c-1").unwrap();
let mut rtc = RV3028::new(i2c);

let now = rtc.datetime().unwrap();
// alarm on the same day of the month, at the same hour and minute
rtc.set_alarm(&now, None, true, true, true).unwrap();
```

With `jiff` (`rv3028c7-rtc = { version = "0.6", default-features = false, features = ["jiff"] }`):

```rust
use rv3028c7_rtc::jiff::civil::date;
use rv3028c7_rtc::{JiffDateTimeAccess, RV3028};

let mut rtc = RV3028::new(i2c);
rtc.set_datetime(&date(2024, 3, 15).at(13, 45, 30, 0)).unwrap();
let now = rtc.datetime().unwrap();
```

On targets without atomic compare-and-swap, such as Cortex-M0 (`thumbv6m-none-eabi`), jiff relies on
[`portable-atomic`](https://crates.io/crates/portable-atomic), and your application has to choose how it
provides atomics, for example by enabling its `critical-section` feature.

Without a date/time library (`default-features = false`):

```rust
use rv3028c7_rtc::{DateTimeParts, RV3028};

let mut rtc = RV3028::new(i2c);
rtc.set_datetime_parts(&DateTimeParts {
    year: 2024, month: 3, day: 15, hour: 13, minute: 45, second: 30,
}).unwrap();
let unix_time = rtc.get_unix_time().unwrap();
```

To talk to several devices on one bus (for example several RTCs behind an i2c mux),
share the bus with [`embedded-hal-bus`](https://crates.io/crates/embedded-hal-bus):

```rust
use core::cell::RefCell;
use embedded_hal_bus::i2c::RefCellDevice;

let i2c_bus = RefCell::new(i2c);
let mut rtc1 = RV3028::new_with_mux(RefCellDevice::new(&i2c_bus), 0x70, 0b0000_0001);
let mut rtc2 = RV3028::new_with_mux(RefCellDevice::new(&i2c_bus), 0x70, 0b1000_0000);
```

### Errors

All fallible driver methods return `rv3028c7_rtc::Error<I2C::Error>`:

- `Error::I2c(error)` preserves the underlying HAL error.
- `Error::InvalidData` reports an invalid input or register value.
- `Error::InconsistentRead` means no consecutive Unix counter reads agreed within
  four reads. The HAL remains responsible for individual transaction timeouts.

Date/time setters require valid dates in 2000–2099 and validate before any bus
access. Alarm setters validate all fields, even disabled ones. Alarm reads preserve
valid disabled values, replacing invalid disabled values with zero (or day 1 for
date alarms). Invalid enabled fields return an error. Subseconds are ignored.

### Upgrading from 0.5

- Explicit result types must use `Error<I2C::Error>` instead of `I2C::Error`;
  match `Error::I2c(inner)` to inspect bus failures.
- Version 0.6 moves from `embedded-hal` 0.2 to 1.0, so your HAL must provide an
  `embedded-hal` 1.0 I2C implementation. If you used `shared-bus`, switch to `embedded-hal-bus`
  as shown above.
- `set_alarm`, `get_alarm_datetime_wday_matches` and `get_event_count_and_datetime` are now
  methods of the `ChronoAccess` trait: add `use rv3028c7_rtc::ChronoAccess;`.
- `config_countdown_timer` takes and returns a `core::time::Duration` instead of a `chrono::Duration`,
  e.g. `Duration::from_millis(1500)` instead of `Duration::milliseconds(1500)`.
- If you import `DateTimeAccess` from `rtcc` directly, use `rtcc` 0.4.
- If you use `default-features = false`, add `features = ["chrono"]` to keep the chrono API.

See the [changelog](./CHANGELOG.md) for details.

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
- [`event_log`](./examples/ext_evt.rs) shows how to configure and enable event logging (for example, 
detecting interrupt signals on the EVI pin)
- [`alarm_int`](./examples/alarm_int.rs) shows how to set and get the alarm, and check for alarm trigger
- [`trickle`](./examples/trickle.rs) shows how to enable and disable trickle charging, as well as enable
Vbackup switchover
- [`countdown`](./examples/countdown.rs) demonstrates setting the Periodic Countdown Timer for repeating and
one-shot modes.
- [`set_alarm`](./examples/set_alarm.rs) shows setting the alarm
- [`jiff_alarm`](./examples/jiff_alarm.rs) sets the clock and an alarm using `jiff` types instead of `chrono`
  (run with `--no-default-features --features jiff`)
- [`countdown_int`](./examples/countdown_int.rs) shows the Periodic Countdown Timer triggering the INT pin
- [`long_clock_setup`](./examples/long_clock_setup.rs) enables backup switchover and logs switchover events
while watching the clock over a long period


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
RUST_BACKTRACE=1 cargo test --tests
```
This will build and run _only_ the tests, on any host platform. 
These tests do not require a real hardware RTC to be connected to the host. 
Note that plain `cargo test` currently also builds all examples,
and if you're testing on a non-linux platform the `linux_embedded_hal`-based examples will fail to build. 

To check that the examples compile from a non-linux host, build them for a linux target:
```
rustup target add aarch64-unknown-linux-gnu
cargo check --examples --target aarch64-unknown-linux-gnu
```



## Funstuff

Here's a breadboard with four RTCs connected via an i2c mux (for the RTC [comparison example](./examples/comp_mux.rs))
![](./res/quad-rtc-drift.jpg)
