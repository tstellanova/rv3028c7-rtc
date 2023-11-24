#![cfg_attr(not(test), no_std)]


pub use rtcc::{
  DateTimeAccess, NaiveDate, NaiveDateTime, Datelike, Timelike,
};



use embedded_hal::blocking::i2c::{Write, Read, WriteRead};

// Fixed i2c bus address of the device (7-bit)
const RV3028_ADDRESS: u8 = 0xA4 >> 1;

// Register addresses
const REG_SECONDS: u8 = 0x00;
const REG_MINUTES: u8 = 0x01;
const REG_HOURS: u8 = 0x02;


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
const REG_MONTH: u8 = 0x05;
const REG_YEAR: u8 = 0x06;


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

// 0Ah – Timer Value 0
// 0Bh – Timer Value 1

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
const WADA_BIT: u8 = 1 << 5; // Weekday Alarm / Date Alarm selection bit WADA
// const USEL_BIT: u8 = 1 << 4;
// const EERD_BIT: u8 = 1 << 3;


// REG_STATUS Status register bits:
// const EEBUSY_BIT: u8 = 1 << 7;
const BACKUP_SWITCH_FLAG: u8 = 1 << 4; // BSF bit
const ALARM_FLAG_BIT : u8 = 1 << 2; // AF / Alarm Flag
const EVENT_FLAG_BIT: u8 = 1 << 1; // EVF / Event Flag

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
const ALARM_INT_ENABLE_BIT: u8 = 1 << 3;// AIE / Alarm Interrupt Enable bit
const EVENT_INT_ENABLE_BIT: u8 = 1 << 2;// EIE / Event Interrupt Enable bit


// EEPROM register bits:
const TRICKLE_CHARGE_ENABLE_BIT: u8 = 1 << 5; // TCE bit
// const TRICKLE_CHARGE_RESISTANCE_BITS: u8 = 1 << 0; // TCR bit
// pub const TRICKLE_CHARGE_RESISTANCE_VALUE_3K: u8 = 0b00;
// pub const TRICKLE_CHARGE_RESISTANCE_VALUE_5K: u8 = 0b01;
// pub const TRICKLE_CHARGE_RESISTANCE_VALUE_9K: u8 = 0b10;
// pub const TRICKLE_CHARGE_RESISTANCE_VALUE_15K: u8 = 0b11;

/// Days of week are 0..6 as defined by chrono::Weekday
/// This value represents a known invalid weekday
pub const INVALID_WEEKDAY:u8 = 28;

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

  fn write_register(&mut self, reg: u8, data: u8) -> Result<(), E> {
    self.select_mux_channel()?;
    self.i2c.write(RV3028_ADDRESS, &[reg, data])
  }

  fn read_register(&mut self, reg: u8) -> Result<u8, E> {
    self.select_mux_channel()?;
    let mut buf = [0];
    self.i2c.write_read(RV3028_ADDRESS, &[reg], &mut buf)?;
    Ok(buf[0])
  }

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
    let mut reg_val = self.read_register(reg)?;
    reg_val |= bits; // Set bits that are high
    self.write_register(reg, reg_val)
  }

  // clear specific bits in a register:
  // all bits must be high that you wish to be cleared
  fn clear_reg_bits(&mut self, reg: u8, bits: u8) -> Result<(), E> {
    let mut reg_val = self.read_register(reg)?;
    reg_val &= !(bits); // Clear  bits that are high
    self.write_register(reg, reg_val)
  }

  /// Enable or disable trickle charging
  pub fn toggle_trickle_charge(&mut self, enable: bool) -> Result<(), E>  {
    self.set_or_clear_reg_bits(EEPROM_MIRROR_ADDRESS, TRICKLE_CHARGE_ENABLE_BIT, enable)
  }


  /// Get the current value of the EEPROM mirror from RAM
  pub fn get_eeprom_mirror_value(&mut self) -> Result<u8, E> {
    let reg_val = self.read_register(EEPROM_MIRROR_ADDRESS)?;
    Ok(reg_val)
  }

  /// Set time of day (hours, minutes, seconds) in binary format
  pub fn set_time(&mut self, hours: u8, minutes: u8, seconds: u8) -> Result<(), E> {
    self.write_register(REG_HOURS, Self::bin_to_bcd(hours))?;
    self.write_register(REG_MINUTES, Self::bin_to_bcd(minutes))?;
    self.write_register(REG_SECONDS, Self::bin_to_bcd(seconds))
  }

  /// Get time of day in binary format (hours, minutes, seconds)
  pub fn get_time(&mut self) -> Result<(u8, u8, u8), E> {
    let hours = Self::bcd_to_bin(self.read_register(REG_HOURS)?);
    let minutes = Self::bcd_to_bin(self.read_register(REG_MINUTES)?);
    let seconds = Self::bcd_to_bin(self.read_register(REG_SECONDS)?);
    Ok((hours, minutes, seconds))
  }

  /// Set the weekday (day of week, 0..6)
  pub fn set_weekday(&mut self, weekday: u8) -> Result<(), E> {
    self.write_register(REG_WEEKDAY, Self::bin_to_bcd(weekday))
  }

  /// Get the weekday (day of week, 0..6)
  pub fn get_weekday(&mut self) -> Result<u8, E> {
    let bcd = self.read_register(REG_WEEKDAY)?;
    Ok(Self::bcd_to_bin(bcd))
  }

  /// Set the calendar year, month, day. Year is 0..99  (for 2000 to 2099)
  pub fn set_year_month_day(&mut self, year: u8, month: u8, day: u8) -> Result<(), E> {
    self.write_register(REG_YEAR, Self::bin_to_bcd(year))?;
    self.write_register(REG_MONTH, Self::bin_to_bcd(month))?;
    self.write_register(REG_DATE, Self::bin_to_bcd(day))
  }

  /// Set the calendar date (day number of month) (1..31)
  pub fn set_date(&mut self, date: u8) -> Result<(), E> {
    self.write_register(REG_DATE, Self::bin_to_bcd(date))
  }

  /// Get the calendar date (day number of month) (1..31)
  pub fn get_date(&mut self) -> Result<u8, E> {
    let bcd = self.read_register(REG_DATE)?;
    Ok(Self::bcd_to_bin(bcd))
  }

  /// Set the calendar month (1..12)
  pub fn set_month(&mut self, month: u8) -> Result<(), E> {
    self.write_register(REG_MONTH, Self::bin_to_bcd(month))
  }

  /// Get the calendar month (1..12)
  pub fn get_month(&mut self) -> Result<u8, E> {
    let bcd = self.read_register(REG_MONTH)?;
    Ok(Self::bcd_to_bin(bcd))
  }

  /// Set the calendar year (1..12)
  pub fn set_year(&mut self, year: u8) -> Result<(), E> {
    self.write_register(REG_YEAR, Self::bin_to_bcd(year))
  }

  /// Get the calendar year (00..99 for 2000..2099)
  pub fn get_year(&mut self) -> Result<u8, E> {
    let bcd = self.read_register(REG_YEAR)?;
    Ok(Self::bcd_to_bin(bcd))
  }

  /// Set the calendar year, month, day
  pub fn get_year_month_day(&mut self) -> Result<(u8, u8, u8), E> {
    let year = self.get_year()?;
    let month = self.get_month()?;
    let day = self.get_date()?;
    Ok((year,month,day))
  }

  // read a block of registers all at once
  fn read_multi_registers(&mut self, reg: u8, read_buf: &mut [u8] )  -> Result<(), E> {
    self.select_mux_channel()?;
    self.i2c.write_read(RV3028_ADDRESS, &[reg], read_buf)
  }

  /// Set the Unix time counter
  pub fn set_unix_time(&mut self, unix_time: u32) -> Result<(), E> {
    self.select_mux_channel()?;
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

  /// Check the alarm status, and if it's triggered, clear it
  /// return bool indicating whether the alarm triggered
  pub fn check_and_clear_alarm(&mut self) -> Result<bool, E> {
    let reg_val = self.read_register(REG_STATUS)?;
    let alarm_flag_set = 0 != (reg_val & ALARM_FLAG_BIT);
    if alarm_flag_set {
      self.clear_reg_bits(REG_STATUS, ALARM_FLAG_BIT)?;
    }
    Ok(alarm_flag_set)
  }

  /// All-in-one method to set an alarm:
  /// See the App Note section "Procedure to use the Alarm Interrupt"
  /// Note only date/weekday, hour, minute are supported
  /// - If `weekday` is in the range 0..6 then it'll setup a weekday alarm rather than date alarm
  /// - `match_day` indicates whether the day (or weekday) should be matched for the alarm
  /// - `match_hour` indicates whether the hour should be matched for the alarm
  /// - `match_minute` indicates whether the minutes should be matched for the alarm
  pub fn set_alarm(&mut self, datetime: &NaiveDateTime,
                   weekday: u8, match_day: bool, match_hour: bool, match_minute: bool) -> Result<(), E> {

    // Initialize AF to 0; AIE/ALARM_INT_ENABLE_BIT is managed independently
    self.clear_reg_bits(REG_STATUS, ALARM_FLAG_BIT)?;

    let bcd_hour = Self::bin_to_bcd(datetime.time().hour() as u8);
    self.write_register(REG_HOURS_ALARM,
                        if match_hour { bcd_hour  }
                        else { ALARM_NO_WATCH_FLAG | bcd_hour })?;

    let bcd_minute = Self::bin_to_bcd(datetime.time().minute() as u8);
    self.write_register(REG_MINUTES_ALARM,
                        if match_minute { bcd_minute }
                        else { ALARM_NO_WATCH_FLAG | bcd_minute })?;

    if weekday < 7 { // Clear WADA for weekday alarm
      let bcd_weekday = Self::bin_to_bcd(weekday);
      self.clear_reg_bits(REG_CONTROL1, WADA_BIT)?;
      self.write_register(REG_WEEKDAY_DATE_ALARM,
                          if match_day { bcd_weekday }
                          else { ALARM_NO_WATCH_FLAG | bcd_weekday }
      )?;
    }
    else { // Set WADA for date alarm
      let bcd_day = Self::bin_to_bcd(datetime.date().day() as u8);
      self.set_reg_bits(REG_CONTROL1, WADA_BIT)?;
      self.write_register(REG_WEEKDAY_DATE_ALARM,
                          if match_day { bcd_day }
                          else { ALARM_NO_WATCH_FLAG | bcd_day })?;
    }
    // Clear AF again in case the above setting process immediately triggered the alarm
    self.clear_reg_bits(REG_STATUS, ALARM_FLAG_BIT)?;

    Ok(())
  }

  pub fn get_alarm_datetime_wday_matches(&mut self)
    -> Result<(NaiveDateTime, u8, bool, bool, bool), E> {

    let raw_day = self.read_register(REG_WEEKDAY_DATE_ALARM)?;
    let match_day = 0 == (raw_day & ALARM_NO_WATCH_FLAG);
    let day = Self::bcd_to_bin(0x7F & raw_day);

    let raw_hour = self.read_register(REG_HOURS_ALARM)?;
    let match_hour = 0 == (raw_hour & ALARM_NO_WATCH_FLAG);
    let hour = Self::bcd_to_bin(0x7F & raw_hour);

    let raw_minutes = self.read_register(REG_MINUTES_ALARM)?;
    let match_minutes = 0 == (raw_minutes & ALARM_NO_WATCH_FLAG);
    let minutes = Self::bcd_to_bin(0x7F & raw_minutes);

    let mut weekday: u8 = INVALID_WEEKDAY;

    let wada_state = self.read_register(REG_CONTROL1)? & WADA_BIT;

    let dt =
      if 0 == wada_state {
        // weekday alarm
        weekday = day;
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
    if set {
      self.set_reg_bits(reg, bits)
    }
    else {
      self.clear_reg_bits(reg, bits)
    }
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
  fn get_event_count_and_datetime(&mut self) -> Result<(u32, NaiveDateTime), Self::Error>;

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
  /// that the caller is setting the RTC datetime to values within its range (from 1970 to 2099).
  /// Setting values beyond 2099 (until 2106) might succeed but the RTC doesn't support
  /// leap year corrections beyond 2099.
  fn set_datetime(&mut self, datetime: &NaiveDateTime) -> Result<(), Self::Error> {
    let unix_timestamp: u32 = datetime.timestamp().try_into().unwrap();
    self.set_unix_time(unix_timestamp)
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
      // Initialize bits TSE and EIE to 0.
      // 2. Select TSOW (0 or 1), clear EVF and BSF.
      // 3. Write 1 to TSR bit, to reset all Time Stamp registers to 00h. Bit TSR always returns 0 when read.
      // 4. Select the External Event Interrupt function (TSS = 0) or the Automatic Backup Switchover Interrupt
      // function (TSS = 1) as time stamp source and initialize the appropriate function (see EXTERNAL EVENT
      //                                                                                  INTERRUPT FUNCTION or AUTOMATIC BACKUP SWITCHOVER INTERRUPT FUNCTION).
      //   5. Set the TSE bit to 1 to enable the Time Stamp function.

      // Initialize bits TSE and EIE to 0.
      self.clear_reg_bits(REG_CONTROL2, TIME_STAMP_ENABLE_BIT)?;
      self.clear_reg_bits(REG_CONTROL2, EVENT_INT_ENABLE_BIT)?;
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

  fn get_event_count_and_datetime(&mut self) -> Result<(u32, NaiveDateTime), Self::Error> {
    // Read the seven raw Time Stamp Function registers in one go
    let mut read_buf:[u8;7] = [0u8;7];
    self.read_multi_registers(REG_COUNT_EVENTS_TS, &mut read_buf)?;

    // Convert BCD values to binary
    let count = read_buf[0]; // Count is already in binary

    let dt = {
      if count > 0 {
        let seconds = Self::bcd_to_bin(read_buf[1]);
        let minutes = Self::bcd_to_bin(read_buf[2]);
        let hours = Self::bcd_to_bin(read_buf[3]);
        let date = Self::bcd_to_bin(read_buf[4]);
        let month = Self::bcd_to_bin(read_buf[5]);
        let year = Self::bcd_to_bin(read_buf[6]);
        NaiveDate::from_ymd_opt(year as i32, month as u32, date as u32)
        .expect("YMD")
          .and_hms_opt(hours as u32, minutes as u32, seconds as u32)
          .expect("HMS")
      }
      else {
        NaiveDateTime::from_timestamp_opt(0,0).unwrap()
      }
    };


    Ok((count.into(), dt))
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

  #[test]
  fn test_set_time() {
    let expectations = [
      I2cTrans::write(RV3028_ADDRESS, vec![REG_HOURS, RV3028::<I2cMock>::bin_to_bcd(23)]),
      I2cTrans::write(RV3028_ADDRESS, vec![REG_MINUTES, RV3028::<I2cMock>::bin_to_bcd(59)]),
      I2cTrans::write(RV3028_ADDRESS, vec![REG_SECONDS, RV3028::<I2cMock>::bin_to_bcd(58)]),
    ];
    let mock = I2cMock::new(&expectations);
    let mut rv3028 = RV3028::new(mock);
    rv3028.set_time(23, 59, 58).unwrap();
  }

  #[test]
  fn test_get_time() {
    let expectations = [
      I2cTrans::write_read(RV3028_ADDRESS, vec![REG_HOURS], vec![RV3028::<I2cMock>::bin_to_bcd(23)]),
      I2cTrans::write_read(RV3028_ADDRESS, vec![REG_MINUTES], vec![RV3028::<I2cMock>::bin_to_bcd(59)]),
      I2cTrans::write_read(RV3028_ADDRESS, vec![REG_SECONDS], vec![RV3028::<I2cMock>::bin_to_bcd(58)]),
    ];
    let mock = I2cMock::new(&expectations);
    let mut rv3028 = RV3028::new(mock);
    let (hours, minutes, seconds) = rv3028.get_time().unwrap();
    assert_eq!(hours, 23);
    assert_eq!(minutes, 59);
    assert_eq!(seconds, 58);
  }

  // #[test]
  // fn test_set_alarm_minutes() {
  //   let expectations = [I2cTrans::write(RV3028_ADDRESS, vec![ REG_MINUTES_ALARM, RV3028::<I2cMock>::bin_to_bcd(15)])];
  //   let mock = I2cMock::new(&expectations);
  //   let mut rv3028 = RV3028::new(mock);
  //   rv3028.set_alarm_minutes(15).unwrap();
  // }

  // #[test]
  // fn test_get_alarm_minutes() {
  //   let expectations = [
  //     I2cTrans::write_read(RV3028_ADDRESS, vec![REG_MINUTES_ALARM], vec![RV3028::<I2cMock>::bin_to_bcd(15)]),
  //   ];
  //   let mock = I2cMock::new(&expectations);
  //   let mut rv3028 = RV3028::new(mock);
  //   assert_eq!(rv3028.get_alarm_minutes().unwrap(), 15);
  // }
  //
  // //TODO similar tests for set_alarm_hours, get_alarm_hours, get_alarm_weekday_date
  //
  // #[test]
  // fn test_set_alarm_weekday_date() {
  //   let expectations = [I2cTrans::write(RV3028_ADDRESS, vec![REG_WEEKDAY_DATE_ALARM, RV3028::<I2cMock>::bin_to_bcd(2)])];
  //   let mock = I2cMock::new(&expectations);
  //   let mut rv3028 = RV3028::new(mock);
  //   rv3028.set_alarm_weekday_date(2, true).unwrap();
  // }
  //
  #[test]
  fn test_set_year_month_day() {
    let expectations = [
      I2cTrans::write(RV3028_ADDRESS, vec![REG_YEAR, RV3028::<I2cMock>::bin_to_bcd(23)]),
      I2cTrans::write(RV3028_ADDRESS, vec![REG_MONTH, RV3028::<I2cMock>::bin_to_bcd(12)]),
      I2cTrans::write(RV3028_ADDRESS, vec![REG_DATE, RV3028::<I2cMock>::bin_to_bcd(31)]),
    ];
    let mock = I2cMock::new(&expectations);
    let mut rv3028 = RV3028::new(mock);
    rv3028.set_year_month_day(23, 12, 31).unwrap();
  }


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



}

