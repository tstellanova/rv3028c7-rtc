//! `no_std` driver for the Micro Crystal RV-3028-C7 real time clock (RTC),
//! for any I2C bus implementing the [`embedded_hal`] 1.0 [`I2c`] trait.
//!
//! The driver is [`RV3028`]. Dates and times can be handled with one of two optional
//! date/time libraries, or without any:
//!
//! | Feature | Default | Provides |
//! |---|---|---|
//! | `chrono` | yes | `DateTimeAccess` (from `rtcc`) and `ChronoAccess`, using `chrono::NaiveDateTime` |
//! | `jiff` | no | `JiffDateTimeAccess`, using `jiff::civil::DateTime` |
//! | (none) | | [`DateTimeParts`] and [`AlarmParts`], with [`RV3028::set_datetime_parts`] and friends |
//!
//! Both features can be enabled at the same time. Everything that doesn't involve a date or time,
//! such as the countdown timer, interrupts, trickle charging and the unix time counter,
//! is always available.
//!
//! # Example
//!
//! ```
//! use embedded_hal::i2c::I2c;
//! use rv3028c7_rtc::{DateTimeParts, Error, RV3028};
//!
//! fn set_clock<I2C: I2c>(i2c: I2C) -> Result<u32, Error<I2C::Error>> {
//!     let mut rtc = RV3028::new(i2c);
//!     rtc.set_datetime_parts(&DateTimeParts {
//!         year: 2024, month: 3, day: 15, hour: 13, minute: 45, second: 30,
//!     })?;
//!     rtc.get_unix_time()
//! }
//! ```
//!
//! With the `chrono` feature, the same through `DateTimeAccess`:
//!
//! ```
//! # #[cfg(feature = "chrono")] {
//! use embedded_hal::i2c::I2c;
//! use rv3028c7_rtc::{DateTimeAccess, Error, NaiveDateTime, RV3028};
//!
//! fn read_clock<I2C: I2c>(i2c: I2C) -> Result<NaiveDateTime, Error<I2C::Error>> {
//!     let mut rtc = RV3028::new(i2c);
//!     rtc.datetime()
//! }
//! # }
//! ```

#![cfg_attr(not(test), no_std)]


#[cfg(feature = "chrono")]
mod chrono_impl;
#[cfg(feature = "chrono")]
pub use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Weekday};
#[cfg(feature = "chrono")]
pub use rtcc::{  DateTimeAccess };
#[cfg(feature = "chrono")]
pub use chrono_impl::ChronoAccess;
#[cfg(feature = "jiff")]
mod jiff_impl;
#[cfg(feature = "jiff")]
pub use jiff;
#[cfg(feature = "jiff")]
pub use jiff_impl::JiffDateTimeAccess;

use core::time::Duration;
use embedded_hal::i2c::I2c;

/// Errors reported by the RTC driver. No allocation is required.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error<E> {
  /// The underlying I2C transaction failed.
  I2c(E),
  /// An input or register value is not a valid supported date, time or alarm.
  InvalidData,
  /// No two consecutive Unix counter reads agreed within four reads.
  InconsistentRead,
}

impl<E: core::fmt::Debug> core::fmt::Display for Error<E> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      Self::I2c(error) => write!(f, "I2C error: {error:?}"),
      Self::InvalidData => f.write_str("invalid RTC data"),
      Self::InconsistentRead => f.write_str("inconsistent RTC counter reads"),
    }
  }
}

impl<E: core::fmt::Debug> core::error::Error for Error<E> {}

// Fixed i2c bus address of the device (7-bit)
const RV3028_ADDRESS: u8 = 0xA4 >> 1;

// Main time register addresses
const REG_SECONDS: u8 = 0x00;
// const REG_MINUTES: u8 = 0x01;
// const REG_HOURS: u8 = 0x02;


// Holds the current day of the week.
// Each value represents one weekday that is assigned by the user.
// Values will range from 0 to 6.
// The weekday counter is simply a 3-bit counter which counts up to 6 and then resets to 0.
const REG_WEEKDAY: u8 = 0x03;

// Holds the current day of the month, in two binary coded decimal (BCD) digits.
// Values will range from 01 to 31.
// Leap years are correctly handled from 2000 to 2099.
const REG_DATE: u8 = 0x04;

// Holds the current month, in two binary coded decimal (BCD) digits.
// Values will range from 01 to 12.
// const REG_MONTH: u8 = 0x05;
// const REG_YEAR: u8 = 0x06;

// Holds the Minutes Alarm Enable bit AE_M,
// and the alarm value for minutes,
// in two binary coded decimal (BCD) digits.
// Values will range from 00 to 59.
const REG_MINUTES_ALARM: u8 = 0x07;

// Holds the Hours Alarm Enable bit AE_H and the alarm value for hours,
// in two binary coded decimal (BCD) digits.
// - If the 12_24 bit is cleared (default value) (see Control 2 register),
// the values will range from 0 to 23.
// - If the 12_24 bit is set, the hour values will be from 1 to 12
// and the AMPM bit will be 0 for AM hours and 1 for PM hours.
// - If the 12_24 hour mode bit is changed then the value in the Hours Alarm register must be re-initialized.
const REG_HOURS_ALARM: u8 = 0x08;

// Holds the Weekday/Date Alarm (WADA) Enable bit AE_WD.
// - If the WADA bit is 0 (Bit 5 in Register REG_CONTROL1),
// it holds the alarm value for the weekday (weekdays assigned by the user),
// in two binary coded decimal (BCD) digits.
// Values will range from 0 to 6.
// - If the WADA bit is 1, it holds the alarm value for the date, in two binary coded decimal (BCD)
// digits. Values will range from 01 to 31.
const REG_WEEKDAY_DATE_ALARM: u8 = 0x09;

// This register is used to set the lower 8 bits of the 12 bit Timer Value (preset value)
// for the Periodic Countdown Timer.
// This value will be automatically reloaded into the Countdown Timer when it reaches zero
// If the TRPT bit is 1, this value will be automatically reloaded into the Countdown Timer
// when it reaches zero: this allows for periodic timer interrupts
const REG_TIMER_VALUE0: u8 = 0x0A;

// This register is used to set the upper 4 bits of the 12 bit Timer Value (preset value)
// for the Periodic Countdown Timer.
// If the TRPT bit is 1, this value will be automatically reloaded into the Countdown Timer
// when it reaches zero: this allows for periodic timer interrupts
// const REG_TIMER_VALUE1: u8 = 0x0B;

const REG_TIMER_STATUS0: u8 = 0x0C; // Read-only lower 8 bits of Periodic Countdown Timer
// const REG_TIMER_STATUS1: u8 = 0x0D; // Read-only upper 4 bits of Periodic Countdown Timer


// This register is used to detect the occurrence of various interrupt events
// and reliability problems in internal data.
const REG_STATUS: u8 = 0x0E;

// This register is used to configure
// - the Alarm Interrupt function
// - the Periodic Time Update Interrupt function
// - and to select or set operations for the Periodic Countdown Timer.
const REG_CONTROL1:u8  = 0x0F;

// This register is used to control:
// - interrupt event output for the INT̅ pin
// - stop/start status of clock and calendar operations
// - interrupt controlled clock output on CLKOUT pin
// - hour mode and time stamp enable
const REG_CONTROL2:u8 = 0x10;

// Clock Interrupt Mask
// This register is used to select a predefined interrupt for automatic clock output.
// Setting a bit to 1 selects the corresponding interrupt.
// Multiple interrupts can be selected.
// After power on, no interrupt is selected (see CLOCK OUTPUT SCHEME).
const REG_CLOCK_INTERRUPT_MASK:u8 = 0x12;

// Event Control register: EHL, ET,TSR, TSOW, TSS
const REG_EVENT_CONTROL: u8 = 0x13;

// Time Stamp function registers (Event Logging)
const REG_COUNT_EVENTS_TS: u8 = 0x14; // Count TS
// const REG_SECONDS_TS: u8 = 0x15; // Seconds TS
// const REG_MINUTES_TS: u8 = 0x16; // Minutes TS
// const REG_HOURS_TS: u8 = 0x17; // Hours TS
// const REG_DATE_TS: u8 = 0x18; // Date TS
// const REG_MONTH_TS: u8 = 0x19; // Month TS
// const REG_YEAR_TS: u8 = 0x1A; // Month TS


// First address of "Unix Time Counter"
const REG_UNIX_TIME_0: u8 = 0x1B;
// const REG_UNIX_TIME_1: u8 = 0x1C;
// const REG_UNIX_TIME_2: u8 = 0x1D;
// const REG_UNIX_TIME_3: u8 = 0x1E;

// REG_CONTROL1 "Control 1" register bits:
#[repr(u8)]
enum RegControl1Bits {
  // TRPT / Timer Repeat bit. Single or Repeat countdown timer
  TimerRepeatBit =  1 << 7,
  // WADA / Weekday Alarm / Date Alarm selection bit
  WadaBit = 1 << 5,
  //  USEL / Update Interrupt Select bit. Seconds or minutes.
  UselBit = 1 << 4,
  //  EERD_BIT: u8 = 1 << 3,
  // TE / Periodic Countdown Timer Enable bit.
  TimerEnableBit = 1 << 2,
  // TD / Timer Clock Frequency selection bits
  TimerClockFreqBits = 0b11,
}

/// Countown timer clock frequency selector
#[derive(Clone, Copy, Debug, PartialEq)]
enum TimerClockFreq {
  Hertz4096 = 0b00, // 4096 Hz, 244.14 μs period
  Hertz64 = 0b01, // 64 Hz, 15.625 ms period
  Hertz1 = 0b10, // 1 Hz, One second period
  HertzSixtieth = 0b11, // 1/60 Hz, One minute period
}

// REG_STATUS Status register bits:
#[repr(u8)]
enum RegStatusBits {
  //EEBUSY_BIT = 1 << 7,
  // CLKF  / Clock Output Interrupt Flag
  ClockIntFlagBit = 1 << 6,
  // BSF bit
  BackupSwitchFlag = 1 << 5,
  // UF / Periodic Time Update Flag
  TimeUpdateFlag = 1 << 4,
  // TF bit / Periodic Countdown Timer Flag
  PeriodicTimerFlag = 1 << 3,
  // AF / Alarm Flag
  AlarmFlagBit = 1 << 2,
  // EVF / Event Flag (external event interrupt)
  EventFlagBit = 1 << 1,
  // PORF / Power On Reset Flag
  PowerOnResetFlagBit = 1 << 0,
}

// EEPROM register addresses and commands
const EEPROM_MIRROR_ADDRESS: u8 = 0x37;// RAM mirror of EEPROM config values
// const EEPROM_CMD_READ: u8 = 0x00;
// const EEPROM_CMD_WRITE: u8 = 0x01;


// REG_EVENT_CONTROL Event Control register bits:   EHL, ET, TSR, TSOW, TSS
#[repr(u8)]
enum RegEventControlBits {
  // EHL bit / Event High/Low Level (Rising/Falling Edge) selection for detection
  EventHighLowBit = 1 << 6,
  // ET bits / Event Filtering Time
  EventFilteringTimeBits = 0b11 << 4,
  // TSR bit
  TimeStampResetBit = 1 << 2,
  // TSOW bit
  TimeStampOverwriteBit = 1 << 1,
  // TSS / Time Stamp Source bit
  TimeStampSourceBit = 1 << 0,
}

/// Event log source is external interrupt EVI (default)
pub const TS_EVENT_SOURCE_EVI: u8 = 0;
/// Event log source is backup power switchover
pub const TS_EVENT_SOURCE_BSF: u8 = 1;

// REG_CLOCK_INTERRUPT_MASK bits
#[repr(u8)]
enum RegClockIntMaskBits {
  // CEIE / Clock output on Event Interrupt bit
  ClockoutOnExtEvtBit = 1 << 3,
  // CAIE / Clock output on Alarm Interrupt bit
  ClockoutOnAlarmBit = 1 << 2,
  // CTIE / Clock output on Periodic Countdown Timer Interrupt bit
  ClockoutOnPctBit = 1 << 1,
  // CUIE / Clock output on Periodic Time Update Interrupt bit
  ClockoutOnUpdateBit = 1 << 0,
}

// REG_CONTROL2 "Control 2" register bits: TSE CLKIE UIE TIE AIE EIE 12_24 RESET
#[repr(u8)]
enum RegControl2Bits {
  // TSE / Time Stamp Enable bit
  TimeStampEnableBit = 1 << 7,
  // CLKIE / Clock Output enabled by Interrupt source. (see also CLKOE)
  ClockoutIntEnableBit = 1 << 6,
  // UIE / Time Update Interrupt Enable
  TimeUpdateIntEnableBit = 1 << 5,
  // TIE Countdown Timer Interrupt Enable bit
  TimerIntEnableBit = 1 << 4,
  // AIE / Alarm Interrupt Enable bit
  AlarmIntEnableBit = 1 << 3,
  // EIE / Event Interrupt Enable bit
  EventIntEnableBit = 1 << 2,
}

// EEPROM_MIRROR_ADDRESS / EEPROM mirror register bits:
#[repr(u8)]
enum RegEepromMirrorBits {
  // CLKOE / CLKOUT Enable bit -- if 1 (default) then normal clock output
  ClockoutOutputEnableBit = 1 << 7,
  // BCIE / Backup Switchover Interrupt Enable bit bit
  BackupSwitchIntEnableBit = 1 << 6,
  // TCE bit
  TrickleChargeEnableBit = 1 << 5,
  // BackupSwitchoverLsm = 0b11 << 2,
  // Backup Switchover Mode / BSM bits as DSM
  BackupSwitchoverDsm = 0b01 << 2,
  TrickleChargeResistanceBits = 0b11, // TCR bits
}

/// Current limiting resistor for trickle charging the backup supply,
/// see [`RV3028::toggle_trickle_charge`]. A higher resistance means less charging current.
#[derive(Clone, Copy)]
pub enum TrickleChargeCurrentLimiter {
  /// 3 kΩ (factory default)
  Ohms3k = 0b00,
  /// 5 kΩ
  Ohms5k = 0b01,
  /// 9 kΩ
  Ohms9k = 0b10,
  /// 15 kΩ
  Ohms15k = 0b11,
}

// Special alarm register value
const ALARM_NO_WATCH_FLAG: u8 = 1 <<  7;

/// Calendar date and time as stored in the RTC's date and time registers,
/// independent of any date/time library.
///
/// The RTC's date registers only cover the years 2000 to 2099.
/// Setters reject invalid dates and times before accessing the bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateTimeParts {
  /// Full year, 2000 to 2099
  pub year: i32,
  /// Month, 1 to 12
  pub month: u8,
  /// Day of month, 1 to 31
  pub day: u8,
  /// Hour, 0 to 23
  pub hour: u8,
  /// Minute, 0 to 59
  pub minute: u8,
  /// Second, 0 to 59
  pub second: u8,
}

impl DateTimeParts {
  fn validate<E>(&self) -> Result<(), Error<E>> {
    let days = match self.month {
      1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
      4 | 6 | 9 | 11 => 30,
      2 if self.year % 4 == 0 => 29,
      2 => 28,
      _ => return Err(Error::InvalidData),
    };
    if !(2000..=2099).contains(&self.year) || self.day == 0 || self.day > days
      || self.hour > 23 || self.minute > 59 || self.second > 59 {
      return Err(Error::InvalidData);
    }
    Ok(())
  }

  // Days since 1970-01-01 in the proleptic Gregorian calendar
  // (Howard Hinnant's `days_from_civil` algorithm)
  fn days_since_unix_epoch(&self) -> i64 {
    let month = self.month as i64;
    let year = self.year as i64 - if month <= 2 { 1 } else { 0 };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let day_of_year = (153 * ((month + 9) % 12) + 2) / 5 + self.day as i64 - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
  }

  // Seconds since the unix epoch, as used by the RTC's unix time counter
  fn unix_timestamp(&self) -> i64 {
    self.days_since_unix_epoch() * 86_400
      + self.hour as i64 * 3_600 + self.minute as i64 * 60 + self.second as i64
  }

  // Weekday as stored by this driver: 0 = Monday through 6 = Sunday
  fn weekday(&self) -> u8 {
    // 1970-01-01 was a Thursday
    (self.days_since_unix_epoch() + 3).rem_euclid(7) as u8
  }
}

/// Alarm settings as stored in the RTC's alarm registers,
/// independent of any date/time library.
///
/// The alarm triggers when all matched fields equal the current time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlarmParts {
  /// Day of month (1 to 31) for a date alarm,
  /// or weekday (0 = Monday through 6 = Sunday) for a weekday alarm
  pub day: u8,
  /// Hour, 0 to 23
  pub hour: u8,
  /// Minute, 0 to 59
  pub minute: u8,
  /// `true` for a weekday alarm, `false` for a date alarm
  pub weekday_alarm: bool,
  /// Whether the day (or weekday) must match
  pub match_day: bool,
  /// Whether the hour must match
  pub match_hour: bool,
  /// Whether the minute must match
  pub match_minute: bool,
}

impl AlarmParts {
  fn validate<E>(&self) -> Result<(), Error<E>> {
    if self.hour > 23 || self.minute > 59
      || (self.weekday_alarm && self.day > 6)
      || (!self.weekday_alarm && !(1..=31).contains(&self.day)) {
      return Err(Error::InvalidData);
    }
    Ok(())
  }
}


/// RV-3028-C7
/// Extreme Low Power Real-Time Clock (RTC) Module with I2C-Bus Interface
/// rust no_std driver (utilizes the embedded_hal i2c interface)
pub struct RV3028<I2C> {
  i2c: I2C,
  mux_addr: u8,
  mux_chan: u8,
}

impl<I2C> RV3028<I2C>
  where
    I2C: I2c,
{

  /// New driver instance, assumes that there is no i2c mux
  /// sitting between the RTC and the host.
  pub fn new(i2c: I2C) -> Self {
    RV3028 {
      i2c,
      mux_addr: 0u8,
      mux_chan: 0u8
    }
  }

  /// Allows the caller to create a new driver instance with
  /// an i2c mux between the RTC and the host.
  /// - `mux_addr` : the i2c address of the mux itself
  /// - `mux_chan` : the mux channel assigned to the RTC
  pub fn new_with_mux(i2c: I2C, mux_addr: u8, mux_chan: u8) -> Self {
    RV3028 {
      i2c,
      mux_addr,
      mux_chan
    }
  }

  // Converts a binary value to BCD format
  fn checked_bcd(value: u8) -> Result<u8, Error<I2C::Error>> {
    if value & 0x0F > 9 || value >> 4 > 9 {
      return Err(Error::InvalidData);
    }
    Ok(Self::bcd_to_bin(value))
  }

  fn bin_to_bcd(value: u8) -> u8 {
    ((value / 10) << 4) | (value % 10)
  }

  // Converts a BCD value to binary format
  fn bcd_to_bin(value: u8) -> u8 {
    ((value & 0xF0) >> 4) * 10 + (value & 0x0F)
  }

  // If using an i2c mux, tell the mux to select our channel
  fn select_mux_channel(&mut self) -> Result<(), Error<I2C::Error>> {
    if self.mux_addr != 0u8 {
      self.i2c.write(self.mux_addr, &[self.mux_chan]).map_err(Error::I2c)
    }
    else {
      Ok(())
    }
  }

  // fn write_register(&mut self, reg: u8, data: u8) -> Result<(), Error<I2C::Error>> {
  //   self.select_mux_channel()?;
  //   self.write_register_raw(reg, data)
  // }

  fn write_register_raw(&mut self, reg: u8, data: u8) -> Result<(), Error<I2C::Error>> {
    self.i2c.write(RV3028_ADDRESS, &[reg, data]).map_err(Error::I2c)
  }

  // fn read_register(&mut self, reg: u8) -> Result<u8, Error<I2C::Error>> {
  //   self.select_mux_channel()?;
  //   self.read_register_raw(reg)
  // }

  fn read_register_raw(&mut self, reg: u8) -> Result<u8, Error<I2C::Error>> {
    let mut buf = [0];
    self.i2c.write_read(RV3028_ADDRESS, &[reg], &mut buf).map_err(Error::I2c)?;
    Ok(buf[0])
  }


  /// Check whether the Power On Reset flag is set.
  /// If this flag is cleared (set to zero) beforehand,
  /// indicates a voltage drop below VPOR.
  /// If this flag is set, the data in the device RAM registers are no longer valid
  /// and all registers must be (re)initialized.
  /// The flag value 1 is retained until a 0 is written by the user.
  /// At power up (POR) the value is set to 1, the user has to write 0 to the flag to use it.
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  pub fn check_and_clear_power_on_reset(&mut self) -> Result<bool, Error<I2C::Error>>  {
    let flag_set = 0 != self.check_and_clear_bits(
      REG_STATUS, RegStatusBits::PowerOnResetFlagBit as u8)?;
    Ok(flag_set)
  }

  /// Check whether an external event has been detected
  /// (an appropriate input signal on the EVI pin)
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  pub fn check_and_clear_ext_event(&mut self)-> Result<bool, Error<I2C::Error>>  {
    let flag_set = 0 != self.check_and_clear_bits(
      REG_STATUS, RegStatusBits::EventFlagBit as u8)?;
    Ok(flag_set)
  }

  /// Check whether a switchover to the backup power supply has occurred
  /// (the Backup Switch Flag, BSF)
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  pub fn check_and_clear_backup_event(&mut self) -> Result<bool, Error<I2C::Error>> {
    let flag_set = 0 != self.check_and_clear_bits(
      REG_STATUS, RegStatusBits::BackupSwitchFlag as u8)?;
    Ok(flag_set)
  }

  // TODO these methods have not been thoroughly tested, and are believed broken.
  // fn is_eeprom_busy(&mut self) -> Result<bool, Error<I2C::Error>> {
  //   let status = self.read_register(REG_STATUS)?;
  //   Ok(status & EEBUSY_BIT != 0)
  // }
  //
  // fn disable_auto_eeprom_refresh(&mut self) -> Result<(), Error<I2C::Error>> {
  //   let mut control_1 = self.read_register(REG_CONTROL1)?;
  //   control_1 |= EERD_BIT; // Set EERD bit
  //   self.write_register(REG_CONTROL1, control_1)
  // }
  //
  // fn enable_auto_eeprom_refresh(&mut self) -> Result<(), Error<I2C::Error>> {
  //   let mut control_1 = self.read_register(REG_CONTROL1)?;
  //   control_1 &= !(EERD_BIT); // Clear EERD bit
  //   self.write_register(REG_CONTROL1, control_1)
  // }
  //
  // pub fn eeprom_read(&mut self, address: u8) -> Result<u8, Error<I2C::Error>> {
  //   self.disable_auto_eeprom_refresh()?;
  //   while self.is_eeprom_busy()? {}
  //   // Read from EEPROM
  //   self.write_register(EEPROM_ADDRESS, address)?;
  //   let res = self.read_register(EEPROM_ADDRESS);
  //   self.enable_auto_eeprom_refresh()?;
  //   res
  // }
  //
  // pub fn eeprom_write(&mut self, address: u8, data: u8) -> Result<(), Error<I2C::Error>> {
  //   self.disable_auto_eeprom_refresh()?;
  //   while self.is_eeprom_busy()? {}
  //   self.write_register(EEPROM_ADDRESS, address)?;
  //   let res = self.write_register(EEPROM_ADDRESS, data);
  //
  //   self.enable_auto_eeprom_refresh()?;
  //   res
  // }

  // // set specific bits in a register:
  // // all bits must be high that you wish to set
  // fn set_reg_bits(&mut self, reg: u8, bits: u8) -> Result<(), Error<I2C::Error>> {
  //   self.select_mux_channel()?;
  //   self.set_reg_bits_raw(reg, bits)
  // }

  // Set specific bits in a register: "raw" means it skips the mux
  // all bits must be high that you wish to set
  fn set_reg_bits_raw(&mut self, reg: u8, bits: u8) -> Result<(), Error<I2C::Error>> {
    let mut reg_val = self.read_register_raw(reg)?;
    reg_val |= bits; // Set bits that are high
    self.write_register_raw(reg, reg_val)
  }

  // clear specific bits in a register:
  // all bits must be high that you wish to be cleared
  // fn clear_reg_bits(&mut self, reg: u8, bits: u8) -> Result<(), Error<I2C::Error>> {
  //   self.select_mux_channel()?;
  //   self.clear_reg_bits_raw(reg,bits)
  // }

  // Clears specific bits in a register, skips the mux.
  // All bits must be high that you wish to be cleared
  fn clear_reg_bits_raw(&mut self, reg: u8, bits: u8) -> Result<(), Error<I2C::Error>> {
    let mut reg_val = self.read_register_raw(reg)?;
    reg_val &= !(bits); // Clear  bits that are high
    self.write_register_raw(reg, reg_val)
  }

  /// Enable or disable trickle charging
  /// - `enable` enables trickle charging if true, disables if false
  /// - `limit_resistance` Sets the current limiting resistor value: higher means less current
  /// Disabling also resets the `limit_resistance` to 3 kΩ, the factory default.
  /// Returns the status of trickle charging (true for enabled, false for disabled)
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  pub fn toggle_trickle_charge(&mut self, enable: bool,
                               limit_resistance: TrickleChargeCurrentLimiter) -> Result<bool, Error<I2C::Error>>  {
    self.select_mux_channel()?;

    // First disable charging before changing settings
    self.clear_reg_bits_raw(
      EEPROM_MIRROR_ADDRESS,  RegEepromMirrorBits::TrickleChargeEnableBit as u8)?;
    // Reset TCR to 3 kΩ, the factory default, by clearing the TCR bits
    self.clear_reg_bits_raw(
      EEPROM_MIRROR_ADDRESS,  RegEepromMirrorBits::TrickleChargeResistanceBits as u8 )?;

    if enable {
      self.set_reg_bits_raw(EEPROM_MIRROR_ADDRESS, limit_resistance as u8)?; //TODO
      self.set_reg_bits_raw(
        EEPROM_MIRROR_ADDRESS, RegEepromMirrorBits::TrickleChargeEnableBit as u8)?;
    }

    // confirm the value set
    let conf_val =
      0 != self.read_register_raw(EEPROM_MIRROR_ADDRESS)?
        & RegEepromMirrorBits::TrickleChargeEnableBit as u8;
    Ok(conf_val)
  }

  /// Toggle whether the Vbackup power source should be used
  /// when Vdd supply level drops below useful level.
  /// - `enable` enables switching to Vbackup, disables if false
  /// Returns the set value
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  pub fn toggle_backup_switchover(&mut self, enable: bool) -> Result<bool, Error<I2C::Error>> {
    self.select_mux_channel()?;
    self.set_or_clear_reg_bits_raw(
      EEPROM_MIRROR_ADDRESS, RegEepromMirrorBits::BackupSwitchoverDsm as u8, enable)?;
    let conf_val =
      0 != self.read_register_raw(
        EEPROM_MIRROR_ADDRESS)? & RegEepromMirrorBits::BackupSwitchoverDsm as u8;
    Ok(conf_val)
  }

  /// Disable all clock outputs triggered by interrupts
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  pub fn clear_all_int_clockout_bits(&mut self) -> Result<(), Error<I2C::Error>> {
    self.select_mux_channel()?;
    self.clear_reg_bits_raw(REG_CLOCK_INTERRUPT_MASK,
                            RegClockIntMaskBits::ClockoutOnExtEvtBit as u8 |
                              RegClockIntMaskBits::ClockoutOnAlarmBit as u8 |
                            RegClockIntMaskBits::ClockoutOnPctBit as u8 |
                              RegClockIntMaskBits::ClockoutOnUpdateBit as u8)
  }


  /// Get the current value of the EEPROM mirror from RAM
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  pub fn get_eeprom_mirror_value(&mut self) -> Result<u8, Error<I2C::Error>> {
    self.select_mux_channel()?;
    let reg_val = self.read_register_raw(EEPROM_MIRROR_ADDRESS)?;
    Ok(reg_val)
  }

  // Set the bcd time tracking registers
  // assumes `select_mux_channel` has already been called
  fn set_time_raw(&mut self, dt: &DateTimeParts) -> Result<(), Error<I2C::Error>> {
    let write_buf = [
      REG_SECONDS, // select the first register
      Self::bin_to_bcd(dt.second),
      Self::bin_to_bcd(dt.minute),
      Self::bin_to_bcd(dt.hour)
    ];
    self.i2c.write(RV3028_ADDRESS, &write_buf).map_err(Error::I2c)
  }


  // Set the internal BCD date registers.
  // Note that only years from 2000 to 2099 are supported.
  // `weekday` is 0 for Monday through 6 for Sunday.
  // Assumes `select_mux_channel` has already been called
  fn set_date_raw(&mut self, dt: &DateTimeParts, weekday: u8) -> Result<(), Error<I2C::Error>> {
    let year = if dt.year > 2000 { (dt.year - 2000) as u8} else {0};
    let month = dt.month % 13;
    let day = dt.day % 32;
    let weekday = weekday % 7;

    let write_buf = [
      REG_WEEKDAY, // select the first register
      Self::bin_to_bcd(weekday ),
      Self::bin_to_bcd(day ),
      Self::bin_to_bcd(month ),
      Self::bin_to_bcd(year )
    ];
    self.i2c.write(RV3028_ADDRESS, &write_buf).map_err(Error::I2c)
  }

  // Set the unix time counter and all BCD date and time registers.
  // The unix time counter is stored in registers separate from everything else:
  // this method tries to align both, because the unix timestamp is not
  // used by eg the Event or Alarm interrupts
  fn set_datetime_raw(&mut self, unix_timestamp: u32, dt: &DateTimeParts, weekday: u8) -> Result<(), Error<I2C::Error>> {
    self.select_mux_channel()?;
    self.set_unix_time_raw(unix_timestamp)?;
    self.set_date_raw(dt, weekday)?;
    // this must come last because writing to the seconds register resets
    // the upper stage of the prescaler
    self.set_time_raw(dt)
  }

  /// Set the date and time: both the unix time counter and all internal BCD date and time
  /// registers, so that they agree. The unix timestamp and weekday are calculated from `datetime`.
  ///
  /// The RTC's date registers only cover the years 2000 to 2099, and
  /// the unix time counter wraps after 2106. Invalid values return [`Error::InvalidData`].
  ///
  /// This method resets the internal prescaler pipeline, which means that
  /// subsecond counters are zeroed, when it writes to the Seconds register.
  /// This assists with clock synchronization with external clocks.
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  ///
  /// [`Error::InvalidData`] if the date is invalid, the year is outside
  /// 2000–2099, or the time is outside 00:00:00–23:59:59. Validation happens
  /// before any bus access.
  pub fn set_datetime_parts(&mut self, datetime: &DateTimeParts) -> Result<(), Error<I2C::Error>> {
    datetime.validate()?;
    let unix_timestamp = datetime.unix_timestamp() as u32;
    self.set_datetime_raw(unix_timestamp, datetime, datetime.weekday())
  }

  /// Get the year, month, day from the internal BCD registers
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  ///
  /// [`Error::InvalidData`] if the registers contain invalid BCD or an
  /// invalid calendar date.
  pub fn get_ymd(&mut self) -> Result<(i32, u8, u8), Error<I2C::Error>> {
    let mut read_buf = [0u8;3];
    self.read_multi_registers(REG_DATE, &mut read_buf)?;
    let day = Self::checked_bcd(read_buf[0])?;
    let month = Self::checked_bcd(read_buf[1])?;
    let year:i32 = Self::checked_bcd(read_buf[2])? as i32 + 2000;

    DateTimeParts { year, month, day, hour: 0, minute: 0, second: 0 }.validate()?;
    Ok((year, month, day))
  }

  /// Get the hour, minute, second from the internal BCD registers
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  ///
  /// [`Error::InvalidData`] if the registers contain invalid BCD or a
  /// time outside 00:00:00–23:59:59.
  pub fn get_hms(&mut self) -> Result<(u8, u8, u8), Error<I2C::Error>> {
    let mut read_buf = [0u8;3];
    self.read_multi_registers(REG_SECONDS, &mut read_buf)?;
    let seconds = Self::checked_bcd(read_buf[0])?;
    let minutes = Self::checked_bcd(read_buf[1])?;
    let hours = Self::checked_bcd(read_buf[2])?;
    if hours > 23 || minutes > 59 || seconds > 59 {
      return Err(Error::InvalidData);
    }
    Ok( (hours, minutes, seconds) )
  }

  // read a block of registers all at once
  fn read_multi_registers(&mut self, reg: u8, read_buf: &mut [u8] )  -> Result<(), Error<I2C::Error>> {
    self.select_mux_channel()?;
    self.read_multi_registers_raw(reg, read_buf)
  }

  // read a block of registers all at once: skip mux
  fn read_multi_registers_raw(&mut self, reg: u8, read_buf: &mut [u8] )  -> Result<(), Error<I2C::Error>> {
    self.i2c.write_read(RV3028_ADDRESS, &[reg], read_buf).map_err(Error::I2c)
  }

  /// Set just the Unix time counter.
  /// Prefer [`Self::set_datetime_parts`] (or the `set_datetime` method of a date/time backend)
  /// to properly set all internal BCD registers.
  /// Note:
  /// - This does NOT set other internal BCD registers
  /// such as Year or Hour, which the alarm and event timestamp functions use.
  /// - This does not reset the prescaler pipeline,
  /// which means subseconds are not reset to zero.
  ///
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  pub fn set_unix_time(&mut self, unix_time: u32) -> Result<(), Error<I2C::Error>> {
    self.select_mux_channel()?;
    self.set_unix_time_raw(unix_time)
  }

  // sets the unix time counter but skips the mux
  fn set_unix_time_raw(&mut self, unix_time: u32) -> Result<(), Error<I2C::Error>> {
    let bytes = unix_time.to_le_bytes(); // Convert to little-endian byte array
    self.i2c.write(RV3028_ADDRESS, &[REG_UNIX_TIME_0, bytes[0], bytes[1], bytes[2], bytes[3]]).map_err(Error::I2c)
  }

  /// Reads the value of the RTC's unix time counter, notionally seconds elapsed since the
  /// common "unix epoch" in the year 1970. It cannot represent datetimes from prior to 1970.
  /// - Note that this is an unsigned u32 value, with different characteristics from the
  /// widely used rust / chrono i64 system timestamp.
  /// - The RTC will continue to increment this counter until it wraps at 0xFFFFFFFF
  /// which defers the "Year 2038 problem" until about the year 2106.
  /// - Note that the RTC's automatic leap year correction is only valid until 2099
  /// See the App Manual section "3.10. UNIX TIME REGISTERS"
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  pub fn get_unix_time(&mut self) -> Result<u32, Error<I2C::Error>> {
    let mut read_buf = [0u8; 4];
    self.read_multi_registers(REG_UNIX_TIME_0, &mut read_buf)?;
    let val = u32::from_le_bytes(read_buf);
    Ok(val)
  }

  /// The vendor application manual suggest we read the unix time twice,
  /// in case an internal increment or timestamp set is interspersed between the multi-byte read.
  /// Compares consecutive reads, with at most four reads total. Returns
  /// [`Error::InconsistentRead`] if none agree. Transaction timeouts remain the HAL's responsibility.
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  ///
  /// [`Error::InconsistentRead`] if no consecutive counter reads agree
  /// within four reads total. Individual transaction timeouts remain the
  /// HAL's responsibility.
  pub fn get_unix_time_blocking(&mut self) -> Result<u32, Error<I2C::Error>> {
    let mut previous = self.get_unix_time()?;
    for _ in 0..3 {
      let current = self.get_unix_time()?;
      if current == previous {
        return Ok(current);
      }
      previous = current;
    }
    Err(Error::InconsistentRead)
  }

  /// Toggle whether EVI events trigger on high/rising or low/falling edges
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  pub fn toggle_event_high_low(&mut self, high: bool) -> Result<(), Error<I2C::Error>> {
    self.set_or_clear_reg_bits(REG_EVENT_CONTROL, RegEventControlBits::EventHighLowBit as u8, high)
  }

  /// Enable INT pin output when alarm occurs
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  pub fn toggle_alarm_int_enable(&mut self, enable: bool) -> Result<(), Error<I2C::Error>> {
    self.set_or_clear_reg_bits(REG_CONTROL2, RegControl2Bits::AlarmIntEnableBit as u8, enable)
  }

  /// Toggle whether the RTC outputs a pulse (active low) on INT pin,
  /// when the countdown timer expires.
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  pub fn toggle_countdown_int_enable(&mut self, enable: bool) -> Result<(), Error<I2C::Error>> {
    self.set_or_clear_reg_bits(REG_CONTROL2, RegControl2Bits::TimerIntEnableBit as u8, enable)
  }

  /// Toggle whether interrupt signal is generated on the INT pin:
  /// - when an External Event on EVI pin occurs and TSS = 0
  /// - or when an Automatic Backup Switchover occurs and TSS = 1.
  /// The signal on the INT pin is retained until the EVF flag is cleared
  /// to 0 (no automatic cancellation)
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  pub fn toggle_ext_event_int_enable(&mut self, enable: bool) -> Result<(), Error<I2C::Error>> {
    self.set_or_clear_reg_bits(REG_CONTROL2, RegControl2Bits::EventIntEnableBit as u8, enable)
  }

  /// Toggles whether an interrupt signal is generated on the INT pin:
  /// - when the time updates at either 1 second or 1 minute intervals
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  pub fn toggle_time_up_int_enable(&mut self, enable: bool) -> Result<(), Error<I2C::Error>> {
    self.set_or_clear_reg_bits(REG_CONTROL2, RegControl2Bits::TimeUpdateIntEnableBit as u8, enable)
  }

  /// Disable all INT pin output selector bits in RAM, excludes PORIE
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  pub fn clear_all_int_out_bits(&mut self) -> Result<(), Error<I2C::Error>> {
    self.select_mux_channel()?;
    // UIE, TIE, AIE,  EIE
    self.clear_reg_bits_raw(REG_CONTROL2,
                        RegControl2Bits::TimeUpdateIntEnableBit as u8 |
                          RegControl2Bits::TimerIntEnableBit as u8 |
                          RegControl2Bits::AlarmIntEnableBit as u8 |
                          RegControl2Bits::EventIntEnableBit as u8  )?;
    // BSIE
    self.clear_reg_bits_raw(
      EEPROM_MIRROR_ADDRESS, RegEepromMirrorBits::BackupSwitchIntEnableBit as u8)?;

    // PORIE -- must be set in EEPROM -- don't bother to set?
    Ok(())
  }


  /// Clear all of the status registers that indicate whether
  /// various conditions have triggered
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  pub fn clear_all_status_flags(&mut self) -> Result<(), Error<I2C::Error>> {
    self.select_mux_channel()?;
    self.clear_reg_bits_raw(REG_STATUS,
                            RegStatusBits::ClockIntFlagBit  as u8 |
                              RegStatusBits::BackupSwitchFlag  as u8 |
                              RegStatusBits::TimeUpdateFlag  as u8 |
                              RegStatusBits::PeriodicTimerFlag  as u8 |
                              RegStatusBits::AlarmFlagBit  as u8 |
                              RegStatusBits::EventFlagBit  as u8 |
                              RegStatusBits::PowerOnResetFlagBit as u8
    )

  }

  /// - `int_enable` enables INT output on the periodic time updates
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  pub fn configure_periodic_time_update(&mut self, minutes: bool, int_enable: bool) -> Result<(), Error<I2C::Error>> {
    self.select_mux_channel()?;

    // 1. Initialize bits UIE and UF to 0.
    // 2. Choose the timer source clock and write the corresponding value in the USEL bit.
    // 3. Set the UIE bit to 1 if you want to get a hardware interrupt on INT̅ ̅ ̅ ̅ ̅
    // pin.
    // 4. Set CUIE bit to 1 to enable clock output when a time update interrupt occurs. See also CLOCK OUTPUT
    // SCHEME.
    // 5. The first interrupt will occur after the next event, either second or minute change.

    // UIE clear
    self.clear_reg_bits_raw(
      REG_CONTROL2, RegControl2Bits::TimeUpdateIntEnableBit as u8)?;
    // UF clear
    self.clear_reg_bits_raw(REG_STATUS, RegStatusBits::TimeUpdateFlag as u8)?;
    // USEL set/clear
    self.set_or_clear_reg_bits_raw(
      REG_CONTROL1, RegControl1Bits::UselBit as u8, minutes)?;
    // UIE re-set
    self.set_or_clear_reg_bits_raw(
      REG_CONTROL2, RegControl2Bits::TimeUpdateIntEnableBit as u8, int_enable)?;

    Ok(())
  }


  /// Get event count -- the number of events that have been logged since enabling logging.
  /// Returns the count of events since last reset, and the date and time of one event
  /// (see [`EventTimeStampLogger`] for which event is recorded).
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  ///
  /// [`Error::InvalidData`] if a nonzero event count accompanies invalid
  /// BCD or an invalid calendar date/time. Timestamp fields are ignored
  /// when the event count is zero.
  pub fn get_event_count_and_parts(&mut self) -> Result<(u32, Option<DateTimeParts>), Error<I2C::Error>> {
    // Read the seven raw Time Stamp Function registers in one go
    let mut read_buf:[u8;7] = [0u8;7];
    self.read_multi_registers(REG_COUNT_EVENTS_TS, &mut read_buf)?;

    // Convert BCD values to binary
    let count = read_buf[0]; // Count is already in binary

    let odt = {
      if count > 0 {
        Some(DateTimeParts {
          second: Self::checked_bcd(read_buf[1])?,
          minute: Self::checked_bcd(read_buf[2])?,
          hour: Self::checked_bcd(read_buf[3])?,
          day: Self::checked_bcd(read_buf[4])?,
          month: Self::checked_bcd(read_buf[5])?,
          year: Self::checked_bcd(read_buf[6])? as i32 + 2000,
        })
      }
      else {
        None
      }
    };

    if let Some(dt) = odt { dt.validate()?; }
    Ok((count as u32, odt))
  }

  /// Check the alarm status, and if it's triggered, clear it
  /// return bool indicating whether the alarm triggered
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  pub fn check_and_clear_alarm(&mut self) -> Result<bool, Error<I2C::Error>> {
    // Check if the AF flag is set
    let alarm_flag_set = 0 != self.check_and_clear_bits(
      REG_STATUS, RegStatusBits::AlarmFlagBit as u8)?;
    Ok(alarm_flag_set)
  }

  /// All-in-one method to set an alarm:
  /// See the App Note section "Procedure to use the Alarm Interrupt".
  /// Only day (or weekday), hour and minute are supported; see [`AlarmParts`].
  /// All fields, including disabled fields, must be valid; checked before bus access.
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  ///
  /// [`Error::InvalidData`] if the day is outside 1–31 for a date alarm
  /// or 0–6 for a weekday alarm, the hour exceeds 23, or the minute exceeds
  /// 59. All fields are validated, including disabled fields, before bus access.
  pub fn set_alarm_parts(&mut self, alarm: &AlarmParts) -> Result<(), Error<I2C::Error>> {

    alarm.validate()?;
    self.select_mux_channel()?;
    // Initialize AF to 0; AIE/AlarmIntEnableBit is managed independently
    self.clear_reg_bits_raw(REG_STATUS, RegStatusBits::AlarmFlagBit as u8)?;

    // Procedure suggested by App Notes:
    // 1. Initialize bits AIE and AF to 0.
    // 2. Choose weekday alarm or date alarm (weekday/date) by setting the WADA bit.
    // WADA = 0 for weekday alarm or WADA = 1 for date alarm.
    // 3. Write the desired alarm settings in registers 07h to 09h. The three alarm enable bits, AE_M, AE_H and
    // AE_WD, are used to select the corresponding register that has to be taken into account for match or not.
    // See the following table.

    // Clear WADA for weekday alarm, or set for date alarm
    self.set_or_clear_reg_bits_raw(
      REG_CONTROL1, RegControl1Bits::WadaBit as u8, !alarm.weekday_alarm)?;

    let bcd_minute = Self::bin_to_bcd(alarm.minute);
    self.write_register_raw(REG_MINUTES_ALARM,
                        if alarm.match_minute { bcd_minute }
                        else { ALARM_NO_WATCH_FLAG | bcd_minute })?;

    let bcd_hour = Self::bin_to_bcd(alarm.hour);
    self.write_register_raw(REG_HOURS_ALARM,
                        if alarm.match_hour { bcd_hour  }
                        else { ALARM_NO_WATCH_FLAG | bcd_hour })?;

    // either the weekday (for a weekday alarm) or the day of month (for a date alarm)
    let bcd_day = Self::bin_to_bcd(alarm.day);
    self.write_register_raw(REG_WEEKDAY_DATE_ALARM,
                        if alarm.match_day { bcd_day }
                        else { ALARM_NO_WATCH_FLAG | bcd_day })?;

    // Clear AF again in case the above setting process immediately triggered the alarm
    self.clear_reg_bits_raw(REG_STATUS, RegStatusBits::AlarmFlagBit as u8)?;

    Ok(())
  }

  /// Read the alarm settings. Invalid enabled fields return [`Error::InvalidData`].
  /// Invalid disabled fields become zero (or day 1 for a date alarm).
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  ///
  /// [`Error::InvalidData`] if an enabled alarm field contains invalid
  /// BCD or an out-of-range value. Invalid disabled fields are replaced
  /// with zero (or day 1 for a date alarm).
  pub fn get_alarm_parts(&mut self) -> Result<AlarmParts, Error<I2C::Error>> {

    self.select_mux_channel()?;

    let raw_day = self.read_register_raw(REG_WEEKDAY_DATE_ALARM)?;
    let match_day = 0 == (raw_day & ALARM_NO_WATCH_FLAG);
    let day = Self::checked_bcd(0x7F & raw_day);

    let raw_hour = self.read_register_raw(REG_HOURS_ALARM)?;
    let match_hour = 0 == (raw_hour & ALARM_NO_WATCH_FLAG);
    let hour = Self::checked_bcd(0x7F & raw_hour);

    let raw_minutes = self.read_register_raw(REG_MINUTES_ALARM)?;
    let match_minutes = 0 == (raw_minutes & ALARM_NO_WATCH_FLAG);
    let minutes = Self::checked_bcd(0x7F & raw_minutes);

    let wada_state = self.read_register_raw(REG_CONTROL1)? & RegControl1Bits::WadaBit as u8;

    // Disabled fields may contain unusable calendar values. Preserve valid
    // values, but substitute a valid placeholder for an invalid ignored field.
    let weekday_alarm = wada_state == 0;
    let day = day.and_then(|v| {
      if (weekday_alarm && v <= 6) || (!weekday_alarm && (1..=31).contains(&v)) {
        Ok(v)
      } else { Err(Error::InvalidData) }
    });
    let hour = hour.and_then(|v| if v <= 23 { Ok(v) } else { Err(Error::InvalidData) });
    let minutes = minutes.and_then(|v| if v <= 59 { Ok(v) } else { Err(Error::InvalidData) });
    Ok(AlarmParts {
      day: if match_day { day? } else { day.unwrap_or(if weekday_alarm { 0 } else { 1 }) },
      hour: if match_hour { hour? } else { hour.unwrap_or(0) },
      minute: if match_minutes { minutes? } else { minutes.unwrap_or(0) },
      weekday_alarm: 0 == wada_state,
      match_day,
      match_hour,
      match_minute: match_minutes,
    })
  }



  // If `set` is true, set the high bits given in `bits`, otherwise clear those bits
  fn set_or_clear_reg_bits(&mut self, reg: u8, bits: u8, set: bool) -> Result<(), Error<I2C::Error>> {
    self.select_mux_channel()?;
    self.set_or_clear_reg_bits_raw(reg, bits, set)
  }

  fn set_or_clear_reg_bits_raw(&mut self, reg: u8, bits: u8, set: bool) -> Result<(), Error<I2C::Error>> {
    if set {
      self.set_reg_bits_raw(reg, bits)
    }
    else {
      self.clear_reg_bits_raw(reg, bits)
    }
  }


  /// Enables or disables default CLKOUT behavior
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  pub fn toggle_plain_clockout(&mut self, enable: bool) -> Result<(), Error<I2C::Error>> {
    self.select_mux_channel()?;
    // TODO self.clear_reg_bits_raw(REG_STATUS, RegStatusBits::ClockIntFlagBit as u8)?;
    self.set_or_clear_reg_bits_raw(
      EEPROM_MIRROR_ADDRESS, RegEepromMirrorBits::ClockoutOutputEnableBit as u8, enable)
  }

  /// Enables or disables interrupt-controlled CLKOUT
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  pub fn toggle_int_clockout(&mut self, enable: bool)  -> Result<(), Error<I2C::Error>> {
    self.select_mux_channel()?;
    self.set_or_clear_reg_bits_raw(
    REG_CONTROL2,  RegControl2Bits::ClockoutIntEnableBit as u8, enable)
  }

  // Configure the Periodic Countdown Timer prior to the next countdown.
  fn config_pct_raw(&mut self, value: u16, freq: TimerClockFreq, repeat: bool ) -> Result<(), Error<I2C::Error>> {
    let value_high: u8 = ((value >> 8) as u8) & 0x0F;
    let value_low: u8 = (value & 0xFF) as u8;

    // configure the timer clock source / period
    self.clear_reg_bits_raw(REG_CONTROL1, RegControl1Bits::TimerEnableBit as u8)?;
    self.set_or_clear_reg_bits_raw(REG_CONTROL1, RegControl1Bits::TimerRepeatBit as u8, repeat)?;

    self.clear_reg_bits_raw(REG_CONTROL1, RegControl1Bits::TimerClockFreqBits as u8)?;
    self.set_reg_bits_raw(REG_CONTROL1, freq as u8)?; //TODO verify

    // write to REG_TIMER_VALUE0 and REG_TIMER_VALUE1
    let write_buf = [ REG_TIMER_VALUE0, value_low, value_high];
    self.i2c.write(RV3028_ADDRESS, &write_buf).map_err(Error::I2c)?;

    self.clear_reg_bits_raw(REG_STATUS, RegStatusBits::PeriodicTimerFlag as u8)?;
    Ok(())
  }

  const MAX_PCT_TICKS: u16 = 0x0FFF; // 4095
  const PCT_MILLIS_PERIOD:i64 = 15; // 15.625 ms period
  const PCT_MICROS_PERIOD:i64 = 244; // 244.14 μs period

  const MAX_PCT_COUNT:i64 = Self::MAX_PCT_TICKS as i64;
  const MAX_PCT_MILLIS:i64 = Self::MAX_PCT_COUNT * Self::PCT_MILLIS_PERIOD;
  const MAX_PCT_MICROS:i64 = Self::MAX_PCT_COUNT * Self::PCT_MICROS_PERIOD;
  const PCT_MILLIS_SECOND_BARRIER: i64 =  Self::PCT_MILLIS_PERIOD*(1000/Self::PCT_MILLIS_PERIOD);

  // Calculate the closest clock frequency and
  // number of ticks to match the requested duration using the
  // Periodic Countdown Timer (PCT)
  fn pct_ticks_and_rate_for_duration(duration: &Duration) -> (u16, TimerClockFreq, Duration)
  {
    // saturate rather than overflow: anything this large is clamped to the maximum below
    let whole_seconds = i64::try_from(duration.as_secs()).unwrap_or(i64::MAX);
    let whole_minutes = whole_seconds / 60;
    let whole_milliseconds = i64::try_from(duration.as_millis()).unwrap_or(i64::MAX);
    let frac_milliseconds = whole_milliseconds % 1_000;
    let infrac_milliseconds = whole_milliseconds % Self::PCT_MILLIS_PERIOD;
    let whole_microseconds = i64::try_from(duration.as_micros()).unwrap_or(i64::MAX);

    return if whole_minutes >= Self::MAX_PCT_COUNT {
      (Self::MAX_PCT_TICKS, TimerClockFreq::HertzSixtieth, Duration::from_secs(Self::MAX_PCT_COUNT as u64 * 60))
    } else if whole_seconds > Self::MAX_PCT_COUNT {
      // use minutes
      let ticks = whole_minutes;
      (ticks as u16, TimerClockFreq::HertzSixtieth, Duration::from_secs(ticks as u64 * 60))
    } else if  (whole_milliseconds > Self::MAX_PCT_MILLIS) ||
      ((0 == frac_milliseconds) && (whole_milliseconds > Self::PCT_MILLIS_SECOND_BARRIER))  {
      // use seconds
      let ticks = whole_seconds;
      (ticks as u16, TimerClockFreq::Hertz1, Duration::from_secs(ticks as u64))
    } else if (whole_microseconds > Self::MAX_PCT_MICROS) ||
      ((0 == infrac_milliseconds) && (whole_milliseconds >= Self::PCT_MILLIS_PERIOD)) {
      // use milliseconds
      let ticks = whole_milliseconds / Self::PCT_MILLIS_PERIOD;
      (ticks as u16, TimerClockFreq::Hertz64,
       Duration::from_millis((ticks * Self::PCT_MILLIS_PERIOD) as u64))
    } else {
      // use microseconds
      let ticks = whole_microseconds / Self::PCT_MICROS_PERIOD;
      (ticks as u16, TimerClockFreq::Hertz4096,
       Duration::from_micros((ticks * Self::PCT_MICROS_PERIOD) as u64))
    }

  }

  /// Prepare the Periodic Countdown Timer for a countdown,
  /// and optionally start the countdown.
  ///
  /// - `repeat`: If true, the countdown timer will repeat as a periodic timer.
  /// If false, the countdown timer will only run once ("one-shot" mode).
  /// Returns the estimated actual duration (which may vary from the requested duration
  /// dur to discrete RTC clock ticks).
  /// - `start`: If true, start the countdown
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  pub fn config_countdown_timer(&mut self, duration: &Duration,
                                repeat: bool, start: bool
  ) -> Result<Duration, Error<I2C::Error>> {
    let (ticks, freq, estimated) =
      Self::pct_ticks_and_rate_for_duration(duration);

    self.select_mux_channel()?;
    self.config_pct_raw(ticks, freq, repeat)?;
    if start {
      self.set_reg_bits_raw(REG_CONTROL1, RegControl1Bits::TimerEnableBit as u8)?;
    }

    Ok(estimated)
  }

  /// Set whether the Periodic Countdown Timer mode is repeating (periodic) or one-shot.
  /// - `enable`: If true, starts the timer countdown. If false, stops the timer.
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  pub fn toggle_countdown_timer(&mut self, enable: bool)  -> Result<(), Error<I2C::Error>> {
    self.set_or_clear_reg_bits(
      REG_CONTROL1, RegControl1Bits::TimerEnableBit as u8, enable)
  }

  /// Check whether countdown timer has finished counting down, and clear it
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  pub fn check_and_clear_countdown(&mut self) -> Result<bool, Error<I2C::Error>> {
    let flag_set = 0 != self.check_and_clear_bits(
      REG_STATUS, RegStatusBits::PeriodicTimerFlag as u8)?;
    Ok(flag_set)
  }

  /// Read the current value of the Periodic Countdown Timer,
  /// which is only valid after the timer has been enabled.
  /// The meaning of the value depends on the configured TimerClockFreq
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  pub fn get_countdown_value(&mut self) -> Result<u16, Error<I2C::Error>> {
    let mut read_buf = [0u8;2];
    self.read_multi_registers(REG_TIMER_STATUS0, &mut read_buf)?;
    let value = ((read_buf[1] as u16) << 8) | (read_buf[0] as u16);
    Ok(value)
  }

  // check and clear a flag
  fn check_and_clear_bits(&mut self, reg: u8, bits: u8) -> Result<u8, Error<I2C::Error>> {
    self.select_mux_channel()?;
    let reg_val = self.read_register_raw(reg)?;
    let bits_val =  reg_val & bits;
    if 0 != bits_val {
      self.clear_reg_bits_raw(reg, bits)?;
    }
    Ok(bits_val)
  }


  /// Configure event detection on the EVI pin
  /// - `rising` whether edge detection is on rising edge / high level
  /// - `int_enable` whether events detected on EVI pin should generate an interrupt on INT pin
  /// - `filtering` 00..11 time filtering
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  pub fn config_ext_event_detection(
    &mut self, rising: bool, int_enable: bool, filtering: u8, clockout_enable: bool) -> Result<(), Error<I2C::Error>>
  {
    self.select_mux_channel()?;

    // 1. Initialize bits TSE and EIE to 0.
    // 2. Clear flag EVF to 0.
    // 4. Set EHL bit to 1 or 0 to choose high or low level (or rising or falling edge) detection on pin EVI.
    // 5. Select EDGE DETECTION (ET = 00) or LEVEL DETECTION WITH FILTERING (ET ≠ 00).
    // 8. Set CEIE bit to 1 to enable clock output when external event occurs. See also CLOCK OUTPUT SCHEME.
    // 10. Set EIE bit to 1 if you want to get a hardware interrupt on INT̅ ̅ ̅ ̅ ̅
    // pin.

    // Pause listening for external events on EVI pin
    // 1. Initialize EIE to 0.
    self.clear_reg_bits_raw(REG_CONTROL2,
                              RegControl2Bits::EventIntEnableBit  as u8)?;
    // 2. Clear flag EVF to 0.
    self.clear_reg_bits_raw(
      REG_STATUS, RegStatusBits::EventFlagBit as u8)?;

    // 4. Set EHL bit to 1 or 0 to choose high or low level
    // (or rising or falling edge) detection on pin EVI.
    self.set_or_clear_reg_bits_raw(
      REG_EVENT_CONTROL, RegEventControlBits::EventHighLowBit as u8, rising)?;

    // 5. Select EDGE DETECTION (ET = 00) or LEVEL DETECTION WITH FILTERING (ET ≠ 00).
    self.clear_reg_bits_raw(REG_EVENT_CONTROL,RegEventControlBits::EventFilteringTimeBits as u8)?;
    if 0 != filtering {
      // TODO verify this sets the correct filtering
      self.set_reg_bits_raw(REG_EVENT_CONTROL, filtering << 4)?;
    }

    // 8. Set CEIE bit to 1 to enable clock output when external event occurs.
    // See also CLOCK OUTPUT SCHEME.
    self.set_or_clear_reg_bits_raw(
      REG_CLOCK_INTERRUPT_MASK, RegClockIntMaskBits::ClockoutOnExtEvtBit as u8, clockout_enable)?;

    // 10. Set EIE bit to 1 if you want to get a hardware interrupt on INT pin
    self.set_or_clear_reg_bits_raw(
      REG_CONTROL2, RegControl2Bits::EventIntEnableBit as u8, int_enable)?;

    Ok(())
  }

}


/// Time stamp logging of events: external events on the EVI pin,
/// or switchovers to the backup power supply.
///
/// Reading the logged time stamp needs a date/time representation, so it is provided by
/// [`RV3028::get_event_count_and_parts`] and by the date/time backend traits.
pub trait EventTimeStampLogger {
  /// Error type
  type Error;

  /// Enable or disable the Time Stamp Function for event logging
  /// This logs external interrupts or other events
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  fn toggle_timestamp_logging(&mut self, enable: bool) -> Result<(), Self::Error>;

  /// clear out any existing logged event timestamps
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  fn reset_timestamp_log(&mut self) -> Result<(), Self::Error>;

  /// Setup time stamp logging for events
  /// - `evt_source` source for timestamp events, eg TS_EVENT_SOURCE_BSF
  /// - `overwrite` Save the most recent event timestamp?
  /// - `start` Should event timestamp logging immediately start?
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  fn config_timestamp_logging(
    &mut self, evt_source: u8, overwrite: bool,   start: bool)
    -> Result<(), Self::Error>;

  /// Enable or disable event time stamp overwriting
  /// If this is disabled (default), the first event time stamp is saved.
  /// If this is enabled, the most recent event time stamp is saved.
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  fn toggle_time_stamp_overwrite(&mut self, enable: bool) -> Result<(), Self::Error>;

  /// Select a source for events to be logged, device-specific
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  fn set_event_timestamp_source(&mut self, source: u8) -> Result<(), Self::Error>;
}

impl<I2C> EventTimeStampLogger for  RV3028<I2C>
  where
    I2C: I2c,
{
  type Error = Error<I2C::Error>;


  fn toggle_timestamp_logging(&mut self, enable: bool) -> Result<(), Self::Error> {
    self.select_mux_channel()?;
    self.set_or_clear_reg_bits_raw(REG_CONTROL2, RegControl2Bits::TimeStampEnableBit as u8, enable)
  }

  fn reset_timestamp_log(&mut self) -> Result<(), Self::Error> {
    self.select_mux_channel()?;
    self.set_reg_bits_raw(
      REG_EVENT_CONTROL, RegEventControlBits::TimeStampResetBit as u8)

  }

  fn config_timestamp_logging(
    &mut self, evt_source: u8, overwrite: bool,  start:bool) -> Result<(), Self::Error>
  {
    self.select_mux_channel()?;

    // Pause listening for events
    // 1. Initialize bits TSE to 0.
    self.clear_reg_bits_raw(REG_CONTROL2,
                            RegControl2Bits::TimeStampEnableBit as u8)?;

    // 2. Clear EVF and BSF
    self.clear_reg_bits_raw(
      REG_STATUS, RegStatusBits::EventFlagBit as u8 | RegStatusBits::BackupSwitchFlag as u8)?;

    // 3. Set TSS bit to
    // External Event Interrupt function (TSS = 0) or the
    // Automatic Backup Switchover Interrupt function (TSS = 1)
    // as time stamp source and initialize the appropriate function
    let enable_bsf = evt_source == TS_EVENT_SOURCE_BSF;
    self.set_or_clear_reg_bits_raw(
      REG_EVENT_CONTROL, RegEventControlBits::TimeStampSourceBit as u8, enable_bsf)?;

    // 6. Set TSOW bit to 1 if the last occurred event has to be recorded and TS registers are overwritten.
    self.set_or_clear_reg_bits_raw(
      REG_EVENT_CONTROL, RegEventControlBits::TimeStampOverwriteBit as u8, overwrite)?;

    // 7. Write 1 to TSR bit, to clear all Time Stamp registers to 0x00
    self.set_reg_bits_raw(
      REG_EVENT_CONTROL, RegEventControlBits::TimeStampResetBit as u8)?;

    // 9. Set TSE bit to 1 if you want to enable the Time Stamp function.
    // see also: toggle_timestamp_logging
    self.set_or_clear_reg_bits_raw(
      REG_CONTROL2, RegControl2Bits::TimeStampEnableBit as u8, start)?;

    // 1. Initialize bits TSE and EIE to 0.
    // 2. Clear flag EVF and BSF to 0.
    // 3. Set TSS bit to 0 to select External Event on EVI pin as Time Stamp and Interrupt source.
    // 6. Set TSOW bit to 1 to record the last occurred event (and TS registers are overwritten).
    // 7. Write 1 to TSR bit, to reset all Time Stamp registers to 00h. Bit TSR always returns 0 when read.
    // 9. Set TSE bit to 1 if you want to enable the Time Stamp function.
    // pin.
    Ok(())
  }

  fn toggle_time_stamp_overwrite(&mut self, enable: bool) -> Result<(), Self::Error> {
    self.set_or_clear_reg_bits(
      REG_EVENT_CONTROL, RegEventControlBits::TimeStampOverwriteBit as u8, enable)
  }

  fn set_event_timestamp_source(&mut self, source: u8) -> Result<(), Self::Error> {
    let enable = TS_EVENT_SOURCE_BSF == source;
    self.set_or_clear_reg_bits(
      REG_EVENT_CONTROL, RegEventControlBits::TimeStampSourceBit as u8, enable)
  }

}

#[cfg(test)]
mod tests {
  use super::*;
  use embedded_hal_mock::eh1::i2c::{Mock as I2cMock, Transaction as I2cTrans};
  use std::vec;


  type TestClass = RV3028<I2cMock>;

  fn mins(n: i64) -> Duration { Duration::from_secs(n as u64 * 60) }
  fn secs(n: i64) -> Duration { Duration::from_secs(n as u64) }
  fn millis(n: i64) -> Duration { Duration::from_millis(n as u64) }
  fn micros(n: i64) -> Duration { Duration::from_micros(n as u64) }


  #[test]
  fn test_set_unix_time() {
    let unix_time: u32 = 1_614_456_789; // Example Unix time
    let bytes = unix_time.to_le_bytes(); // Convert to little-endian byte array
    let expectations = [
      I2cTrans::write(
        RV3028_ADDRESS,
        vec![
          REG_UNIX_TIME_0,
          bytes[0],
          bytes[1],
          bytes[2],
          bytes[3],
        ],
      ),
    ];
    let mock = I2cMock::new(&expectations);
    let mut rv3028 = RV3028::new(mock);
    rv3028.set_unix_time(unix_time).unwrap();
    rv3028.i2c.done();
  }

  #[test]
  fn test_get_unix_time() {
    let unix_time: u32 = 1_614_456_789; // Example Unix time
    let bytes = unix_time.to_le_bytes(); // Convert to little-endian byte array
    let expectations = [
      I2cTrans::write_read(
        RV3028_ADDRESS,
        vec![REG_UNIX_TIME_0],
        bytes.to_vec(),
      ),
      I2cTrans::write_read(
        RV3028_ADDRESS,
        vec![REG_UNIX_TIME_0],
        bytes.to_vec(),
      ),
    ];
    let mock = I2cMock::new(&expectations);
    let mut rv3028 = RV3028::new(mock);
    assert_eq!(rv3028.get_unix_time_blocking().unwrap(), unix_time);
    rv3028.i2c.done();
  }

  // The duration requested should exactly match the duration the RTC can deliver with
  // pct_ticks_and_rate_for_duration
  fn verify_whole_time_estimate(duration: &Duration, known_freq: TimerClockFreq, known_ticks: u16) {
    let (ticks, freq, estimated) =
      TestClass::pct_ticks_and_rate_for_duration(&duration);
    assert_eq!(freq, known_freq);
    assert_eq!(ticks, known_ticks);
    assert_eq!(*duration, estimated);
  }

  // We know that the RTC can't precisely match the requested duration with
  // pct_ticks_and_rate_for_duration, so just match ticks and freq
  fn verify_ticks_and_freq(duration: &Duration, known_freq: TimerClockFreq, known_ticks: u16) {
    let (ticks, freq, _estimated) =
      TestClass::pct_ticks_and_rate_for_duration(&duration);
    assert_eq!(freq, known_freq);
    assert_eq!(ticks, known_ticks);
    // assert_eq!(*duration, estimated); // TODO calculate
  }

  #[test]
  fn test_countdown_timer_conversion_minutes() {
    // should be fulfilled as minutes
    let minutes_clock_freq = TimerClockFreq::HertzSixtieth;

    // request a longer countdown than th RTC can fulfill
    verify_ticks_and_freq(
      &mins(TestClass::MAX_PCT_COUNT + 32),
      minutes_clock_freq, TestClass::MAX_PCT_TICKS);

    verify_whole_time_estimate(
      &mins(TestClass::MAX_PCT_COUNT),
      minutes_clock_freq, TestClass::MAX_PCT_TICKS);

    // exceed the Seconds counter just slightly to invoke Minutes counter
    const MAX_SECONDS_IN_MINUTES: i64 = (TestClass::MAX_PCT_COUNT/ 60) + 1;
    verify_whole_time_estimate(
      &mins(MAX_SECONDS_IN_MINUTES),
      minutes_clock_freq, MAX_SECONDS_IN_MINUTES as u16);

    verify_whole_time_estimate(
      &mins(2047),
      minutes_clock_freq, 2047);

  }

  #[test]
  fn test_countdown_timer_conversion_seconds() {
    // should be fulfilled as seconds
    let seconds_clock_freq = TimerClockFreq::Hertz1;

    // Maximum seconds ticks
    verify_whole_time_estimate(
      &secs(TestClass::MAX_PCT_COUNT),
      seconds_clock_freq, TestClass::MAX_PCT_TICKS);

    verify_whole_time_estimate(
      &secs(2047),
      seconds_clock_freq, 2047);

    verify_whole_time_estimate(
      &secs(61),
      seconds_clock_freq, 61);

    // we serve whole minutes (under max seconds) with a seconds countdown
    verify_whole_time_estimate(
      &secs(60),
      seconds_clock_freq, 60);

    verify_whole_time_estimate(
      &mins(1),
      seconds_clock_freq, 60);

    verify_whole_time_estimate(
      &mins(45),
      seconds_clock_freq, 45*60);

    // minimum Seconds ticks
    verify_whole_time_estimate(
      &secs(1),
      seconds_clock_freq, 1);

  }

  #[test]
  fn test_countdown_timer_conversion_micros() {
    // should be fulfilled as microseconds
    let micros_clock_freq = TimerClockFreq::Hertz4096;

    verify_whole_time_estimate(
      &micros(TestClass::MAX_PCT_MICROS),
      micros_clock_freq, TestClass::MAX_PCT_TICKS);

    verify_ticks_and_freq(
      &micros(2048),
      micros_clock_freq, (2048 / TestClass::PCT_MICROS_PERIOD) as u16);

    verify_ticks_and_freq(
      &micros(655),
      micros_clock_freq, (655 / TestClass::PCT_MICROS_PERIOD) as u16);

    verify_ticks_and_freq(
      &micros(1024),
      micros_clock_freq, (1024 / TestClass::PCT_MICROS_PERIOD) as u16);

    // some exact micros values

    verify_whole_time_estimate(
      &micros(999*TestClass::PCT_MICROS_PERIOD),
      micros_clock_freq, 999);

    verify_whole_time_estimate(
      &micros(100*TestClass::PCT_MICROS_PERIOD),
      micros_clock_freq, 100);

    verify_whole_time_estimate(
      &micros(17*TestClass::PCT_MICROS_PERIOD),
      micros_clock_freq, 17);

    // minimum microseconds tick
    verify_whole_time_estimate(
      &micros(TestClass::PCT_MICROS_PERIOD),
      micros_clock_freq, 1);

  }

  #[test]
  fn test_countdown_timer_conversion_millis() {
    // should be fulfilled as milliseconds
    let millis_clock_freq = TimerClockFreq::Hertz64;

    // a bit more than max micros counter, but less than max millis counter
    verify_ticks_and_freq(
      &micros(TestClass::MAX_PCT_MICROS + 1),
      millis_clock_freq,
      ((TestClass::MAX_PCT_MICROS + 1) / (TestClass::PCT_MILLIS_PERIOD * 1_000)) as u16
    );

    // maximum millis counter
    verify_whole_time_estimate(
      &millis(TestClass::MAX_PCT_MILLIS),
      millis_clock_freq, TestClass::MAX_PCT_TICKS);

    // mid-value millis
    verify_ticks_and_freq(
      &millis(2047),
      millis_clock_freq, (2047 / TestClass::PCT_MILLIS_PERIOD) as u16);

    // exactly on the seconds clock
    verify_whole_time_estimate(
      &millis(1000*TestClass::PCT_MILLIS_PERIOD),
      TimerClockFreq::Hertz1, TestClass::PCT_MILLIS_PERIOD as u16 );

    // exactly on the millis period

    verify_whole_time_estimate(
      &millis(999*TestClass::PCT_MILLIS_PERIOD),
      millis_clock_freq, 999);

    verify_whole_time_estimate(
      &millis(100*TestClass::PCT_MILLIS_PERIOD),
      millis_clock_freq, 100);

    // minimum millis ticks
    verify_whole_time_estimate(
      &millis(TestClass::PCT_MILLIS_PERIOD),
      millis_clock_freq, 1);

  }

  // 2024-03-15 13:45:30 UTC, a Friday
  const TEST_PARTS: DateTimeParts =
    DateTimeParts { year: 2024, month: 3, day: 15, hour: 13, minute: 45, second: 30 };
  const TEST_UNIX_TIME: u32 = 1_710_510_330;

  #[test]
  fn test_set_datetime_parts() {
    let unix_bytes = TEST_UNIX_TIME.to_le_bytes();
    let expectations = [
      I2cTrans::write(RV3028_ADDRESS, vec![
        REG_UNIX_TIME_0, unix_bytes[0], unix_bytes[1], unix_bytes[2], unix_bytes[3]]),
      // weekday (Friday = 4), day, month, year
      I2cTrans::write(RV3028_ADDRESS, vec![REG_WEEKDAY, 0x04, 0x15, 0x03, 0x24]),
      // seconds, minutes, hours
      I2cTrans::write(RV3028_ADDRESS, vec![REG_SECONDS, 0x30, 0x45, 0x13]),
    ];
    let mut rv3028 = RV3028::new(I2cMock::new(&expectations));
    rv3028.set_datetime_parts(&TEST_PARTS).unwrap();
    rv3028.i2c.done();
  }

  // Check the unix timestamp and weekday calculation against chrono
  // for every day the RTC's date registers support
  #[test]
  fn test_parts_unix_timestamp_and_weekday() {
    use chrono::{Datelike, NaiveDate};
    let mut date = NaiveDate::from_ymd_opt(2000, 1, 1).unwrap();
    let last = NaiveDate::from_ymd_opt(2099, 12, 31).unwrap();
    while date <= last {
      let parts = DateTimeParts {
        year: date.year(), month: date.month() as u8, day: date.day() as u8,
        hour: 23, minute: 59, second: 58,
      };
      let expected = date.and_hms_opt(23, 59, 58).unwrap().and_utc().timestamp();
      assert_eq!(parts.unix_timestamp(), expected, "{date}");
      assert_eq!(parts.weekday(), date.weekday().num_days_from_monday() as u8, "{date}");
      date = date.succ_opt().unwrap();
    }
  }

  #[test]
  fn test_get_alarm_parts() {
    let expectations = [
      I2cTrans::write_read(RV3028_ADDRESS, vec![REG_WEEKDAY_DATE_ALARM], vec![0x82]),
      I2cTrans::write_read(RV3028_ADDRESS, vec![REG_HOURS_ALARM], vec![0x13]),
      I2cTrans::write_read(RV3028_ADDRESS, vec![REG_MINUTES_ALARM], vec![0x45]),
      I2cTrans::write_read(RV3028_ADDRESS, vec![REG_CONTROL1], vec![0x00]),
    ];
    let mut rv3028 = RV3028::new(I2cMock::new(&expectations));
    assert_eq!(rv3028.get_alarm_parts().unwrap(), AlarmParts {
      day: 2, hour: 13, minute: 45, weekday_alarm: true,
      match_day: false, match_hour: true, match_minute: true,
    });
    rv3028.i2c.done();
  }

  #[test]
  fn test_get_event_count_and_parts() {
    let expectations = [
      // count, seconds, minutes, hours, date, month, year
      I2cTrans::write_read(RV3028_ADDRESS, vec![REG_COUNT_EVENTS_TS],
                           vec![3, 0x30, 0x45, 0x13, 0x15, 0x03, 0x24]),
    ];
    let mut rv3028 = RV3028::new(I2cMock::new(&expectations));
    assert_eq!(rv3028.get_event_count_and_parts().unwrap(), (3, Some(TEST_PARTS)));
    rv3028.i2c.done();
  }

  #[test]
  fn counter_retry_limit_and_last_read_success() {
    for values in [[1u32, 2, 3, 4], [1, 2, 3, 3]] {
      let expectations = values.map(|v|
        I2cTrans::write_read(RV3028_ADDRESS, vec![REG_UNIX_TIME_0], v.to_le_bytes().to_vec()));
      let mut rtc = RV3028::new(I2cMock::new(&expectations));
      let expected = if values[3] == 3 { Ok(3) } else { Err(Error::InconsistentRead) };
      assert_eq!(rtc.get_unix_time_blocking(), expected);
      rtc.i2c.done();
    }
  }

  #[test]
  fn bus_errors_are_preserved() {
    use embedded_hal::i2c::ErrorKind;
    let expectations = [I2cTrans::write_read(RV3028_ADDRESS, vec![REG_UNIX_TIME_0], vec![0; 4])
      .with_error(ErrorKind::Bus)];
    let mut rtc = RV3028::new(I2cMock::new(&expectations));
    assert_eq!(rtc.get_unix_time_blocking(), Err(Error::I2c(ErrorKind::Bus)));
    rtc.i2c.done();
    let expectations = [I2cTrans::write(0x70, vec![1]).with_error(ErrorKind::ArbitrationLoss)];
    let mut rtc = RV3028::new_with_mux(I2cMock::new(&expectations), 0x70, 1);
    assert_eq!(rtc.set_datetime_parts(&TEST_PARTS), Err(Error::I2c(ErrorKind::ArbitrationLoss)));
    rtc.i2c.done();
  }

  #[test]
  fn invalid_inputs_do_not_access_bus() {
    let mut rtc = RV3028::new_with_mux(I2cMock::new(&[]), 0x70, 1);
    for dt in [
      DateTimeParts { year: 1999, ..TEST_PARTS },
      DateTimeParts { year: 2100, ..TEST_PARTS },
      DateTimeParts { month: 0, ..TEST_PARTS },
      DateTimeParts { month: 4, day: 31, ..TEST_PARTS },
      DateTimeParts { year: 2023, month: 2, day: 29, ..TEST_PARTS },
      DateTimeParts { day: 0, ..TEST_PARTS },
      DateTimeParts { hour: 24, ..TEST_PARTS },
      DateTimeParts { minute: 60, ..TEST_PARTS },
      DateTimeParts { second: 60, ..TEST_PARTS },
    ] {
      assert_eq!(rtc.set_datetime_parts(&dt), Err(Error::InvalidData));
    }
    let alarm = AlarmParts { day: 1, hour: 0, minute: 0, weekday_alarm: false,
      match_day: true, match_hour: true, match_minute: true };
    for invalid in [AlarmParts { day: 0, ..alarm }, AlarmParts { hour: 24, ..alarm },
      AlarmParts { minute: 60, ..alarm }, AlarmParts { day: 7, weekday_alarm: true, ..alarm }] {
      assert_eq!(rtc.set_alarm_parts(&invalid), Err(Error::InvalidData));
    }
    rtc.i2c.done();
  }

  #[test]
  fn invalid_event_data_returns_error() {
    // Invalid BCD which would otherwise decode to a valid 10 seconds,
    // and a correctly encoded but impossible February 30.
    for bytes in [vec![1, 0x0A, 0, 0, 1, 1, 0x24], vec![1, 0, 0, 0, 0x30, 2, 0x24]] {
      let expectations = [I2cTrans::write_read(RV3028_ADDRESS, vec![REG_COUNT_EVENTS_TS], bytes)];
      let mut rtc = RV3028::new(I2cMock::new(&expectations));
      assert_eq!(rtc.get_event_count_and_parts(), Err(Error::InvalidData));
      rtc.i2c.done();
    }
  }

  #[test]
  fn invalid_alarm_fields_are_only_ignored_when_disabled() {
    for disabled in [false, true] {
      let flag = if disabled { 0x80 } else { 0 };
      let expectations = [
        I2cTrans::write_read(RV3028_ADDRESS, vec![REG_WEEKDAY_DATE_ALARM], vec![flag]),
        I2cTrans::write_read(RV3028_ADDRESS, vec![REG_HOURS_ALARM], vec![flag | 0x24]),
        I2cTrans::write_read(RV3028_ADDRESS, vec![REG_MINUTES_ALARM], vec![flag | 0x6A]),
        I2cTrans::write_read(RV3028_ADDRESS, vec![REG_CONTROL1], vec![0x20]),
      ];
      let mut rtc = RV3028::new(I2cMock::new(&expectations));
      let result = rtc.get_alarm_parts();
      if disabled {
        assert_eq!(result, Ok(AlarmParts { day: 1, hour: 0, minute: 0, weekday_alarm: false,
          match_day: false, match_hour: false, match_minute: false }));
      } else { assert_eq!(result, Err(Error::InvalidData)); }
      rtc.i2c.done();
    }
  }

}
