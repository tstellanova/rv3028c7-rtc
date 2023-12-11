#![cfg_attr(not(test), no_std)]


pub use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Weekday};
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

// First address of the user "write password" register (Password PW)
const REG_USER_PASSWORD_0: u8 = 0x21;


// EEPROM RAM mirror register addresses and commands

// EEADDR register -- EE Address: the target address within the eeprom for operations
const REG_EEPROM_EE_ADDRESS: u8 = 0x25;
// EEDATA
const REG_EEPROM_EE_DATA: u8 = 0x26;
// EECMD -- the command to operate on
const REG_EEPROM_EE_CMD: u8 = 0x27;

// EEPWE / EEPROM Password Enable
const REG_EEPROM_PASSWORD_ENABLE: u8 = 0x30;
// EEPROM Password 0 (first of four)
const REG_EEPROM_PASSWORD_0: u8 = 0x31;
// EEPROM CLKOUT control register
const REG_EEPROM_CLKOUT: u8 = 0x35;
// RAM mirror of EEPROM config values
const REG_EEPROM_BACKUP_CONFIG: u8 = 0x37;



// REG_CONTROL1 "Control 1" register bits:
#[repr(u8)]
enum RegControl1Bits {
  // TRPT / Timer Repeat bit. Single or Repeat countdown timer
  TimerRepeatBit =  1 << 7,
  // WADA / Weekday Alarm / Date Alarm selection bit
  WadaBit = 1 << 5,
  //  USEL / Update Interrupt Select bit. Seconds or minutes.
  UselBit = 1 << 4,
  // EERD / EEPROM Memory Refresh Disable bit. When 1, disables the automatic refresh of the
  // Configuration Registers from the EEPROM Memory
  EeerdBit= 1 << 3,
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
  // EEBUSY_BIT
  EepromBusyBit = 1 << 7,
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

pub const TS_EVENT_SOURCE_EVI: u8 = 0; /// Event log source is external interrupt EVI (default)
pub const TS_EVENT_SOURCE_BSF: u8 = 1; /// Event log source is backup power switchover

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
enum RegEepromBackupBits {
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

#[derive(Clone, Copy)]
pub enum TrickleChargeCurrentLimiter {
  Ohms3k = 0b00,
  Ohms5k = 0b01,
  Ohms9k = 0b10,
  Ohms15k = 0b11,
}

/// Used for controlling the CLKOUT rate via the
/// FD register (EEPROM 35h)
#[repr(u8)]
pub enum ClockoutRate {
  /// 32.768 kHz –Default value on delivery
  Clkout32768Khz = 0b000,
  Clkout8192Hz = 0b001,
  Clkout1024Hz = 0b010,
  Clkout64Hz = 0b011,
  Clkout1Hz = 0b101,
  /// Predefined periodic countdown timer (PCT) interrupt
  ClkoutPct = 0b110,
  /// All Frequency selection bits. Value sets CLKOUT = Low
  ClkoutFreqSelectionBits = 0b111,
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


  fn write_register_raw(&mut self, reg: u8, data: u8) -> Result<(), E> {
    self.i2c.write(RV3028_ADDRESS, &[reg, data])
  }

  fn read_register_raw(&mut self, reg: u8) -> Result<u8, E> {
    let mut buf = [0];
    self.i2c.write_read(RV3028_ADDRESS, &[reg], &mut buf)?;
    Ok(buf[0])
  }


  /// Check whether the Power On Reset flag is set.
  /// If this flag is cleared (set to zero) beforehand,
  /// indicates a voltage drop below VPOR.
  /// If this flag is set, the data in the device RAM registers are no longer valid
  /// and all registers must be (re)initialized.
  /// The flag value 1 is retained until a 0 is written by the user.
  /// At power up (POR) the value is set to 1, the user has to write 0 to the flag to use it.
  pub fn check_and_clear_power_on_reset(&mut self) -> Result<bool, E>  {
    let flag_set = 0 != self.check_and_clear_bits(
      REG_STATUS, RegStatusBits::PowerOnResetFlagBit as u8)?;
    Ok(flag_set)
  }

  /// Check whether an external event has been detected
  /// (an appropriate input signal on the EVI pin)
  pub fn check_and_clear_ext_event(&mut self)-> Result<bool, E>  {
    let flag_set = 0 != self.check_and_clear_bits(
      REG_STATUS, RegStatusBits::EventFlagBit as u8)?;
    Ok(flag_set)
  }

  /// Check whether an Automatic Backup Switchover event
  /// (switching over to backup power source, Vbackup)
  /// has taken place, as indicated by the
  /// Backup Switch Flag (BSF).
  /// This method also clears the BSF flag.
  /// See AUTOMATIC BACKUP SWITCHOVER FUNCTION.
  pub fn check_and_clear_backup_event(&mut self)-> Result<bool, E>  {
    let flag_set = 0 != self.check_and_clear_bits(
      REG_STATUS, RegStatusBits::BackupSwitchFlag as u8)?;
    Ok(flag_set)
  }

  /// Check whether a switchover to Vbackup power supply has
  /// occurred at least once.
  pub fn check_backup_event_flag(&mut self)-> Result<bool, E>  {
    self.check_bits_nonzero(REG_STATUS, RegStatusBits::BackupSwitchFlag as u8)
  }


  // Is the EEPROM currently busy
  fn is_eeprom_busy_raw(&mut self) -> Result<bool, E> {
    let status = self.read_register_raw(REG_STATUS)?;
    Ok(status & (RegStatusBits::EepromBusyBit as u8) != 0)
  }

  // Enable or disable EEPROM auto refresh from the RAM mirror
  fn toggle_auto_eeprom_refresh_raw(&mut self, disable: bool) -> Result<(), E> {
    self.set_or_clear_reg_bits_raw(
      REG_CONTROL1, RegControl1Bits::EeerdBit as u8, disable)
  }


  // // write a single EEPROM register
  // // - `ee_address` The memory address within the eeprom
  // fn eeprom_write_raw(&mut self, ee_address: u8, data: u8) -> Result<(), E> {
  //   self.toggle_auto_eeprom_refresh_raw(true)?;
  //   while self.is_eeprom_busy_raw()? {}
  //   self.write_register_raw(REG_EEPROM_EE_ADDRESS, ee_address)?;
  //   self.write_register_raw(REG_EEPROM_EE_DATA, data)?;    // the data to be written to ee_address
  //   self.write_register_raw(REG_EEPROM_EE_CMD, 0x00)?; // first cmd must be zero
  //   let res= self.write_register_raw(REG_EEPROM_EE_CMD, 0x21); //write a single byte
  //   while self.is_eeprom_busy_raw()? {}
  //   self.toggle_auto_eeprom_refresh_raw(false)?;
  //   res
  // }
  //
  // // read a single register from EEPROM
  // // - `ee_address` The memory address within the eeprom
  // fn eeprom_read_raw(&mut self, ee_address: u8) -> Result<u8, E> {
  //   self.toggle_auto_eeprom_refresh_raw(true)?;
  //   while self.is_eeprom_busy_raw()? {}
  //   self.write_register_raw(REG_EEPROM_EE_ADDRESS, ee_address)?;
  //   self.write_register_raw(REG_EEPROM_EE_CMD, 0x00)?; // first cmd must be zero
  //   self.write_register_raw(REG_EEPROM_EE_CMD, 0x22)?; // read a single byte
  //   let res = self.read_register_raw(REG_EEPROM_EE_DATA);
  //   while self.is_eeprom_busy_raw()? {}
  //   self.toggle_auto_eeprom_refresh_raw(false)?;
  //   res
  // }

  // read multiple sequential registers from EEPROM
  // - `start_ee_address` The memory address within the eeprom to start reading
  fn eeprom_multi_read_raw(&mut self, start_ee_address: u8, read_buf: &mut [u8]) -> Result<(), E> {
    self.toggle_auto_eeprom_refresh_raw(true)?;
    while self.is_eeprom_busy_raw()? {}
    for i in 0..(read_buf.len() ) {
      let ee_address = start_ee_address + (i as u8);
      self.write_register_raw(REG_EEPROM_EE_ADDRESS, ee_address)?;
      while self.is_eeprom_busy_raw()? {}
      self.write_register_raw(REG_EEPROM_EE_CMD, 0x00)?; // first cmd must be zero
      while self.is_eeprom_busy_raw()? {}
      self.write_register_raw(REG_EEPROM_EE_CMD, 0x22)?; // read a single byte
      while self.is_eeprom_busy_raw()? {}
      read_buf[i] = self.read_register_raw(REG_EEPROM_EE_DATA)?;
    }
    while self.is_eeprom_busy_raw()? {}
    self.toggle_auto_eeprom_refresh_raw(false)?;
    Ok(())
  }

  // Update all of the EEPROM registers from the EEPROM RAM mirror
  fn eeprom_update_all_raw(&mut self) -> Result<(), E> {
    self.toggle_auto_eeprom_refresh_raw(true)?;
    while self.is_eeprom_busy_raw()? {}
    self.write_register_raw(REG_EEPROM_EE_CMD, 0x00)?; // first cmd must be zero
    while self.is_eeprom_busy_raw()? {}
    let res= self.write_register_raw(REG_EEPROM_EE_CMD, 0x11); //update all
    while self.is_eeprom_busy_raw()? {}
    self.toggle_auto_eeprom_refresh_raw(false)?;
    res
  }

  // // Read all of the EEPROM registers into their corresponding RAM mirrors
  // fn eeprom_refresh_all(&mut self) -> Result<(), E> {
  //   self.toggle_auto_eeprom_refresh_raw(true)?;
  //   while self.is_eeprom_busy_raw()? {}
  //
  //   //  writing the command 00h into the register EECMD,
  //   // and then the second command 12h into the register EECMD
  //   // will start the copy of the configuration into the RAM mirror
  //
  //   self.write_register_raw(REG_EEPROM_EE_CMD, 0x00)?; // first cmd must be zero
  //   let res = self.write_register_raw(REG_EEPROM_EE_CMD, 0x12); // refresh all
  //   while self.is_eeprom_busy_raw()? {}
  //   self.toggle_auto_eeprom_refresh_raw(false)?;
  //   res
  // }

  // Set specific bits in a register: "raw" means it skips the mux
  // all bits must be high that you wish to set
  fn set_reg_bits_raw(&mut self, reg: u8, bits: u8) -> Result<(), E> {
    let mut reg_val = self.read_register_raw(reg)?;
    reg_val |= bits; // Set bits that are high
    self.write_register_raw(reg, reg_val)
  }

  // Clears specific bits in a register: "raw" means it skips the mux.
  // All bits must be high that you wish to be cleared
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
  pub fn config_trickle_charge(&mut self, enable: bool,
                               limit_resistance: TrickleChargeCurrentLimiter) -> Result<bool, E>  {
    self.select_mux_channel()?;

    // First disable charging before changing settings
    self.clear_reg_bits_raw(
      REG_EEPROM_BACKUP_CONFIG, RegEepromBackupBits::TrickleChargeEnableBit as u8)?;
    // Reset TCR to 3 kΩ, the factory default, by clearing the TCR bits
    self.clear_reg_bits_raw(
      REG_EEPROM_BACKUP_CONFIG, RegEepromBackupBits::TrickleChargeResistanceBits as u8 )?;

    if enable {
      self.set_reg_bits_raw(
        REG_EEPROM_BACKUP_CONFIG, limit_resistance as u8)?;
      self.set_reg_bits_raw(
        REG_EEPROM_BACKUP_CONFIG, RegEepromBackupBits::TrickleChargeEnableBit as u8)?;
    }

    // confirm the value set
    let conf_val = self.check_bits_nonzero_raw(
      REG_EEPROM_BACKUP_CONFIG, RegEepromBackupBits::TrickleChargeEnableBit as u8)?;
    Ok(conf_val)
  }

  /// Toggle whether the Vbackup power source should be used
  /// when Vdd supply level drops below useful level.
  /// - `enable` enables switching to Vbackup, disables if false
  /// Returns the set value
  pub fn toggle_backup_switchover(&mut self, enable: bool) -> Result<bool, E> {
    self.select_mux_channel()?;
    self.clear_reg_bits_raw(REG_STATUS, RegStatusBits::BackupSwitchFlag as u8)?;

    self.set_or_clear_reg_bits_raw(
      REG_EEPROM_BACKUP_CONFIG, RegEepromBackupBits::BackupSwitchoverDsm as u8, enable)?;
    let conf_val =
      0 != self.read_register_raw(
        REG_EEPROM_BACKUP_CONFIG)? & RegEepromBackupBits::BackupSwitchoverDsm as u8;
    Ok(conf_val)
  }

  /// Disable all clock outputs triggered by interrupts
  pub fn clear_all_int_clockout_bits(&mut self) -> Result<(), E> {
    self.select_mux_channel()?;
    self.clear_reg_bits_raw(REG_CLOCK_INTERRUPT_MASK,
                            RegClockIntMaskBits::ClockoutOnExtEvtBit as u8 |
                              RegClockIntMaskBits::ClockoutOnAlarmBit as u8 |
                            RegClockIntMaskBits::ClockoutOnPctBit as u8 |
                              RegClockIntMaskBits::ClockoutOnUpdateBit as u8)
  }


  /// Enable or disable CLKOUT when alarm triggers
  pub fn toggle_clockout_on_alarm(&mut self, enable: bool) -> Result<(), E> {
    self.set_or_clear_reg_bits(
      REG_CLOCK_INTERRUPT_MASK, RegClockIntMaskBits::ClockoutOnAlarmBit as u8, enable)
  }

  /// Get the current value of the EEPROM backup config from RAM mirror
  pub fn get_eeprom_backup_config(&mut self) -> Result<u8, E> {
    self.select_mux_channel()?;
    let reg_val = self.read_register_raw(REG_EEPROM_BACKUP_CONFIG)?;
    Ok(reg_val)
  }

  /// Enter the password that will unlock the ability to write to "WP" registers
  /// such as the current datetime
  pub fn enter_user_password(&mut self, pass: &[u8; 4]) -> Result<(), E> {
    self.select_mux_channel()?;
    // let mut write_buf: [u8; 5] = [REG_USER_PASSWORD_0, 0,0,0,0];
    // // write_buf[1..5].copy_from_slice(password);
    // write_buf[1..=4].copy_from_slice(password);
    // self.i2c.write(RV3028_ADDRESS, &write_buf)?;

    self.i2c.write(RV3028_ADDRESS,
                   &[REG_USER_PASSWORD_0, pass[0], pass[1], pass[2], pass[3]])

    // Ok(())
  }

  // pub fn set_ancient_password(&mut self) -> Result<(), E> {
  //   let mut write_buf: [u8; 5] = [REG_USER_PASSWORD_0, 187, 254, 237, 208];
  //   self.i2c.write(RV3028_ADDRESS, &write_buf)?;
  //   Ok(())
  // }

  /// Set the "permanent" write protection password that
  /// guards the permission to write to "WP" registers
  /// Warning: This modifies EEPROM settings.
  pub fn set_write_protect_password(&mut self, pass: &[u8; 4], enable: bool) -> Result<(), E> {
    self.select_mux_channel()?;
    // To code a new password, the user has to first enter the current
    // (correct) Password PW (PW = EEPW) into registers 21h to 24h,
    // if the WP-Registers are write protected,
    // and then write a value value ≠ 255  in the EEPWE register to unlock write protection,
    // and then write the new reference password EEPW
    // into the EEPROM registers 31h to 34h
    // and value = 255 in the EEPWE register to enable password function.

    self.i2c.write(RV3028_ADDRESS,
                   &[REG_EEPROM_PASSWORD_0, pass[0], pass[1], pass[2], pass[3]])?;

    self.toggle_write_protect_enabled(enable)?;
    self.eeprom_update_all_raw()?;
    Ok(())
  }

  /// Read back the write-protection password from EEPROM.
  /// Note that this will only succeed (return nonzero values) if
  /// write-protection is already unlocked
  /// (by setting a user password that matches the actual write-protection password)
  pub fn get_write_protect_settings(&mut self) -> Result<(bool, [u8;4]), E> {
    self.select_mux_channel()?;
    let mut read_buf = [0u8;5];
    self.eeprom_multi_read_raw( REG_EEPROM_PASSWORD_ENABLE,  &mut read_buf)?;
    let enabled = 0 != read_buf[0];

    let res: [u8; 4] = [read_buf[1], read_buf[2], read_buf[3], read_buf[4]];
    // res[0..=3].copy_from_slice(&read_buf[1..=4]);

    Ok((enabled, res) )
  }

  /// Check whether the RTC has write-protection enabled for all registers marked with
  /// "WP" for "Write Protected"
  pub fn check_write_protect_enabled(&mut self,) -> Result<bool, E> {
    self.select_mux_channel()?;
    let reg_val = self.read_register_raw( REG_EEPROM_PASSWORD_ENABLE)?;
    Ok( reg_val == 255 )
  }

  /// Enable or disable write protection.
  /// If write protection is enabled, and this method attempts to disable it,
  /// The user password MUST already be set to a value that matches the
  /// EEPROM setting, in order for this method to successfully update.
  /// - returns the (possibly updated) value of write protection.
  pub fn toggle_write_protect_enabled(&mut self, enable: bool) -> Result<bool, E> {
    self.select_mux_channel()?;
    self.write_register_raw( REG_EEPROM_PASSWORD_ENABLE, if enable { 255 } else { 0} )?;
    let reg_val = self.read_register_raw( REG_EEPROM_PASSWORD_ENABLE)?;
    Ok( reg_val == 255 )
  }

  /// Set the value of the User Ram registers (two bytes)
  pub fn set_user_ram(&mut self, data: &[u8; 2]) -> Result<(), E> {
    self.select_mux_channel()?;
    let mut write_buf: [u8; 3] = [0x1F, 0,0]; // User RAM 1 register
    // write_buf[1..3].copy_from_slice(data);
    write_buf[1..=2].copy_from_slice(data);

    self.i2c.write(RV3028_ADDRESS, &write_buf)?;
    Ok(())
  }

  /// Get the value of the User Ram registers (two bytes)
  pub fn get_user_ram(&mut self) -> Result<[u8; 2], E> {
    self.select_mux_channel()?;
    let mut read_buf = [0u8; 2];
    self.read_multi_registers_raw(0x1F, &mut read_buf)?; // User RAM 1 register
    Ok(read_buf)
  }

  // Set the bcd time tracking registers.
  // Assumes `select_mux_channel` has already been called
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
    self.set_or_clear_reg_bits(REG_EVENT_CONTROL, RegEventControlBits::EventHighLowBit as u8, high)
  }

  /// Enable INT pin output when alarm occurs
  pub fn toggle_alarm_int_enable(&mut self, enable: bool) -> Result<(), E> {
    self.set_or_clear_reg_bits(REG_CONTROL2, RegControl2Bits::AlarmIntEnableBit as u8, enable)
  }

  /// Toggle whether the RTC outputs a pulse (active low) on INT pin,
  /// when the countdown timer expires.
  pub fn toggle_countdown_int_enable(&mut self, enable: bool) -> Result<(), E> {
    self.set_or_clear_reg_bits(REG_CONTROL2, RegControl2Bits::TimerIntEnableBit as u8, enable)
  }

  /// Toggle whether interrupt signal is generated on the INT pin:
  /// - when an External Event on EVI pin occurs and TSS = 0
  /// - or when an Automatic Backup Switchover occurs and TSS = 1.
  /// The signal on the INT pin is retained until the EVF flag is cleared
  /// to 0 (no automatic cancellation)
  pub fn toggle_ext_event_int_enable(&mut self, enable: bool) -> Result<(), E> {
    self.set_or_clear_reg_bits(REG_CONTROL2, RegControl2Bits::EventIntEnableBit as u8, enable)
  }

  /// Toggles whether an interrupt signal is generated on the INT pin:
  /// - when the time updates at either 1 second or 1 minute intervals
  pub fn toggle_time_up_int_enable(&mut self, enable: bool) -> Result<(), E> {
    self.set_or_clear_reg_bits(REG_CONTROL2, RegControl2Bits::TimeUpdateIntEnableBit as u8, enable)
  }

  /// Disable all INT pin output selector bits in RAM, excludes PORIE
  pub fn clear_all_int_out_bits(&mut self) -> Result<(), E> {
    self.select_mux_channel()?;
    // UIE, TIE, AIE,  EIE
    self.clear_reg_bits_raw(REG_CONTROL2,
                        RegControl2Bits::TimeUpdateIntEnableBit as u8 |
                          RegControl2Bits::TimerIntEnableBit as u8 |
                          RegControl2Bits::AlarmIntEnableBit as u8 |
                          RegControl2Bits::EventIntEnableBit as u8  )?;
    // BSIE
    self.clear_reg_bits_raw(
      REG_EEPROM_BACKUP_CONFIG, RegEepromBackupBits::BackupSwitchIntEnableBit as u8)?;

    // PORIE -- must be set in EEPROM -- don't bother to set?
    Ok(())
  }


  /// Clear all of the status registers that indicate whether
  /// various conditions have triggered
  pub fn clear_all_status_flags(&mut self) -> Result<(), E> {
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
  pub fn configure_periodic_time_update(&mut self, minutes: bool, int_enable: bool) -> Result<(), E> {
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


  /// Check the alarm status, and if it's triggered, clear it
  /// return bool indicating whether the alarm triggered
  pub fn check_and_clear_alarm(&mut self) -> Result<bool, E> {
    // Check if the AF flag is set
    let alarm_flag_set = 0 != self.check_and_clear_bits(
      REG_STATUS, RegStatusBits::AlarmFlagBit as u8)?;
    Ok(alarm_flag_set)
  }

  /// Check whether the alarm flag (AF) is set
  pub fn check_alarm_flag(&mut self) -> Result<bool, E> {
    // Check if the AF flag is set
    self.check_bits_nonzero(REG_STATUS, RegStatusBits::AlarmFlagBit as u8)
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
      REG_CONTROL1, RegControl1Bits::WadaBit as u8, !weekday.is_some())?;

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
    self.clear_reg_bits_raw(REG_STATUS, RegStatusBits::AlarmFlagBit as u8)?;

    Ok(())
  }

  /// Read the alarm settings
  /// Matches are flag settings for whether the alarm should match day, hour, minute
  ///
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

    let wada_state = self.read_register_raw(REG_CONTROL1)? & RegControl1Bits::WadaBit as u8;

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


  // If `set` is true, set the high bits given in `bits`, otherwise clear those bits
  fn set_or_clear_reg_bits(&mut self, reg: u8, bits: u8, set: bool) -> Result<(), E> {
    self.select_mux_channel()?;
    self.set_or_clear_reg_bits_raw(reg, bits, set)
  }

  // If `set` is true, set the high bits given in `bits`, otherwise clear those bits;
  // skips mux
  fn set_or_clear_reg_bits_raw(&mut self, reg: u8, bits: u8, set: bool) -> Result<(), E> {
    if set {
      self.set_reg_bits_raw(reg, bits)
    }
    else {
      self.clear_reg_bits_raw(reg, bits)
    }
  }


  /// Set the CLKOUT pin frequency, when clockout is enabled
  pub fn set_clockout_rate(&mut self, rate: ClockoutRate)-> Result<(), E> {
    self.select_mux_channel()?;
    self.clear_reg_bits_raw(REG_EEPROM_CLKOUT, ClockoutRate::ClkoutFreqSelectionBits as u8)?;
    self.set_reg_bits_raw(REG_EEPROM_CLKOUT, rate as u8)
  }


  /// Configures the behavior of the CLKOUT pin
  /// - `int_enable` enables or disables interrupt-controlled clockout.
  /// If false, use plain or "normal" clockout.
  /// - `rate` sets the clockout frequency
  pub fn config_clockout(&mut self, int_enable: bool, rate: ClockoutRate) -> Result<(), E> {
    self.select_mux_channel()?;

    // configure the clockout rate
    self.clear_reg_bits_raw(REG_EEPROM_CLKOUT, ClockoutRate::ClkoutFreqSelectionBits as u8)?;
    self.set_reg_bits_raw(REG_EEPROM_CLKOUT, rate as u8)?;

    // if we're enabling interrupt-controlled clockout, need to disable CLKOE
    self.set_or_clear_reg_bits_raw(
      REG_EEPROM_BACKUP_CONFIG, RegEepromBackupBits::ClockoutOutputEnableBit as u8,
      !int_enable)?;

    self.set_or_clear_reg_bits_raw(
      REG_CONTROL2, RegControl2Bits::ClockoutIntEnableBit as u8, int_enable)
  }

  // Configure the Periodic Countdown Timer prior to the next countdown.
  fn config_pct_raw(&mut self, value: u16, freq: TimerClockFreq, repeat: bool ) -> Result<(), E> {
    let value_high: u8 = ((value >> 8) as u8) & 0x0F;
    let value_low: u8 = (value & 0xFF) as u8;

    // configure the timer clock source / period
    self.clear_reg_bits_raw(REG_CONTROL1, RegControl1Bits::TimerEnableBit as u8)?;
    self.set_or_clear_reg_bits_raw(REG_CONTROL1, RegControl1Bits::TimerRepeatBit as u8, repeat)?;

    self.clear_reg_bits_raw(REG_CONTROL1, RegControl1Bits::TimerClockFreqBits as u8)?;
    self.set_reg_bits_raw(REG_CONTROL1, freq as u8)?; //TODO verify

    // write to REG_TIMER_VALUE0 and REG_TIMER_VALUE1
    let write_buf = [ REG_TIMER_VALUE0, value_low, value_high];
    self.i2c.write(RV3028_ADDRESS, &write_buf)?;

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
  /// and optionally start the countdown.
  ///
  /// - `repeat`: If true, the countdown timer will repeat as a periodic timer.
  /// If false, the countdown timer will only run once ("one-shot" mode).
  /// Returns the estimated actual duration (which may vary from the requested duration
  /// dur to discrete RTC clock ticks).
  /// - `start`: If true, start the countdown
  pub fn config_countdown_timer(&mut self, duration: &Duration,
                                repeat: bool, start: bool
  ) -> Result<Duration, E> {
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
  pub fn toggle_countdown_timer(&mut self, enable: bool)  -> Result<(), E> {
    self.set_or_clear_reg_bits(
      REG_CONTROL1, RegControl1Bits::TimerEnableBit as u8, enable)
  }

  /// Check whether countdown timer has finished counting down, and clear it
  pub fn check_and_clear_countdown(&mut self) -> Result<bool, E> {
    let flag_set = 0 != self.check_and_clear_bits(
      REG_STATUS, RegStatusBits::PeriodicTimerFlag as u8)?;
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

  fn check_bits_nonzero(&mut self, reg: u8, bits: u8) -> Result<bool, E> {
    self.select_mux_channel()?;
    self.check_bits_nonzero_raw(reg, bits)
  }

  fn check_bits_nonzero_raw(&mut self, reg: u8, bits: u8) -> Result<bool, E> {
    let reg_val = self.read_register_raw(reg)?;
    let bits_val =  reg_val & bits;
    Ok(0 != bits_val)
  }



  /// Configure event detection on the EVI pin
  /// - `rising` whether edge detection is on rising edge / high level
  /// - `int_enable` whether events detected on EVI pin should generate an interrupt on INT pin
  /// - `filtering` 00..11 time filtering
  pub fn config_ext_event_detection(
    &mut self, rising: bool, int_enable: bool, filtering: u8, clockout_enable: bool) -> Result<(), E>
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


pub trait EventTimeStampLogger {
  /// Error type
  type Error;

  /// Enable or disable the Time Stamp Function for event logging
  /// This logs external interrupts or other events
  fn toggle_timestamp_logging(&mut self, enable: bool) -> Result<(), Self::Error>;

  /// clear out any existing logged event timestamps
  fn reset_timestamp_log(&mut self) -> Result<(), Self::Error>;

  /// Setup time stamp logging for events
  /// - `evt_source` source for timestamp events, eg TS_EVENT_SOURCE_BSF
  /// - `overwrite` Save the most recent event timestamp?
  /// - `start` Should event timestamp logging immediately start?
  fn config_timestamp_logging(
    &mut self, evt_source: u8, overwrite: bool,   start: bool)
    -> Result<(), Self::Error>;

  /// Get event count -- the number of events that have been logged since enabling logging
  /// Returns the count of events since last reset, and the datetime of one event
  fn get_event_count_and_datetime(&mut self) -> Result<(u32, Option<NaiveDateTime>), Self::Error>;

  /// Enable or disable event time stamp overwriting
  /// If this is disabled (default), the first event time stamp is saved.
  /// If this is enabled, the most recent event time stamp is saved.
  fn toggle_time_stamp_overwrite(&mut self, enable: bool) -> Result<(), Self::Error>;

  /// Select a source for events to be logged, device-specific
  fn set_event_timestamp_source(&mut self, source: u8) -> Result<(), Self::Error>;
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
    &mut self, evt_source: u8, overwrite: bool,  start:bool) -> Result<(), E>
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

    Ok((count as u32, odt))
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

