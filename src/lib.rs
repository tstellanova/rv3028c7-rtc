#![cfg_attr(not(test), no_std)]

use embedded_hal::blocking::i2c::{Write, Read, WriteRead};

const RV3028_ADDRESS: u8 = 0xA4 >> 1; // 7-bit address

// Register addresses
const ADDR_SECONDS: u8 = 0x00;
const ADDR_MINUTES: u8 = 0x01;
const ADDR_HOURS: u8 = 0x02;

const ADDR_WEEKDAY: u8 = 0x03;
const ADDR_DATE: u8 = 0x04;
const ADDR_MONTH: u8 = 0x05;
const ADDR_YEAR: u8 = 0x06;


const STATUS_REG: u8 = 0x0E; // Status register address
const CONTROL_1_REG: u8 = 0x1D; // Control 1 register address
const EEBUSY_BIT: u8 = 7; // EEbusy bit in the Status register


// EEPROM register addresses and commands
pub const EEPROM_ADDRESS: u8 = 0x37;
pub const EEPROM_CMD_READ: u8 = 0x00;
pub const EEPROM_CMD_WRITE: u8 = 0x01;


pub struct RV3028<I2C> {
    i2c: I2C,
}

impl<I2C, E> RV3028<I2C>
where
    I2C: Write<Error = E> + Read<Error = E> + WriteRead<Error = E>,
{
    pub fn new(i2c: I2C) -> Self {
        RV3028 { i2c }
    }

    /// Converts a binary value to BCD format
    fn bin_to_bcd(value: u8) -> u8 {
        ((value / 10) << 4) | (value % 10)
    }

    /// Converts a BCD value to binary format
    fn bcd_to_bin(value: u8) -> u8 {
        ((value & 0xF0) >> 4) * 10 + (value & 0x0F)
    }

    fn write_register(&mut self, reg: u8, data: u8) -> Result<(), E> {
        self.i2c.write(RV3028_ADDRESS, &[reg, data])
    }

    fn read_register(&mut self, reg: u8) -> Result<u8, E> {
        let mut buf = [0];
        self.i2c.write_read(RV3028_ADDRESS, &[reg], &mut buf)?;
        Ok(buf[0])
    }

    fn is_eeprom_busy(&mut self) -> Result<bool, E> {
        let status = self.read_register(STATUS_REG)?;
        Ok(status & (1 << EEBUSY_BIT) != 0)
    }

    fn disable_auto_eeprom_refresh(&mut self) -> Result<(), E> {
        let mut control_1 = self.read_register(CONTROL_1_REG)?;
        control_1 |= 1 << 3; // Set EERD bit
        self.write_register(CONTROL_1_REG, control_1)
    }

    fn enable_auto_eeprom_refresh(&mut self) -> Result<(), E> {
        let mut control_1 = self.read_register(CONTROL_1_REG)?;
        control_1 &= !(1 << 3); // Clear EERD bit
        self.write_register(CONTROL_1_REG, control_1)
    }

    pub fn eeprom_read(&mut self, address: u8) -> Result<u8, E> {
        self.disable_auto_eeprom_refresh()?;
        while self.is_eeprom_busy()? {}
        // Read from EEPROM
        self.write_register(EEPROM_ADDRESS, address)?;
        let res = self.read_register(EEPROM_ADDRESS);
        self.enable_auto_eeprom_refresh()?;
        res
    }

    pub fn eeprom_write(&mut self, address: u8, data: u8) -> Result<(), E> {
        self.disable_auto_eeprom_refresh()?;
        while self.is_eeprom_busy()? {}
        // Write to EEPROM
        // ...
        self.write_register(EEPROM_ADDRESS, address)?;
        let res = self.write_register(EEPROM_ADDRESS, data);

        self.enable_auto_eeprom_refresh()?;
        res
    }

    // Set time (hours, minutes, seconds) in binary format
    pub fn set_time(&mut self, hours: u8, minutes: u8, seconds: u8) -> Result<(), E> {
        self.write_register(ADDR_HOURS, Self::bin_to_bcd(hours))?;
        self.write_register(ADDR_MINUTES, Self::bin_to_bcd(minutes))?;
        self.write_register(ADDR_SECONDS, Self::bin_to_bcd(seconds))
    }

    // Get time in binary format
    pub fn get_time(&mut self) -> Result<(u8, u8, u8), E> {
        let hours = Self::bcd_to_bin(self.read_register(ADDR_HOURS)?);
        let minutes = Self::bcd_to_bin(self.read_register(ADDR_MINUTES)?);
        let seconds = Self::bcd_to_bin(self.read_register(ADDR_SECONDS)?);
        Ok((hours, minutes, seconds))
    }

        // Set the weekday
    pub fn set_weekday(&mut self, weekday: u8) -> Result<(), E> {
        self.write_register(ADDR_WEEKDAY, Self::bin_to_bcd(weekday))
    }

    // Get the weekday
    pub fn get_weekday(&mut self) -> Result<u8, E> {
        let bcd = self.read_register(ADDR_WEEKDAY)?;
        Ok(Self::bcd_to_bin(bcd))
    }

    // Year is 0..99  (2000 to 2099)
    pub fn set_year_month_day(&mut self, year: u8, month: u8, day: u8) -> Result<(), E> {
      self.write_register(ADDR_YEAR, Self::bin_to_bcd(year))?; 
      self.write_register(ADDR_MONTH, Self::bin_to_bcd(month))?; 
      self.write_register(ADDR_DATE, Self::bin_to_bcd(day))
    }
 

    // Set the date
    pub fn set_date(&mut self, date: u8) -> Result<(), E> {
        self.write_register(ADDR_DATE, Self::bin_to_bcd(date))
    }

    // Get the date
    pub fn get_date(&mut self) -> Result<u8, E> {
        let bcd = self.read_register(ADDR_DATE)?;
        Ok(Self::bcd_to_bin(bcd))
    }

    // Set the month
    pub fn set_month(&mut self, month: u8) -> Result<(), E> {
        self.write_register(ADDR_MONTH, Self::bin_to_bcd(month))
    }

    // Get the month
    pub fn get_month(&mut self) -> Result<u8, E> {
        let bcd = self.read_register(ADDR_MONTH)?;
        Ok(Self::bcd_to_bin(bcd))
    }

    // Set the year
    pub fn set_year(&mut self, year: u8) -> Result<(), E> {
        self.write_register(ADDR_YEAR, Self::bin_to_bcd(year))
    }

    // Get the year
    pub fn get_year(&mut self) -> Result<u8, E> {
        let bcd = self.read_register(ADDR_YEAR)?;
        Ok(Self::bcd_to_bin(bcd))
    }

    pub fn get_year_month_day(&mut self) -> Result<(u8, u8, u8), E> {
	let year = self.get_year()?;
        let month = self.get_month()?;
        let day = self.get_date()?;
        Ok((year,month,day))
    }


    // Add more methods for other functionalities...
}

#[cfg(test)]
mod tests {
    use super::*;
    use embedded_hal_mock::i2c::{Mock as I2cMock, Transaction as I2cTrans};

     use std::vec;


    // Test setting time
    #[test]
    fn test_set_time() {
        let expectations = [
            I2cTrans::write(RV3028_ADDRESS, vec![ADDR_HOURS, RV3028::<I2cMock>::bin_to_bcd(23)]),
            I2cTrans::write(RV3028_ADDRESS, vec![ADDR_MINUTES, RV3028::<I2cMock>::bin_to_bcd(59)]),
            I2cTrans::write(RV3028_ADDRESS, vec![ADDR_SECONDS, RV3028::<I2cMock>::bin_to_bcd(58)]),
        ];
        let mock = I2cMock::new(&expectations);
        let mut rv3028 = RV3028::new(mock);
        rv3028.set_time(23, 59, 58).unwrap();
    }

    // Test getting time
    #[test]
    fn test_get_time() {
        let expectations = [
            I2cTrans::write_read(RV3028_ADDRESS, vec![ADDR_HOURS], vec![0x17]),   // 23 in BCD
            I2cTrans::write_read(RV3028_ADDRESS, vec![ADDR_MINUTES], vec![0x59]), // 59 in BCD
            I2cTrans::write_read(RV3028_ADDRESS, vec![ADDR_SECONDS], vec![0x58]), // 58 in BCD
        ];
        let mock = I2cMock::new(&expectations);
        let mut rv3028 = RV3028::new(mock);
        let (hours, minutes, seconds) = rv3028.get_time().unwrap();
        assert_eq!(hours, 23);
        assert_eq!(minutes, 59);
        assert_eq!(seconds, 58);
    }

    // Add more tests for other methods like eeprom_read, eeprom_write, etc.
    // ...

}


