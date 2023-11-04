# rv3028c7-rtc

Rust `no_std` driver for RV-3028-C7 and similar Real Time Clocks (RTC),
provided by Micro Crystal, Switzerland. 
Based on the
[Application Manual](https://www.microcrystal.com/fileadmin/Media/Products/RTC/App.Manual/RV-3028-C7_App-Manual.pdf)
downloaded November 2023.

This driver provides basic methods for reading and writing the i2c registers of the RTC,
but it does not exercise all the features of the RTC. 

## Running examples

- `rpil` demonstrates some basic interactions with the RTC, using a Raspberry Pi platform. 
This can be run with `cargo run --example rpil` from the linux command line.


## Testing

By default, `cargo test` currently also builds all examples,
and if you're testing on a non-linux platform the Raspberry Pi example will fail. 
You can use the following to only build tests:

```
cargo test --tests
```

