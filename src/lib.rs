#![cfg_attr(not(test), no_std)]


pub use chrono::{Datelike, NaiveDate, NaiveDateTime, Timelike, Weekday};
use chrono::{Duration, NaiveTime};
pub use rtcc::{  DateTimeAccess };

use embedded_hal::blocking::i2c::{Write, Read, WriteRead};

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
const TIMER_REPEAT_BIT: u8 = 1 << 7;// TRPT / Timer Repeat bit. Single or Repeat countdown timer
const WADA_BIT: u8 = 1 << 5; // WADA / Weekday Alarm / Date Alarm selection bit
// const USEL_BIT: u8 = 1 << 4;
// const EERD_BIT: u8 = 1 << 3;
const TIMER_ENABLE_BIT: u8 = 1 << 2; // TE / Periodic Countdown Timer Enable bit.
const TIMER_CLOCK_FREQ_BITS: u8 = 0b11; // TD / Timer Clock Frequency selection bits

/// Countown timer clock frequency selector
#[derive(Clone, Copy, Debug, PartialEq)]
enum TimerClockFreq {
  Hertz4096 = 0b00, // 4096 Hz, 244.14 μs period
  Hertz64 = 0b01, // 64 Hz, 15.625 ms period
  Hertz1 = 0b10, // 1 Hz, One second period
  HertzSixtieth = 0b11, // 1/60 Hz, One minute period
}

// REG_STATUS Status register bits:
// const EEBUSY_BIT: u8 = 1 << 7;
const CLOCK_INT_FLAG_BIT: u8 = 1 << 6; // CLKF  / Clock Output Interrupt Flag
const BACKUP_SWITCH_FLAG: u8 = 1 << 4; // BSF bit
const PERIODIC_TIMER_FLAG: u8 = 1 << 3; // TF bit / Periodic Countdown Timer Flag
const ALARM_FLAG_BIT : u8 = 1 << 2; // AF / Alarm Flag
const EVENT_FLAG_BIT: u8 = 1 << 1; // EVF / Event Flag (external event interrupt)



// EEPROM register addresses and commands
const EEPROM_MIRROR_ADDRESS: u8 = 0x37;// RAM mirror of EEPROM config values
// const EEPROM_CMD_READ: u8 = 0x00;
// const EEPROM_CMD_WRITE: u8 = 0x01;


// REG_EVENT_CONTROL Event Control register bits:   EHL, ET, TSR, TSOW, TSS
const EVENT_HIGH_LOW_BIT: u8 = 1 << 6; // EHL bit
const TIME_STAMP_RESET_BIT: u8 = 1 << 2; // TSR bit
const TIME_STAMP_OVERWRITE_BIT: u8 = 1 << 1; // TSOW bit
const TIME_STAMP_SOURCE_BIT: u8 = 1 << 0; // TSS bit

pub const TS_EVENT_SOURCE_EVI: u8 = 0; /// Event log source is external interrupt EVI (default)
pub const TS_EVENT_SOURCE_BSF: u8 = 1; /// Event log source is backup power switchover

// REG_CONTROL2 "Control 2" register bits: TSE CLKIE UIE TIE AIE EIE 12_24 RESET
const TIME_STAMP_ENABLE_BIT: u8 = 1 << 7; // TSE / Time Stamp Enable bit
const TIME_UPDATE_INT_ENABLE_BIT: u8 = 1 << 5; // UIE / Time Update Interrupt Enable
const TIMER_INT_ENABLE_BITL: u8 = 1 << 4;// TIE Countdown Timer Interrupt Enable bit
const ALARM_INT_ENABLE_BIT: u8 = 1 << 3;// AIE / Alarm Interrupt Enable bit
const EVENT_INT_ENABLE_BIT: u8 = 1 << 2;// EIE / Event Interrupt Enable bit

// EEPROM_MIRROR_ADDRESS / EEPROM mirror register bits:
const CLOCK_OUT_ENABLE_BIT:u8 = 1<< 7; // CLKOE / CLKOUT Enable bit
const BACKUP_SWITCH_INT_ENABLE_BIT:u8 = 1 << 6; // BCIE / Backup Switchover Interrupt Enable bit bit
const TRICKLE_CHARGE_ENABLE_BIT: u8 = 1 << 5; // TCE bit
const BACKUP_SWITCHOVER_BITS:  u8  = 0b11 << 2; // Backup Switchover Mode / BSM bits
const BACKUP_SWITCHOVER_DSM:  u8  = 0b01 << 2; // Backup Switchover Mode / BSM bits as DSM

const TRICKLE_CHARGE_RESISTANCE_BITS: u8 = 0b11; // TCR bits
#[derive(Clone, Copy)]
pub enum TrickleChargeCurrentLimiter {
  Ohms3k = 0b00,
  Ohms5k = 0b01,
  Ohms9k = 0b10,
  Ohms15k = 0b11,
}

// Special alarm register value
const ALARM_NO_WATCH_FLAG: u8 = 1 <<  7;


/// RV-3028-C7
/// Extreme Low Power Real-Time Clock (RTC) Module with I2C-Bus Interface
/// rust no_std driver (utilizes the embedded_hal i2c interface)
pub struct RV3028<I2C> {
  i2c: I2C,
  mux_addr: u8,
  mux_chan: u8,
}

impl<I2C, E> RV3028<I2C>
  where
    I2C: Write<Error = E> + Read<Error = E> + WriteRead<Error = E>,
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
  fn bin_to_bcd(value: u8) -> u8 {
    ((value / 10) << 4) | (value % 10)
  }

  // Converts a BCD value to binary format
  fn bcd_to_bin(value: u8) -> u8 {
    ((value & 0xF0) >> 4) * 10 + (value & 0x0F)
  }

  // If using an i2c mux, tell the mux to select our channel
  fn select_mux_channel(&mut self) -> Result<(), E> {
    if self.mux_addr != 0u8 {
      self.i2c.write(self.mux_addr, &[self.mux_chan])
    }
    else {
      Ok(())
    }
  }

  // fn write_register(&mut self, reg: u8, data: u8) -> Result<(), E> {
  //   self.select_mux_channel()?;
  //   self.write_register_raw(reg, data)
  // }

  fn write_register_raw(&mut self, reg: u8, data: u8) -> Result<(), E> {
    self.i2c.write(RV3028_ADDRESS, &[reg, data])
  }

  // fn read_register(&mut self, reg: u8) -> Result<u8, E> {
  //   self.select_mux_channel()?;
  //   self.read_register_raw(reg)
  // }

  fn read_register_raw(&mut self, reg: u8) -> Result<u8, E> {
    let mut buf = [0];
    self.i2c.write_read(RV3028_ADDRESS, &[reg], &mut buf)?;
    Ok(buf[0])
  }

  //clear_register


  // TODO these methods have not been thoroughly tested, and are believed broken.
  // fn is_eeprom_busy(&mut self) -> Result<bool, E> {
  //   let status = self.read_register(REG_STATUS)?;
  //   Ok(status & EEBUSY_BIT != 0)
  // }
  //
  // fn disable_auto_eeprom_refresh(&mut self) -> Result<(), E> {
  //   let mut control_1 = self.read_register(REG_CONTROL1)?;
  //   control_1 |= EERD_BIT; // Set EERD bit
  //   self.write_register(REG_CONTROL1, control_1)
  // }
  //
  // fn enable_auto_eeprom_refresh(&mut self) -> Result<(), E> {
  //   let mut control_1 = self.read_register(REG_CONTROL1)?;
  //   control_1 &= !(EERD_BIT); // Clear EERD bit
  //   self.write_register(REG_CONTROL1, control_1)
  // }
  //
  // pub fn eeprom_read(&mut self, address: u8) -> Result<u8, E> {
  //   self.disable_auto_eeprom_refresh()?;
  //   while self.is_eeprom_busy()? {}
  //   // Read from EEPROM
  //   self.write_register(EEPROM_ADDRESS, address)?;
  //   let res = self.read_register(EEPROM_ADDRESS);
  //   self.enable_auto_eeprom_refresh()?;
  //   res
  // }
  //
  // pub fn eeprom_write(&mut self, address: u8, data: u8) -> Result<(), E> {
  //   self.disable_auto_eeprom_refresh()?;
  //   while self.is_eeprom_busy()? {}
  //   self.write_register(EEPROM_ADDRESS, address)?;
  //   let res = self.write_register(EEPROM_ADDRESS, data);
  //
  //   self.enable_auto_eeprom_refresh()?;
  //   res
  // }

  // set specific bits in a register:
  // all bits must be high that you wish to set
  fn set_reg_bits(&mut self, reg: u8, bits: u8) -> Result<(), E> {
    self.select_mux_channel()?;
    self.set_reg_bits_raw(reg, bits)
  }

  // set specific bits in a register: skips the mux
  // all bits must be high that you wish to set
  fn set_reg_bits_raw(&mut self, reg: u8, bits: u8) -> Result<(), E> {
    let mut reg_val = self.read_register_raw(reg)?;
    reg_val |= bits; // Set bits that are high
    self.write_register_raw(reg, reg_val)
  }

  // clear specific bits in a register:
  // all bits must be high that you wish to be cleared
  fn clear_reg_bits(&mut self, reg: u8, bits: u8) -> Result<(), E> {
    self.select_mux_channel()?;
    self.clear_reg_bits_raw(reg,bits)
  }

  // clear specific bits in a register, skips the mux
  // all bits must be high that you wish to be cleared
  fn clear_reg_bits_raw(&mut self, reg: u8, bits: u8) -> Result<(), E> {
    let mut reg_val = self.read_register_raw(reg)?;
    reg_val &= !(bits); // Clear  bits that are high
    self.write_register_raw(reg, reg_val)
  }

  /// Enable or disable trickle charging
  /// - `enable` enables trickle charging if true, disables if false
  /// - `limit_resistance` Sets the current limiting resistor value: higher means less current
  /// Disabling also resets the `limit_resistance` to 3 kΩ, the factory default.
  /// Returns the status of trickle charging (true for enabled, false for disabled)
  pub fn toggle_trickle_charge(&mut self, enable: bool,
                               limit_resistance: TrickleChargeCurrentLimiter) -> Result<bool, E>  {
    self.select_mux_channel()?;

    // First disable charging before changing settings
    self.clear_reg_bits_raw(EEPROM_MIRROR_ADDRESS,  TRICKLE_CHARGE_ENABLE_BIT)?;
    // Reset TCR to 3 kΩ, the factory default, by clearing the TCR bits
    self.clear_reg_bits_raw(EEPROM_MIRROR_ADDRESS,  TRICKLE_CHARGE_RESISTANCE_BITS )?;

    if enable {
      self.set_reg_bits_raw(EEPROM_MIRROR_ADDRESS, limit_resistance as u8)?;
      self.set_reg_bits_raw(EEPROM_MIRROR_ADDRESS, TRICKLE_CHARGE_ENABLE_BIT)?;
    }

    // confirm the value set
    let conf_val =
      0 != self.read_register_raw(EEPROM_MIRROR_ADDRESS)? & TRICKLE_CHARGE_ENABLE_BIT;
    Ok(conf_val)
  }

  /// Toggle whether the Vbackup power source should be used
  /// when Vdd supply level drops below useful level.
  /// - `enable` enables switching to Vbackup, disables if false
  /// Returns the set value
  pub fn toggle_backup_switchover(&mut self, enable: bool) -> Result<bool, E> {
    self.select_mux_channel()?;
    self.set_or_clear_reg_bits_raw( EEPROM_MIRROR_ADDRESS, BACKUP_SWITCHOVER_DSM, enable)?;
    let conf_val =
      0 != self.read_register_raw(EEPROM_MIRROR_ADDRESS)? & BACKUP_SWITCHOVER_DSM;
    Ok(conf_val)
  }

  /// Get the current value of the EEPROM mirror from RAM
  pub fn get_eeprom_mirror_value(&mut self) -> Result<u8, E> {
    self.select_mux_channel()?;
    let reg_val = self.read_register_raw(EEPROM_MIRROR_ADDRESS)?;
    Ok(reg_val)
  }

  // Set the bcd time tracking registers
  // assumes `select_mux_channel` has already been called
  fn set_time_raw(&mut self, time: &NaiveTime) -> Result<(), E> {
    let write_buf = [
      REG_SECONDS, // select the first register
      Self::bin_to_bcd(time.second() as u8 ),
      Self::bin_to_bcd(time.minute() as u8 ),
      Self::bin_to_bcd(time.hour() as u8 )
    ];
    self.i2c.write(RV3028_ADDRESS, &write_buf)
  }


  // Set the internal BCD date registers.
  // Note that only years from 2000 to 2099 are supported.
  // Assumes `select_mux_channel` has already been called
  fn set_date_raw(&mut self, date: &NaiveDate) -> Result<(), E> {
    let year = if date.year() > 2000 { (date.year() - 2000) as u8} else {0};
    let month = (date.month() % 13) as u8;
    let day = (date.day() % 32) as u8;
    let weekday = (date.weekday() as u8) % 7;

    let write_buf = [
      REG_WEEKDAY, // select the first register
      Self::bin_to_bcd(weekday ),
      Self::bin_to_bcd(day ),
      Self::bin_to_bcd(month ),
      Self::bin_to_bcd(year )
    ];
    self.i2c.write(RV3028_ADDRESS, &write_buf)
  }

  /// Get the year, month, day from the internal BCD registers
  pub fn get_ymd(&mut self) -> Result<(i32, u8, u8), E> {
    let mut read_buf = [0u8;3];
    self.read_multi_registers(REG_DATE, &mut read_buf)?;
    let day = Self::bcd_to_bin(read_buf[0]);
    let month = Self::bcd_to_bin(read_buf[1]);
    let year:i32 = Self::bcd_to_bin(read_buf[2]) as i32 + 2000;

    Ok((year, month, day))
  }

  /// Get the hour, minute, second from the internal BCD registers
  pub fn get_hms(&mut self) -> Result<(u8, u8, u8), E> {
    let mut read_buf = [0u8;3];
    self.read_multi_registers(REG_SECONDS, &mut read_buf)?;
    let seconds = Self::bcd_to_bin(read_buf[0]);
    let minutes = Self::bcd_to_bin(read_buf[1]);
    let hours = Self::bcd_to_bin(read_buf[2]);
    Ok( (hours, minutes, seconds) )
  }

  // read a block of registers all at once
  fn read_multi_registers(&mut self, reg: u8, read_buf: &mut [u8] )  -> Result<(), E> {
    self.select_mux_channel()?;
    self.read_multi_registers_raw(reg, read_buf)
  }

  // read a block of registers all at once: skip mux
  fn read_multi_registers_raw(&mut self, reg: u8, read_buf: &mut [u8] )  -> Result<(), E> {
    self.i2c.write_read(RV3028_ADDRESS, &[reg], read_buf)
  }

  /// Set just the Unix time counter.
  /// Prefer the `set_datetime` method to properly set all internal BCD registers.
  /// Note:
  /// - This does NOT set other internal BCD registers
  /// such as Year or Hour: if you want to set those as well, use the
  /// `set_datetime` method instead.
  /// - This does not reset the prescaler pipeline,
  /// which means subseconds are not reset to zero.
  ///
  pub fn set_unix_time(&mut self, unix_time: u32) -> Result<(), E> {
    self.select_mux_channel()?;
    self.set_unix_time_raw(unix_time)
  }

  // sets the unix time counter but skips the mux
  fn set_unix_time_raw(&mut self, unix_time: u32) -> Result<(), E> {
    let bytes = unix_time.to_le_bytes(); // Convert to little-endian byte array
    self.i2c.write(RV3028_ADDRESS, &[REG_UNIX_TIME_0, bytes[0], bytes[1], bytes[2], bytes[3]])
  }

  /// Reads the value of the RTC's unix time counter, notionally seconds elapsed since the
  /// common "unix epoch" in the year 1970. It cannot represent datetimes from prior to 1970.
  /// - Note that this is an unsigned u32 value, with different characteristics from the
  /// widely used rust / chrono i64 system timestamp.
  /// - The RTC will continue to increment this counter until it wraps at 0xFFFFFFFF
  /// which defers the "Year 2038 problem" until about the year 2106.
  /// - Note that the RTC's automatic leap year correction is only valid until 2099
  /// See the App Manual section "3.10. UNIX TIME REGISTERS"
  pub fn get_unix_time(&mut self) -> Result<u32, E> {
    let mut read_buf = [0u8; 4];
    self.read_multi_registers(REG_UNIX_TIME_0, &mut read_buf)?;
    let val = u32::from_le_bytes(read_buf);
    Ok(val)
  }

  /// The vendor application manual suggest we read the unix time twice,
  /// in case an internal increment or timestamp set is interspersed between the multi-byte read.
  /// This method performs the recommended read-twice.
  pub fn get_unix_time_blocking(&mut self) -> Result<u32, E> {
    loop {
      let val1 = self.get_unix_time()?;
      let val2 = self.get_unix_time()?;

      if val1 == val2 {
        return Ok(val2)
      }
    }
  }

  /// Toggle whether EVI events trigger on high/rising or low/falling edges
  pub fn toggle_event_high_low(&mut self, high: bool) -> Result<(), E> {
    self.set_or_clear_reg_bits(REG_EVENT_CONTROL, EVENT_HIGH_LOW_BIT, high)
  }

  /// Toggle whether an alarm outputs an interrupt signal on the INT pin
  pub fn toggle_alarm_interrupt_out(&mut self, enable: bool) -> Result<(), E> {
    self.set_or_clear_reg_bits(REG_CONTROL2, ALARM_INT_ENABLE_BIT, enable)
  }

  /// Toggle whether interrupt signal is generated on the INT pin:
  /// - when an External Event on EVI pin occurs and TSS = 0
  /// - or when an Automatic Backup Switchover occurs and TSS = 1.
  /// The signal on the INT pin is retained until the EVF flag is cleared
  /// to 0 (no automatic cancellation)
  pub fn toggle_event_interrupt_out(&mut self, enable: bool) -> Result<(), E> {
    self.set_or_clear_reg_bits(REG_CONTROL2, EVENT_INT_ENABLE_BIT, enable)
  }

  /// Check the alarm status, and if it's triggered, clear it
  /// return bool indicating whether the alarm triggered
  pub fn check_and_clear_alarm(&mut self) -> Result<bool, E> {
    // Check if the AF flag is set
    let alarm_flag_set = 0 != self.check_and_clear_bits(REG_STATUS, ALARM_FLAG_BIT)?;
    // let reg_val = self.read_register(REG_STATUS)?;
    // let alarm_flag_set =  0 != (reg_val & ALARM_FLAG_BIT); // Check if the AF flag is set
    // if alarm_flag_set {
    //   self.clear_reg_bits(REG_STATUS, ALARM_FLAG_BIT)?;
    // }
    Ok(alarm_flag_set)
  }

  /// All-in-one method to set an alarm:
  /// See the App Note section "Procedure to use the Alarm Interrupt"
  /// Note only date/weekday, hour, minute are supported
  /// - If `weekday` is provided then it'll setup a weekday alarm rather than date alarm
  /// - `match_day` indicates whether the day (or weekday) should be matched for the alarm
  /// - `match_hour` indicates whether the hour should be matched for the alarm
  /// - `match_minute` indicates whether the minutes should be matched for the alarm
  pub fn set_alarm(&mut self, datetime: &NaiveDateTime,
                   weekday: Option<Weekday>, match_day: bool, match_hour: bool, match_minute: bool) -> Result<(), E> {

    self.select_mux_channel()?;
    // Initialize AF to 0; AIE/ALARM_INT_ENABLE_BIT is managed independently
    self.clear_reg_bits_raw(REG_STATUS, ALARM_FLAG_BIT)?;

    // Procedure suggested by App Notes:
    // 1. Initialize bits AIE and AF to 0.
    // 2. Choose weekday alarm or date alarm (weekday/date) by setting the WADA bit.
    // WADA = 0 for weekday alarm or WADA = 1 for date alarm.
    // 3. Write the desired alarm settings in registers 07h to 09h. The three alarm enable bits, AE_M, AE_H and
    // AE_WD, are used to select the corresponding register that has to be taken into account for match or not.
    // See the following table.

    // Clear WADA for weekday alarm, or set for date alarm
    self.set_or_clear_reg_bits_raw(REG_CONTROL1, WADA_BIT, !weekday.is_some())?;

    let bcd_minute = Self::bin_to_bcd(datetime.time().minute() as u8);
    self.write_register_raw(REG_MINUTES_ALARM,
                        if match_minute { bcd_minute }
                        else { ALARM_NO_WATCH_FLAG | bcd_minute })?;

    let bcd_hour = Self::bin_to_bcd(datetime.time().hour() as u8);
    self.write_register_raw(REG_HOURS_ALARM,
                        if match_hour { bcd_hour  }
                        else { ALARM_NO_WATCH_FLAG | bcd_hour })?;

    if let Some(inner_weekday) = weekday {
      let bcd_weekday = Self::bin_to_bcd(inner_weekday as u8);
      self.write_register_raw(REG_WEEKDAY_DATE_ALARM,
                          if match_day { bcd_weekday }
                          else { ALARM_NO_WATCH_FLAG | bcd_weekday }
      )?;
    }
    else {
      let bcd_day = Self::bin_to_bcd(datetime.date().day() as u8);
      self.write_register_raw(REG_WEEKDAY_DATE_ALARM,
                          if match_day { bcd_day }
                          else { ALARM_NO_WATCH_FLAG | bcd_day })?;
    }

    // Clear AF again in case the above setting process immediately triggered the alarm
    self.clear_reg_bits_raw(REG_STATUS, ALARM_FLAG_BIT)?;

    Ok(())
  }

  pub fn get_alarm_datetime_wday_matches(&mut self)
    -> Result<(NaiveDateTime, Option<Weekday>, bool, bool, bool), E> {

    self.select_mux_channel()?;

    let raw_day = self.read_register_raw(REG_WEEKDAY_DATE_ALARM)?;
    let match_day = 0 == (raw_day & ALARM_NO_WATCH_FLAG);
    let day = Self::bcd_to_bin(0x7F & raw_day);

    let raw_hour = self.read_register_raw(REG_HOURS_ALARM)?;
    let match_hour = 0 == (raw_hour & ALARM_NO_WATCH_FLAG);
    let hour = Self::bcd_to_bin(0x7F & raw_hour);

    let raw_minutes = self.read_register_raw(REG_MINUTES_ALARM)?;
    let match_minutes = 0 == (raw_minutes & ALARM_NO_WATCH_FLAG);
    let minutes = Self::bcd_to_bin(0x7F & raw_minutes);

    let mut weekday = None;

    let wada_state = self.read_register_raw(REG_CONTROL1)? & WADA_BIT;

    let dt =
      if 0 == wada_state {
        // weekday alarm
        weekday = Some(Weekday::try_from(day).unwrap());
        NaiveDateTime::UNIX_EPOCH.with_hour(hour as u32).unwrap()
          .with_minute(minutes as u32).unwrap()
      }
      else {
        // date alarm
        NaiveDateTime::UNIX_EPOCH.with_day(day as u32).unwrap()
          .with_hour(hour as u32).unwrap()
          .with_minute(minutes as u32).unwrap()
      };

    Ok((dt, weekday, match_day, match_hour, match_minutes))
  }

  /// Enable INT pin output when alarm occurs
  pub fn toggle_alarm_int_enable(&mut self, enable: bool) -> Result<(), E> {
    self.set_or_clear_reg_bits(REG_CONTROL2, ALARM_INT_ENABLE_BIT, enable)
  }

  // If `set` is true, set the high bits given in `bits`, otherwise clear those bits
  fn set_or_clear_reg_bits(&mut self, reg: u8, bits: u8, set: bool) -> Result<(), E> {
    self.select_mux_channel()?;
    self.set_or_clear_reg_bits_raw(reg, bits, set)
  }

  fn set_or_clear_reg_bits_raw(&mut self, reg: u8, bits: u8, set: bool) -> Result<(), E> {
    if set {
      self.set_reg_bits_raw(reg, bits)
    }
    else {
      self.clear_reg_bits_raw(reg, bits)
    }
  }

  /// Disable all INT pin output selector bits in RAM, excludes PORIE
  pub fn clear_all_int_out_bits(&mut self) -> Result<(), E> {
    // UIE, TIE, AIE,  EIE
    self.clear_reg_bits(REG_CONTROL2,
                        TIME_UPDATE_INT_ENABLE_BIT | TIMER_INT_ENABLE_BITL |
                          ALARM_INT_ENABLE_BIT | EVENT_INT_ENABLE_BIT )?;
    // BSIE
    self.clear_reg_bits(EEPROM_MIRROR_ADDRESS, BACKUP_SWITCH_INT_ENABLE_BIT)?;

    // PORIE -- must be set in EEPROM -- don't bother to set?
    Ok(())
  }

  /// Enables or disables CLKOUT
  pub fn toggle_clock_output(&mut self, enable: bool)  -> Result<(), E> {
    self.select_mux_channel()?;
    self.clear_reg_bits_raw(REG_STATUS, CLOCK_INT_FLAG_BIT)?;
    self.set_or_clear_reg_bits_raw(EEPROM_MIRROR_ADDRESS, CLOCK_OUT_ENABLE_BIT, enable)
  }

  // Configure the Periodic Countdown Timer prior to the next countdown.
  fn pct_prep(&mut self, value: u16, freq: TimerClockFreq,  repeat: bool ) -> Result<(), E> {
    self.select_mux_channel()?;

    let value_high: u8 = ((value >> 8) as u8) & 0x0F;
    let value_low: u8 = (value & 0xFF) as u8;


    // configure the timer clock source / period
    self.clear_reg_bits_raw(REG_CONTROL1, TIMER_ENABLE_BIT)?;
    self.set_or_clear_reg_bits_raw(REG_CONTROL1, TIMER_REPEAT_BIT, repeat)?;

    self.clear_reg_bits_raw(REG_CONTROL1, TIMER_CLOCK_FREQ_BITS)?;
    self.set_reg_bits_raw(REG_CONTROL1, freq as u8)?;

    // write to REG_TIMER_VALUE0 and REG_TIMER_VALUE1
    let write_buf = [ REG_TIMER_VALUE0, value_low, value_high];
    self.i2c.write(RV3028_ADDRESS, &write_buf)?;

    self.clear_reg_bits_raw(REG_STATUS, PERIODIC_TIMER_FLAG)?;
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
    let whole_minutes = duration.num_minutes();
    let whole_seconds = duration.num_seconds();
    let whole_milliseconds = duration.num_milliseconds();
    let frac_milliseconds = whole_milliseconds % 1_000;
    let infrac_milliseconds = whole_milliseconds % Self::PCT_MILLIS_PERIOD;
    let whole_microseconds = duration.num_microseconds().unwrap();

    return if whole_minutes >= Self::MAX_PCT_COUNT {
      (Self::MAX_PCT_TICKS, TimerClockFreq::HertzSixtieth, Duration::minutes(Self::MAX_PCT_COUNT))
    } else if whole_seconds > Self::MAX_PCT_COUNT {
      // use minutes
      let ticks = whole_minutes;
      (ticks as u16, TimerClockFreq::HertzSixtieth, Duration::minutes(ticks))
    } else if  (whole_milliseconds > Self::MAX_PCT_MILLIS) ||
      ((0 == frac_milliseconds) && (whole_milliseconds > Self::PCT_MILLIS_SECOND_BARRIER))  {
      // use seconds
      let ticks = whole_seconds;
      (ticks as u16, TimerClockFreq::Hertz1, Duration::seconds(ticks))
    } else if (whole_microseconds > Self::MAX_PCT_MICROS) ||
      ((0 == infrac_milliseconds) && (whole_milliseconds >= Self::PCT_MILLIS_PERIOD)) {
      // use milliseconds
      let ticks = whole_milliseconds / Self::PCT_MILLIS_PERIOD;
      (ticks as u16, TimerClockFreq::Hertz64,
       Duration::milliseconds(ticks * Self::PCT_MILLIS_PERIOD))
    } else {
      // use microseconds
      let ticks = whole_microseconds / Self::PCT_MICROS_PERIOD;
      (ticks as u16, TimerClockFreq::Hertz4096,
       Duration::microseconds(ticks * Self::PCT_MICROS_PERIOD))
    }

  }

  /// Prepare the Periodic Countdown Timer for a countdown,
  /// but don't start it counting down yet.
  ///
  /// - `repeat`: If true, the countdown timer will repeat as a periodic timer.
  /// If false, the countdown timer will only run once ("one-shot" mode).
  /// Returns the estimated actual duration (which may vary from the requested duration
  /// dur to discrete RTC clock ticks).
  pub fn setup_countdown_timer(&mut self, duration: &Duration,
                               repeat: bool
  )  -> Result<Duration, E> {
    let (ticks, freq, estimated) =
      Self::pct_ticks_and_rate_for_duration(duration);
    self.pct_prep(ticks, freq, repeat)?;
    Ok(estimated)
  }

  /// Set whether the Periodic Countdown Timer mode is repeating (periodic) or one-shot.
  /// - `enable`: If true, starts the timer countdown. If false, stops the timer.
  pub fn toggle_countdown_timer(&mut self, enable: bool)  -> Result<(), E> {
    self.set_or_clear_reg_bits(REG_CONTROL1, TIMER_ENABLE_BIT, enable)
  }

  // /// Check whether the Periodic Countdown Timer has finished
  // pub fn check_countdown_finished(&mut self) -> Result<bool, E> {
  //   let reg_val = self.read_register(REG_STATUS)?;
  //   let flag_set =  0 != (reg_val & PERIODIC_TIMER_FLAG); // Check if the TF flag is set
  //   Ok(flag_set)
  // }

  /// Check whether countdown timer has finished counting down, and clear it
  pub fn check_and_clear_countdown(&mut self) -> Result<bool, E> {
    let flag_set = 0 != self.check_and_clear_bits(REG_STATUS, PERIODIC_TIMER_FLAG)?;
    Ok(flag_set)
  }

  /// Read the current value of the Periodic Countdown Timer,
  /// which is only valid after the timer has been enabled.
  /// The meaning of the value depends on the configured TimerClockFreq
  pub fn get_countdown_value(&mut self) -> Result<u16, E> {
    let mut read_buf = [0u8;2];
    self.read_multi_registers(REG_TIMER_STATUS0, &mut read_buf)?;
    let value = ((read_buf[1] as u16) << 8) | (read_buf[0] as u16);
    Ok(value)
  }

  // check and clear a flag
  fn check_and_clear_bits(&mut self, reg: u8, bits: u8) -> Result<u8, E> {
    self.select_mux_channel()?;
    let reg_val = self.read_register_raw(reg)?;
    let bits_val =  reg_val & bits;
    if 0 != bits_val {
      self.clear_reg_bits_raw(reg, bits)?;
    }
    Ok(bits_val)
  }

}


pub trait EventTimeStampLogger {
  /// Error type
  type Error;

  /// Enable or disable the Time Stamp Function for event logging
  /// This logs external interrupts or other events
  fn toggle_event_log(&mut self, enable: bool) -> Result<(), Self::Error>;

  /// Get event count -- the number of events that have been logged since enabling logging
  /// Returns the count of events since last reset, and the datetime of one event
  fn get_event_count_and_datetime(&mut self) -> Result<(u32, Option<NaiveDateTime>), Self::Error>;

  /// Enable or disable event time stamp overwriting
  /// If this is disabled (default), the first event time stamp is saved.
  /// If this is enabled, the most recent event time stamp is saved.
  fn toggle_time_stamp_overwrite(&mut self, enable: bool) -> Result<(), Self::Error>;

  /// Select a source for events to be logged, device-specific
  fn set_event_source(&mut self, source: u8) -> Result<(), Self::Error>;
}

impl<I2C, E> DateTimeAccess for  RV3028<I2C>
  where
    I2C: Write<Error = E> + Read<Error = E> + WriteRead<Error = E>,
{
  type Error = E;

  /// This particular RTC's timestamps wrap at 0xFFFF_FFFF, around the year 2106.
  /// It doesn't support:
  /// - years prior to 1970
  /// - leap year calculations past 2099
  fn datetime(&mut self) -> Result<NaiveDateTime, Self::Error> {
    let unix_timestamp = self.get_unix_time()?;
    Ok(NaiveDateTime::from_timestamp_opt(unix_timestamp.into(), 0).unwrap())
  }

  /// This implementation assumes (but doesn't verify)
  /// that the caller is setting the RTC datetime to values within its range (from 2000 to 2099).
  /// The RTC doesn't support leap year corrections beyond 2099,
  /// and the internal Year BCD register only runs from 0..99 (for 2000..2099).
  /// This method resets the internal prescaler pipeline, which means that
  /// subsecond counters are zeroed, when it writes to the Seconds register.
  /// This assists with clock synchronization with external clocks.
  fn set_datetime(&mut self, datetime: &NaiveDateTime) -> Result<(), Self::Error> {
    let unix_timestamp: u32 = datetime.timestamp().try_into().unwrap();
    self.select_mux_channel()?;
    // unix timestamp counter is stored in registers separate from everything else:
    // this method tries to align both, because the unix timestamp is not
    // used by eg the Event or Alarm interrupts
    self.set_unix_time_raw(unix_timestamp)?;
    self.set_date_raw(&datetime.date())?;
    // this must come last because writing to the seconds register resets
    // the upper stage of the prescaler
    self.set_time_raw(&datetime.time())?;
    Ok(())
  }

}
impl<I2C, E> EventTimeStampLogger for  RV3028<I2C>
  where
    I2C: Write<Error = E> + Read<Error = E> + WriteRead<Error = E>
{
  type Error = E;

  fn toggle_event_log(&mut self, enable: bool) -> Result<(), Self::Error> {
    if enable {
      // App notes recommend first disabling the event log with TSE and setting TSR
      // 1. Initialize bits TSE and EIE to 0.
      // 2. Select TSOW (0 or 1), clear EVF and BSF.
      // 3. Write 1 to TSR bit, to reset all Time Stamp registers to 00h. Bit TSR always returns 0 when read.
      // 4. Select the External Event Interrupt function (TSS = 0) or the Automatic Backup Switchover Interrupt
      // function (TSS = 1) as time stamp source and initialize the appropriate function
      // (see EXTERNAL EVENT INTERRUPT FUNCTION or AUTOMATIC BACKUP SWITCHOVER INTERRUPT FUNCTION).
      // 5. Set the TSE bit to 1 to enable the Time Stamp function.

      // Initialize bits TSE and EIE to 0.
      self.clear_reg_bits(REG_CONTROL2, TIME_STAMP_ENABLE_BIT)?;
      // Assume that TSOW has already been selected
      // Clear the single event detect flag EVF and BSF
      self.clear_reg_bits(REG_STATUS, EVENT_FLAG_BIT)?;
      self.clear_reg_bits(REG_STATUS, BACKUP_SWITCH_FLAG)?;
      // Reset all Time Stamp registers to zero
      self.set_reg_bits(REG_EVENT_CONTROL, TIME_STAMP_RESET_BIT)?;

      // start listening for events
      self.set_reg_bits(REG_CONTROL2, TIME_STAMP_ENABLE_BIT)
    }
    else {
      // stop listening for events
      self.clear_reg_bits(REG_CONTROL2, TIME_STAMP_ENABLE_BIT)
    }
  }

  fn get_event_count_and_datetime(&mut self) -> Result<(u32, Option<NaiveDateTime>), Self::Error> {
    // Read the seven raw Time Stamp Function registers in one go
    let mut read_buf:[u8;7] = [0u8;7];
    self.read_multi_registers(REG_COUNT_EVENTS_TS, &mut read_buf)?;

    // Convert BCD values to binary
    let count = read_buf[0]; // Count is already in binary

    let odt = {
      if count > 0 {
        let seconds = Self::bcd_to_bin(read_buf[1]);
        let minutes = Self::bcd_to_bin(read_buf[2]);
        let hours = Self::bcd_to_bin(read_buf[3]);
        let date = Self::bcd_to_bin(read_buf[4]);
        let month = Self::bcd_to_bin(read_buf[5]);
        let year:i32 = Self::bcd_to_bin(read_buf[6]) as i32 + 2000;
        Some(NaiveDate::from_ymd_opt(year as i32, month as u32, date as u32)
        .expect("YMD")
          .and_hms_opt(hours as u32, minutes as u32, seconds as u32)
          .expect("HMS"))
      }
      else {
        None
      }
    };

    Ok((count.into(), odt))
  }

  fn toggle_time_stamp_overwrite(&mut self, enable: bool) -> Result<(), Self::Error> {
    self.set_or_clear_reg_bits(REG_EVENT_CONTROL, TIME_STAMP_OVERWRITE_BIT, enable)
  }

  fn set_event_source(&mut self, source: u8) -> Result<(), Self::Error> {
    self.set_or_clear_reg_bits(REG_EVENT_CONTROL, TIME_STAMP_SOURCE_BIT,
                               TS_EVENT_SOURCE_EVI != source)
  }

}

#[cfg(test)]
mod tests {
  use super::*;
  use embedded_hal_mock::i2c::{Mock as I2cMock, Transaction as I2cTrans};
  use std::vec;


  type TestClass = RV3028<I2cMock>;


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
    assert_eq!(rv3028.get_unix_time().unwrap(), unix_time);
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
      &Duration::minutes(TestClass::MAX_PCT_COUNT + 32),
      minutes_clock_freq, TestClass::MAX_PCT_TICKS);

    verify_whole_time_estimate(
      &Duration::minutes(TestClass::MAX_PCT_COUNT),
      minutes_clock_freq, TestClass::MAX_PCT_TICKS);

    // exceed the Seconds counter just slightly to invoke Minutes counter
    const MAX_SECONDS_IN_MINUTES: i64 = (TestClass::MAX_PCT_COUNT/ 60) + 1;
    verify_whole_time_estimate(
      &Duration::minutes(MAX_SECONDS_IN_MINUTES),
      minutes_clock_freq, MAX_SECONDS_IN_MINUTES as u16);

    verify_whole_time_estimate(
      &Duration::minutes(2047),
      minutes_clock_freq, 2047);

  }

  #[test]
  fn test_countdown_timer_conversion_seconds() {
    // should be fulfilled as seconds
    let seconds_clock_freq = TimerClockFreq::Hertz1;

    // Maximum seconds ticks
    verify_whole_time_estimate(
      &Duration::seconds(TestClass::MAX_PCT_COUNT),
      seconds_clock_freq, TestClass::MAX_PCT_TICKS);

    verify_whole_time_estimate(
      &Duration::seconds(2047),
      seconds_clock_freq, 2047);

    verify_whole_time_estimate(
      &Duration::seconds(61),
      seconds_clock_freq, 61);

    // we serve whole minutes (under max seconds) with a seconds countdown
    verify_whole_time_estimate(
      &Duration::seconds(60),
      seconds_clock_freq, 60);

    verify_whole_time_estimate(
      &Duration::minutes(1),
      seconds_clock_freq, 60);

    verify_whole_time_estimate(
      &Duration::minutes(45),
      seconds_clock_freq, 45*60);

    // minimum Seconds ticks
    verify_whole_time_estimate(
      &Duration::seconds(1),
      seconds_clock_freq, 1);

  }

  #[test]
  fn test_countdown_timer_conversion_micros() {
    // should be fulfilled as microseconds
    let micros_clock_freq = TimerClockFreq::Hertz4096;

    verify_whole_time_estimate(
      &Duration::microseconds(TestClass::MAX_PCT_MICROS),
      micros_clock_freq, TestClass::MAX_PCT_TICKS);

    verify_ticks_and_freq(
      &Duration::microseconds(2048),
      micros_clock_freq, (2048 / TestClass::PCT_MICROS_PERIOD) as u16);

    verify_ticks_and_freq(
      &Duration::microseconds(655),
      micros_clock_freq, (655 / TestClass::PCT_MICROS_PERIOD) as u16);

    verify_ticks_and_freq(
      &Duration::microseconds(1024),
      micros_clock_freq, (1024 / TestClass::PCT_MICROS_PERIOD) as u16);

    // some exact micros values

    verify_whole_time_estimate(
      &Duration::microseconds(999*TestClass::PCT_MICROS_PERIOD),
      micros_clock_freq, 999);

    verify_whole_time_estimate(
      &Duration::microseconds(100*TestClass::PCT_MICROS_PERIOD),
      micros_clock_freq, 100);

    verify_whole_time_estimate(
      &Duration::microseconds(17*TestClass::PCT_MICROS_PERIOD),
      micros_clock_freq, 17);

    // minimum microseconds tick
    verify_whole_time_estimate(
      &Duration::microseconds(TestClass::PCT_MICROS_PERIOD),
      micros_clock_freq, 1);

  }

  #[test]
  fn test_countdown_timer_conversion_millis() {
    // should be fulfilled as milliseconds
    let millis_clock_freq = TimerClockFreq::Hertz64;

    // a bit more than max micros counter, but less than max millis counter
    verify_ticks_and_freq(
      &Duration::microseconds(TestClass::MAX_PCT_MICROS + 1),
      millis_clock_freq,
      ((TestClass::MAX_PCT_MICROS + 1) / (TestClass::PCT_MILLIS_PERIOD * 1_000)) as u16
    );

    // maximum millis counter
    verify_whole_time_estimate(
      &Duration::milliseconds(TestClass::MAX_PCT_MILLIS),
      millis_clock_freq, TestClass::MAX_PCT_TICKS);

    // mid-value millis
    verify_ticks_and_freq(
      &Duration::milliseconds(2047),
      millis_clock_freq, (2047 / TestClass::PCT_MILLIS_PERIOD) as u16);

    // exactly on the seconds clock
    verify_whole_time_estimate(
      &Duration::milliseconds(1000*TestClass::PCT_MILLIS_PERIOD),
      TimerClockFreq::Hertz1, TestClass::PCT_MILLIS_PERIOD as u16 );

    // exactly on the millis period

    verify_whole_time_estimate(
      &Duration::milliseconds(999*TestClass::PCT_MILLIS_PERIOD),
      millis_clock_freq, 999);

    verify_whole_time_estimate(
      &Duration::milliseconds(100*TestClass::PCT_MILLIS_PERIOD),
      millis_clock_freq, 100);

    // minimum millis ticks
    verify_whole_time_estimate(
      &Duration::milliseconds(TestClass::PCT_MILLIS_PERIOD),
      millis_clock_freq, 1);

  }



}

