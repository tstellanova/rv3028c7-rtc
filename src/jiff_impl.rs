//! Support for the [`jiff`] date and time types, enabled by the `jiff` feature.

use embedded_hal::i2c::I2c;
use jiff::civil::{self, Weekday};
use jiff::tz::Offset;
use jiff::Timestamp;

use crate::{AlarmParts, DateTimeParts, Error, RV3028};

/// Date and time, alarm and event timestamp access using [`jiff`] types.
///
/// The RTC has no notion of time zones, so all values are [`civil::DateTime`],
/// which the RTC stores as UTC.
pub trait JiffDateTimeAccess {
  /// Error type
  type Error;

  /// Read the date and time from the RTC's unix time counter.
  ///
  /// This particular RTC's timestamps wrap at 0xFFFF_FFFF, around the year 2106.
  /// It doesn't support:
  /// - years prior to 1970
  /// - leap year calculations past 2099
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  ///
  /// [`Error::InconsistentRead`] if no consecutive counter reads agree
  /// within four reads total. Individual transaction timeouts remain the
  /// HAL's responsibility.
  fn datetime(&mut self) -> Result<civil::DateTime, Self::Error>;

  /// Set the date and time: both the unix time counter and all internal BCD registers.
  ///
  /// Dates outside 2000 to 2099 return [`Error::InvalidData`] before any bus access.
  /// The RTC doesn't support leap year corrections beyond 2099,
  /// and the internal Year BCD register only runs from 0..99 (for 2000..2099).
  /// Subsecond values are ignored.
  /// This method resets the internal prescaler pipeline, which means that
  /// subsecond counters are zeroed, when it writes to the Seconds register.
  /// This assists with clock synchronization with external clocks.
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  ///
  /// [`Error::InvalidData`] if the year is outside 2000–2099.
  /// Validation happens before any bus access. Subseconds are ignored.
  fn set_datetime(&mut self, datetime: &civil::DateTime) -> Result<(), Self::Error>;

  /// All-in-one method to set an alarm:
  /// See the App Note section "Procedure to use the Alarm Interrupt"
  /// Note only date/weekday, hour, minute are supported
  /// - If `weekday` is provided then it'll setup a weekday alarm rather than date alarm
  /// - `match_day` indicates whether the day (or weekday) should be matched for the alarm
  /// - `match_hour` indicates whether the hour should be matched for the alarm
  /// - `match_minute` indicates whether the minutes should be matched for the alarm
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  fn set_alarm(&mut self, datetime: &civil::DateTime, weekday: Option<Weekday>,
               match_day: bool, match_hour: bool, match_minute: bool) -> Result<(), Self::Error>;

  /// Read the alarm settings
  /// Matches are flag settings for whether the alarm should match day, hour, minute.
  /// The date part of the returned datetime is 1970-01-01 for a weekday alarm,
  /// or 1970-01-`day` for a date alarm.
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  ///
  /// [`Error::InvalidData`] if an enabled alarm field contains invalid
  /// BCD or an out-of-range value. Invalid disabled fields are replaced
  /// with zero (or day 1 for a date alarm).
  fn get_alarm_datetime_wday_matches(&mut self)
    -> Result<(civil::DateTime, Option<Weekday>, bool, bool, bool), Self::Error>;

  /// Get event count -- the number of events that have been logged since enabling logging
  /// Returns the count of events since last reset, and the datetime of one event
  ///
  /// # Errors
  ///
  /// [`Error::I2c`] if an I2C transaction fails, including mux selection.
  ///
  /// [`Error::InvalidData`] if a nonzero event count accompanies invalid
  /// BCD or an invalid calendar date/time. Timestamp fields are ignored
  /// when the event count is zero.
  fn get_event_count_and_datetime(&mut self) -> Result<(u32, Option<civil::DateTime>), Self::Error>;
}

impl<I2C> JiffDateTimeAccess for RV3028<I2C>
  where
    I2C: I2c,
{
  type Error = Error<I2C::Error>;

  fn datetime(&mut self) -> Result<civil::DateTime, Self::Error> {
    let unix_timestamp = self.get_unix_time_blocking()?;
    // every u32 timestamp is within jiff's supported range
    let timestamp = Timestamp::from_second(unix_timestamp.into()).map_err(|_| Error::InvalidData)?;
    Ok(Offset::UTC.to_datetime(timestamp))
  }

  fn set_datetime(&mut self, datetime: &civil::DateTime) -> Result<(), Self::Error> {
    self.set_datetime_parts(&DateTimeParts {
      year: datetime.year() as i32,
      month: datetime.month() as u8,
      day: datetime.day() as u8,
      hour: datetime.hour() as u8,
      minute: datetime.minute() as u8,
      second: datetime.second() as u8,
    })
  }

  fn set_alarm(&mut self, datetime: &civil::DateTime, weekday: Option<Weekday>,
               match_day: bool, match_hour: bool, match_minute: bool) -> Result<(), Self::Error> {
    let day = match weekday {
      Some(inner_weekday) => inner_weekday.to_monday_zero_offset() as u8,
      None => datetime.day() as u8,
    };
    self.set_alarm_parts(&AlarmParts {
      day,
      hour: datetime.hour() as u8,
      minute: datetime.minute() as u8,
      weekday_alarm: weekday.is_some(),
      match_day,
      match_hour,
      match_minute,
    })
  }

  fn get_alarm_datetime_wday_matches(&mut self)
    -> Result<(civil::DateTime, Option<Weekday>, bool, bool, bool), Self::Error> {
    let alarm = self.get_alarm_parts()?;
    let mut weekday = None;

    let day =
      if alarm.weekday_alarm {
        weekday = Some(Weekday::from_monday_zero_offset(alarm.day as i8).map_err(|_| Error::InvalidData)?);
        1
      }
      else {
        alarm.day as i8
      };
    let dt = civil::DateTime::new(1970, 1, day, alarm.hour as i8, alarm.minute as i8, 0, 0)
      .map_err(|_| Error::InvalidData)?;

    Ok((dt, weekday, alarm.match_day, alarm.match_hour, alarm.match_minute))
  }

  fn get_event_count_and_datetime(&mut self) -> Result<(u32, Option<civil::DateTime>), Self::Error> {
    let (count, parts) = self.get_event_count_and_parts()?;
    let odt = parts.map(|dt|
      civil::DateTime::new(dt.year as i16, dt.month as i8, dt.day as i8,
                           dt.hour as i8, dt.minute as i8, dt.second as i8, 0)
        .map_err(|_| Error::InvalidData)).transpose()?;
    Ok((count, odt))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{
    REG_CONTROL1, REG_COUNT_EVENTS_TS, REG_HOURS_ALARM, REG_MINUTES_ALARM, REG_SECONDS, REG_STATUS,
    REG_UNIX_TIME_0, REG_WEEKDAY, REG_WEEKDAY_DATE_ALARM, RV3028_ADDRESS,
  };
  use embedded_hal_mock::eh1::i2c::{Mock as I2cMock, Transaction as I2cTrans};
  use std::vec;

  const MUX_ADDR: u8 = 0x70;
  const MUX_CHAN: u8 = 0b0000_0001;

  // 2024-03-15 is a Friday (weekday 4, counting from Monday = 0)
  fn test_datetime() -> civil::DateTime {
    civil::date(2024, 3, 15).at(13, 45, 30, 0)
  }

  #[test]
  fn test_datetime_from_unix_time() {
    let before = 1_710_510_329u32.to_le_bytes();
    let unix_bytes = 1_710_510_330u32.to_le_bytes();
    let expectations = [
      // the counter ticks between the first two reads, so they are read again
      I2cTrans::write_read(RV3028_ADDRESS, vec![REG_UNIX_TIME_0], before.to_vec()),
      I2cTrans::write_read(RV3028_ADDRESS, vec![REG_UNIX_TIME_0], unix_bytes.to_vec()),
      I2cTrans::write_read(RV3028_ADDRESS, vec![REG_UNIX_TIME_0], unix_bytes.to_vec()),
    ];
    let mut rv3028 = RV3028::new(I2cMock::new(&expectations));
    assert_eq!(JiffDateTimeAccess::datetime(&mut rv3028).unwrap(), test_datetime());
    rv3028.i2c.done();
  }

  #[test]
  fn test_set_datetime() {
    let unix_bytes = 1_710_510_330u32.to_le_bytes();
    let expectations = [
      I2cTrans::write(MUX_ADDR, vec![MUX_CHAN]),
      I2cTrans::write(RV3028_ADDRESS, vec![
        REG_UNIX_TIME_0, unix_bytes[0], unix_bytes[1], unix_bytes[2], unix_bytes[3]]),
      // weekday, day, month, year
      I2cTrans::write(RV3028_ADDRESS, vec![REG_WEEKDAY, 0x04, 0x15, 0x03, 0x24]),
      // seconds, minutes, hours
      I2cTrans::write(RV3028_ADDRESS, vec![REG_SECONDS, 0x30, 0x45, 0x13]),
    ];
    let mut rv3028 = RV3028::new_with_mux(I2cMock::new(&expectations), MUX_ADDR, MUX_CHAN);
    // subseconds are ignored
    JiffDateTimeAccess::set_datetime(&mut rv3028, &civil::date(2024, 3, 15).at(13, 45, 30, 999))
      .unwrap();
    rv3028.i2c.done();
  }

  #[test]
  fn test_set_date_alarm() {
    let expectations = [
      // clear AF
      I2cTrans::write_read(RV3028_ADDRESS, vec![REG_STATUS], vec![0xFF]),
      I2cTrans::write(RV3028_ADDRESS, vec![REG_STATUS, 0xFB]),
      // set WADA for a date alarm
      I2cTrans::write_read(RV3028_ADDRESS, vec![REG_CONTROL1], vec![0x00]),
      I2cTrans::write(RV3028_ADDRESS, vec![REG_CONTROL1, 0x20]),
      I2cTrans::write(RV3028_ADDRESS, vec![REG_MINUTES_ALARM, 0x45]),
      // hour is not matched
      I2cTrans::write(RV3028_ADDRESS, vec![REG_HOURS_ALARM, 0x93]),
      I2cTrans::write(RV3028_ADDRESS, vec![REG_WEEKDAY_DATE_ALARM, 0x15]),
      // clear AF again
      I2cTrans::write_read(RV3028_ADDRESS, vec![REG_STATUS], vec![0x04]),
      I2cTrans::write(RV3028_ADDRESS, vec![REG_STATUS, 0x00]),
    ];
    let mut rv3028 = RV3028::new(I2cMock::new(&expectations));
    rv3028.set_alarm(&test_datetime(), None, true, false, true).unwrap();
    rv3028.i2c.done();
  }

  #[test]
  fn test_set_weekday_alarm() {
    let expectations = [
      // clear AF
      I2cTrans::write_read(RV3028_ADDRESS, vec![REG_STATUS], vec![0x00]),
      I2cTrans::write(RV3028_ADDRESS, vec![REG_STATUS, 0x00]),
      // clear WADA for a weekday alarm
      I2cTrans::write_read(RV3028_ADDRESS, vec![REG_CONTROL1], vec![0x20]),
      I2cTrans::write(RV3028_ADDRESS, vec![REG_CONTROL1, 0x00]),
      I2cTrans::write(RV3028_ADDRESS, vec![REG_MINUTES_ALARM, 0x45]),
      I2cTrans::write(RV3028_ADDRESS, vec![REG_HOURS_ALARM, 0x13]),
      // Wednesday, not matched
      I2cTrans::write(RV3028_ADDRESS, vec![REG_WEEKDAY_DATE_ALARM, 0x82]),
      // clear AF again
      I2cTrans::write_read(RV3028_ADDRESS, vec![REG_STATUS], vec![0x00]),
      I2cTrans::write(RV3028_ADDRESS, vec![REG_STATUS, 0x00]),
    ];
    let mut rv3028 = RV3028::new(I2cMock::new(&expectations));
    rv3028.set_alarm(&test_datetime(), Some(Weekday::Wednesday), false, true, true).unwrap();
    rv3028.i2c.done();
  }

  #[test]
  fn test_get_date_alarm() {
    let expectations = [
      I2cTrans::write_read(RV3028_ADDRESS, vec![REG_WEEKDAY_DATE_ALARM], vec![0x15]),
      I2cTrans::write_read(RV3028_ADDRESS, vec![REG_HOURS_ALARM], vec![0x93]),
      I2cTrans::write_read(RV3028_ADDRESS, vec![REG_MINUTES_ALARM], vec![0x45]),
      I2cTrans::write_read(RV3028_ADDRESS, vec![REG_CONTROL1], vec![0x20]),
    ];
    let mut rv3028 = RV3028::new(I2cMock::new(&expectations));
    let (dt, weekday, match_day, match_hour, match_minute) =
      rv3028.get_alarm_datetime_wday_matches().unwrap();
    rv3028.i2c.done();
    assert_eq!(dt, civil::date(1970, 1, 15).at(13, 45, 0, 0));
    assert_eq!(weekday, None);
    assert_eq!((match_day, match_hour, match_minute), (true, false, true));
  }

  #[test]
  fn test_get_weekday_alarm() {
    let expectations = [
      I2cTrans::write_read(RV3028_ADDRESS, vec![REG_WEEKDAY_DATE_ALARM], vec![0x82]),
      I2cTrans::write_read(RV3028_ADDRESS, vec![REG_HOURS_ALARM], vec![0x13]),
      I2cTrans::write_read(RV3028_ADDRESS, vec![REG_MINUTES_ALARM], vec![0x45]),
      I2cTrans::write_read(RV3028_ADDRESS, vec![REG_CONTROL1], vec![0x00]),
    ];
    let mut rv3028 = RV3028::new(I2cMock::new(&expectations));
    let (dt, weekday, match_day, match_hour, match_minute) =
      rv3028.get_alarm_datetime_wday_matches().unwrap();
    rv3028.i2c.done();
    assert_eq!(dt, civil::date(1970, 1, 1).at(13, 45, 0, 0));
    assert_eq!(weekday, Some(Weekday::Wednesday));
    assert_eq!((match_day, match_hour, match_minute), (false, true, true));
  }

  #[test]
  fn test_get_event_count_and_datetime() {
    let expectations = [
      // count, seconds, minutes, hours, date, month, year
      I2cTrans::write_read(RV3028_ADDRESS, vec![REG_COUNT_EVENTS_TS],
                           vec![3, 0x30, 0x45, 0x13, 0x15, 0x03, 0x24]),
      I2cTrans::write_read(RV3028_ADDRESS, vec![REG_COUNT_EVENTS_TS],
                           vec![0, 0, 0, 0, 0, 0, 0]),
    ];
    let mut rv3028 = RV3028::new(I2cMock::new(&expectations));
    assert_eq!(rv3028.get_event_count_and_datetime().unwrap(), (3, Some(test_datetime())));
    assert_eq!(rv3028.get_event_count_and_datetime().unwrap(), (0, None));
    rv3028.i2c.done();
  }
  #[test]
  fn unsupported_years_return_errors_without_bus_access() {
    let mut rtc = RV3028::new(I2cMock::new(&[]));
    for year in [1969, 1999, 2100, 2200] {
      let dt = civil::date(year, 1, 1).at(0, 0, 0, 0);
      assert_eq!(JiffDateTimeAccess::set_datetime(&mut rtc, &dt), Err(Error::InvalidData));
    }
    rtc.i2c.done();
  }

}
