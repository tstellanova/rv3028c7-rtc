#![cfg_attr(not(test), no_std)]


pub use rtcc::{
  DateTimeAccess, NaiveDateTime
};



use embedded_hal::blocking::i2c::{Write, Read, WriteRead};

// i2c address of the device (7-bit)
const RV3028_ADDRESS: u8 = 0xA4 >> 1;

// Register addresses
const REG_SECONDS: u8 = 0x00;
const REG_MINUTES: u8 = 0x01;
const REG_HOURS: u8 = 0x02;


/// Holds the current day of the week.
/// Each value represents one weekday that is assigned by the user.
/// Values will range from 0 to 6.
/// The weekday counter is simply a 3-bit counter which counts up to 6 and then resets to 0.
const REG_WEEKDAY: u8 = 0x03;


/// Holds the current day of the month, in two binary coded decimal (BCD) digits.
/// Values will range from 01 to 31.
/// Leap years are correctly handled from 2000 to 2099.
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
// - If the WADA bit is 0 (Bit 5 in Register 0Fh),
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
// const REG_STATUS: u8 = 0x0E;

// This register is used to specify the target for the Alarm Interrupt function
// and the Periodic Time Update Interrupt function
// and to select or set operations for the Periodic Countdown Timer.
// const REG_CONTROL1:u8  = 0x0F;

// This register is used to control:
// - interrupt event output for the INT̅ pin
// - stop/start status of clock and calendar operations
// - interrupt controlled clock output on CLKOUT pin
// - hour mode and time stamp enable
// const REG_CONTROL2:u8 = 0x10;

// First address of "Unix Time Counter"
const REG_UNIX_TIME_0: u8 = 0x1B;
// const REG_UNIX_TIME_1: u8 = 0x1C;
// const REG_UNIX_TIME_2: u8 = 0x1D;
// const REG_UNIX_TIME_3: u8 = 0x1E;

// Control1 register bits:
// const EERD_BIT: u8 = 1 << 3;
// const USEL_BIT: u8 = 1 << 4;
// const WADA_BIT: u8 = 1 << 5;

// Status register bits:
// const EEBUSY_BIT: u8 = 1 << 7;

// EEPROM register addresses and commands
pub const EEPROM_ADDRESS: u8 = 0x37;
pub const EEPROM_CMD_READ: u8 = 0x00;
pub const EEPROM_CMD_WRITE: u8 = 0x01;

// EEPROM register bits:
const TRICKLE_CHARGE_ENABLE_BIT: u8 = 1 << 5; // TCE bit
// const TRICKLE_CHARGE_RESISTANCE_BITS_3K: u8 = 0b00; //TCR bit
// const TRICKLE_CHARGE_RESISTANCE_BITS_5K: u8 = 0b01;
// const TRICKLE_CHARGE_RESISTANCE_BITS_9K: u8 = 0b10;
// const TRICKLE_CHARGE_RESISTANCE_BITS_15K: u8 = 0b11;

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

  /// Converts a binary value to BCD format
  fn bin_to_bcd(value: u8) -> u8 {
    ((value / 10) << 4) | (value % 10)
  }

  /// Converts a BCD value to binary format
  fn bcd_to_bin(value: u8) -> u8 {
    ((value & 0xF0) >> 4) * 10 + (value & 0x0F)
  }

  /// If using an i2c mux, tell the mux to select our channel
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


  fn set_reg_bits(&mut self, reg: u8, bits: u8) -> Result<(), E> {
    let mut reg_val = self.read_register(reg)?;
    reg_val |= bits; // Set bits that are high
    self.write_register(reg, reg_val)
  }

  // clear specific bits in a register -- all bits must be high
  // that you desire to be cleared
  fn clear_reg_bits(&mut self, reg: u8, bits: u8) -> Result<(), E> {
    let mut reg_val = self.read_register(reg)?;
    reg_val &= !(bits); // Clear  bits that are high
    self.write_register(reg, reg_val)
  }

  /// Disable trickle charging of the VBACKUP power source
  pub fn disable_trickle_charge(&mut self) -> Result<(), E> {
    self.clear_reg_bits(EEPROM_ADDRESS, TRICKLE_CHARGE_ENABLE_BIT)
  }

  /// Enable trickle charging of the VBACKUP power source
  pub fn enable_trickle_charge(&mut self) -> Result<(), E> {
    self.set_reg_bits(EEPROM_ADDRESS, TRICKLE_CHARGE_ENABLE_BIT)
  }

  /// Get the current value of the EEPROM register
  pub fn get_eeprom_reg_value(&mut self) -> Result<u8, E> {
    let reg_val = self.read_register(EEPROM_ADDRESS)?;
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

  /// Set the minutes for the alarm
  pub fn set_alarm_minutes(&mut self, minutes: u8) -> Result<(), E> {
    let bcd_minutes = Self::bin_to_bcd(minutes);
    self.write_register( REG_MINUTES_ALARM , bcd_minutes)
  }

  /// Get the minutes for the alarm
  pub fn get_alarm_minutes(&mut self) -> Result<u8, E> {
    let bcd_minutes = self.read_register(REG_MINUTES_ALARM)?;
    Ok(Self::bcd_to_bin(bcd_minutes))
  }

  /// Set the hours for the alarm
  pub fn set_alarm_hours(&mut self, hours: u8) -> Result<(), E> {
    let bcd_hours = Self::bin_to_bcd(hours);
    self.write_register(REG_HOURS_ALARM , bcd_hours)
  }

  /// Get the hours for the alarm
  pub fn get_alarm_hours(&mut self) -> Result<u8, E> {
    let bcd_hours = self.read_register(REG_HOURS_ALARM)?;
    Ok(Self::bcd_to_bin(bcd_hours))
  }

  /// Set the weekday/date for the alarm
  /// - `is_weekday` is true for weekday, false for date
  pub fn set_alarm_weekday_date(&mut self, value: u8, is_weekday: bool) -> Result<(), E> {
    let bcd_value = Self::bin_to_bcd(value);
    let alarm_value = if is_weekday { bcd_value } else { bcd_value | 0x40 };
    self.write_register(REG_WEEKDAY_DATE_ALARM , alarm_value)
  }

  /// Get the weekday/date for the alarm
  /// - Returns (value, is_weekday): `is_weekday` is true if the alarm is set for a weekday
  pub fn get_alarm_weekday_date(&mut self) -> Result<(u8, bool), E> {
    let alarm_value = self.read_register(REG_WEEKDAY_DATE_ALARM)?;
    let is_weekday = (alarm_value & 0x40) == 0;
    let value = if is_weekday { alarm_value & 0x3F } else { alarm_value & 0x1F };
    Ok((Self::bcd_to_bin(value), is_weekday))
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
    self.select_mux_channel()?;
    let mut read_buf = [0u8; 4];
    self.i2c.write_read(RV3028_ADDRESS, &[REG_UNIX_TIME_0], &mut read_buf)?;
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

}

impl<I2C, E> DateTimeAccess for  RV3028<I2C>
  where
    I2C: Write<Error = E> + Read<Error = E> + WriteRead<Error = E>
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

  #[test]
  fn test_set_alarm_minutes() {
    let expectations = [I2cTrans::write(RV3028_ADDRESS, vec![ REG_MINUTES_ALARM, RV3028::<I2cMock>::bin_to_bcd(15)])];
    let mock = I2cMock::new(&expectations);
    let mut rv3028 = RV3028::new(mock);
    rv3028.set_alarm_minutes(15).unwrap();
  }

  #[test]
  fn test_get_alarm_minutes() {
    let expectations = [
      I2cTrans::write_read(RV3028_ADDRESS, vec![REG_MINUTES_ALARM], vec![RV3028::<I2cMock>::bin_to_bcd(15)]),
    ];
    let mock = I2cMock::new(&expectations);
    let mut rv3028 = RV3028::new(mock);
    assert_eq!(rv3028.get_alarm_minutes().unwrap(), 15);
  }

  //TODO similar tests for set_alarm_hours, get_alarm_hours, get_alarm_weekday_date

  #[test]
  fn test_set_alarm_weekday_date() {
    let expectations = [I2cTrans::write(RV3028_ADDRESS, vec![REG_WEEKDAY_DATE_ALARM, RV3028::<I2cMock>::bin_to_bcd(2)])];
    let mock = I2cMock::new(&expectations);
    let mut rv3028 = RV3028::new(mock);
    rv3028.set_alarm_weekday_date(2, true).unwrap();
  }

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

