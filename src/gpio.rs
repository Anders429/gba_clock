//! Communications with the RTC over General Purpose I/O.

use crate::{
    bcd::Bcd,
    date_time::{
        RtcDateTimeOffset,
        RtcTimeOffset,
    },
    Error,
};
use core::ops::{
    BitAnd,
    BitOr,
};

/// I/O Port Data.
///
/// Used for sending data directly to the RTC chip.
const DATA: *mut Data = 0x080000c4 as *mut Data;

/// I/O Port Direction.
///
/// This specifies which bits are writable and which bits are readable.
const RW_MODE: *mut RwMode = 0x080000c6 as *mut RwMode;

/// I/O Port Control.
///
/// By setting this to `1`, the General Purpose I/O (GPIO) will be both readable and writable.
const ENABLE: *mut u16 = 0x080000c8 as *mut u16;

/// Interrupt Master Enable.
///
/// This register allows enabling and disabling interrupts.
const IME: *mut bool = 0x0400_0208 as *mut bool;

/// A command used to interact with the RTC.
///
/// These commands are defined in the S-3511A specification.
enum Command {
    Reset = 0x60,
    WriteStatus = 0x62,
    ReadStatus = 0x63,
    ReadDateTime = 0x65,
    ReadTime = 0x67,
}

/// Configurations for I/O port direction.
///
/// There are three relevant bits for RTC:
/// - 0: SCK (Serial Clock Input)
/// - 1: SIO (Serial Data Input/Output)
/// - 2: CS (Chip Select)
///
/// Both SCK and CS should always be set high. Therefore, the only relevant bit is SIO, which can
/// either be set low to receive data or set high to send data, a single bit at a time.
#[repr(u16)]
enum RwMode {
    /// Sets SIO low, allowing data to be received from the RTC.
    Read = 5,
    /// Sets SIO high, allowing data to be sent to the RTC.
    Write = 7,
}

/// Data written to or received from the RTC.
///
/// While this is a 16-bit value, only the lowest 3 bits are used. This is because the RTC only
/// uses 3 of the 4 possible bits for interacting with the GPIO.
#[derive(Debug, PartialEq, Eq)]
struct Data(u16);

impl Data {
    /// Serial Clock Input.
    const SCK: Data = Data(0b0000_0000_0000_0001);
    /// Serial Data Input/Output.
    const SIO: Data = Data(0b0000_0000_0000_0010);
    /// Chip Select.
    const CS: Data = Data(0b0000_0000_0000_0100);
}

impl BitOr for Data {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl BitOr<u8> for Data {
    type Output = Self;

    fn bitor(self, other: u8) -> Self::Output {
        Self(self.0 | other as u16)
    }
}

impl BitOr<Data> for u8 {
    type Output = Data;

    fn bitor(self, other: Data) -> Self::Output {
        Data(self as u16 | other.0)
    }
}

impl BitAnd for Data {
    type Output = Self;

    fn bitand(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }
}

impl From<Data> for u8 {
    fn from(data: Data) -> Self {
        data.0 as u8
    }
}

/// Send a command to the RTC.
///
/// This must be called before every interaction with the RTC. See the `Command` variants for more
/// information.
fn send_command(command: Command) {
    let bits = (command as u8) << 1;
    // Bits must be sent from highest to lowest.
    for i in (0..8).rev() {
        let bit = (bits >> i) & 2;
        unsafe {
            DATA.write_volatile(Data::CS | bit);
            DATA.write_volatile(Data::CS | bit);
            DATA.write_volatile(Data::CS | bit);
            DATA.write_volatile(Data::CS | Data::SCK | bit);
        }
    }
}

/// Read a single byte.
fn read_byte() -> u8 {
    let mut byte: u8 = 0;
    for _ in 0..8 {
        unsafe {
            DATA.write_volatile(Data::CS);
            DATA.write_volatile(Data::CS);
            DATA.write_volatile(Data::CS);
            DATA.write_volatile(Data::CS);
            DATA.write_volatile(Data::CS);
            DATA.write_volatile(Data::CS | Data::SCK);
            byte = (byte >> 1) | (((u8::from(DATA.read_volatile() & Data::SIO)) >> 1) << 7);
        }
    }
    byte
}

// Write a single byte.
fn write_byte(byte: u8) {
    for i in 0..8 {
        unsafe {
            let bit = (byte >> i << 1) & 2;
            DATA.write_volatile(bit | Data::CS);
            DATA.write_volatile(bit | Data::CS);
            DATA.write_volatile(bit | Data::CS);
            DATA.write_volatile(bit | Data::CS | Data::SCK);
        }
    }
}

/// The RTC's status register.
///
/// This is an 8-bit representation of the various modes and states stored in the RTC itself. All
/// bits except `POWER` are writable. Bits 0, 2, and 4 are unused and therefore should never be
/// set.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Status(u8);

impl Status {
    pub(crate) const POWER: Status = Status(0b1000_0000);
    pub(crate) const HOUR_24: Status = Status(0b0100_0000);

    pub(crate) fn contains(&self, other: &Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl TryFrom<u8> for Status {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        // Check for unused bits.
        if value & 0b0001_0101 != 0 {
            Err(Error::InvalidStatus(value))
        } else {
            Ok(Status(value))
        }
    }
}

/// Attempt to obtain the `Status` register from the RTC.
pub(crate) fn try_read_status() -> Result<Status, Error> {
    // Disable interrupts, storing the previous value.
    //
    // This prevents interrupts while reading data from the device. This is necessary because GPIO
    // reads data one bit at a time.
    let previous_ime = unsafe { IME.read_volatile() };
    unsafe { IME.write_volatile(false) };

    // Request status.
    unsafe {
        DATA.write_volatile(Data::SCK);
        DATA.write_volatile(Data::CS | Data::SCK);
        RW_MODE.write_volatile(RwMode::Write);
    }
    send_command(Command::ReadStatus);

    // Receive status.
    unsafe {
        RW_MODE.write_volatile(RwMode::Read);
    }
    let status = read_byte();
    unsafe {
        DATA.write_volatile(Data::SCK);
        DATA.write_volatile(Data::SCK);
    }

    // Restore the previous interrupt enable value.
    unsafe {
        IME.write_volatile(previous_ime);
    }

    status.try_into()
}

/// Enable operations with the RTC via General Purpose I/O (GPIO).
pub(crate) fn enable() {
    unsafe {
        ENABLE.write_volatile(1);
    }
}

pub(crate) fn reset() {
    // Disable interrupts, storing the previous value.
    //
    // This prevents interrupts while reading data from the device. This is necessary because GPIO
    // reads data one bit at a time.
    let previous_ime = unsafe { IME.read_volatile() };
    unsafe { IME.write_volatile(false) };

    // Request reset.
    unsafe {
        DATA.write_volatile(Data::SCK);
        DATA.write_volatile(Data::CS | Data::SCK);
        RW_MODE.write_volatile(RwMode::Write);
    }
    send_command(Command::Reset);
    unsafe {
        DATA.write_volatile(Data::SCK);
        DATA.write_volatile(Data::SCK);
    }

    // Restore the previous interrupt enable value.
    unsafe {
        IME.write_volatile(previous_ime);
    }
}

/// Attempt to read the current RTC date and time value as an `RtcOffset`.
pub(crate) fn try_read_datetime_offset() -> Result<RtcDateTimeOffset, Error> {
    // Disable interrupts, storing the previous value.
    //
    // This prevents interrupts while reading data from the device. This is necessary because GPIO
    // reads data one bit at a time.
    let previous_ime = unsafe { IME.read_volatile() };
    unsafe { IME.write_volatile(false) };

    // Request datetime.
    unsafe {
        DATA.write_volatile(Data::SCK);
        DATA.write_volatile(Data::CS | Data::SCK);
        RW_MODE.write_volatile(RwMode::Write);
    }
    send_command(Command::ReadDateTime);

    // Receive datetime.
    unsafe {
        RW_MODE.write_volatile(RwMode::Read);
    }
    let year = read_byte();
    let month = read_byte();
    let day = read_byte();
    let _weekday = read_byte();
    let hour = read_byte();
    let minute = read_byte();
    let second = read_byte();
    unsafe {
        DATA.write_volatile(Data::SCK);
        DATA.write_volatile(Data::SCK);
    }

    // Restore the previous interrupt enable value.
    unsafe {
        IME.write_volatile(previous_ime);
    }

    Ok(RtcDateTimeOffset::new(
        Bcd::try_from(year)?.into(),
        Bcd::try_from(month)?.try_into()?,
        Bcd::try_from(day)?.try_into()?,
        Bcd::try_from(hour)?.try_into()?,
        Bcd::try_from(minute)?.try_into()?,
        Bcd::try_from(second)?.try_into()?,
    ))
}

pub(crate) fn try_read_time_offset() -> Result<RtcTimeOffset, Error> {
    // Disable interrupts, storing the previous value.
    //
    // This prevents interrupts while reading data from the device. This is necessary because GPIO
    // reads data one bit at a time.
    let previous_ime = unsafe { IME.read_volatile() };
    unsafe { IME.write_volatile(false) };

    // Request datetime.
    unsafe {
        DATA.write_volatile(Data::SCK);
        DATA.write_volatile(Data::CS | Data::SCK);
        RW_MODE.write_volatile(RwMode::Write);
    }
    send_command(Command::ReadTime);

    // Receive time.
    unsafe {
        RW_MODE.write_volatile(RwMode::Read);
    }
    let hour = read_byte();
    let minute = read_byte();
    let second = read_byte();
    unsafe {
        DATA.write_volatile(Data::SCK);
        DATA.write_volatile(Data::SCK);
    }

    // Restore the previous interrupt enable value.
    unsafe {
        IME.write_volatile(previous_ime);
    }

    Ok(RtcTimeOffset::new(
        Bcd::try_from(hour)?.try_into()?,
        Bcd::try_from(minute)?.try_into()?,
        Bcd::try_from(second)?.try_into()?,
    ))
}

pub(crate) fn is_test_mode() -> bool {
    // Disable interrupts, storing the previous value.
    //
    // This prevents interrupts while reading data from the device. This is necessary because GPIO
    // reads data one bit at a time.
    let previous_ime = unsafe { IME.read_volatile() };
    unsafe { IME.write_volatile(false) };

    // Request time.
    unsafe {
        DATA.write_volatile(Data::SCK);
        DATA.write_volatile(Data::CS | Data::SCK);
        RW_MODE.write_volatile(RwMode::Write);
    }
    send_command(Command::ReadTime);

    // Receive time.
    unsafe {
        RW_MODE.write_volatile(RwMode::Read);
    }
    let _hour = read_byte();
    let _minute = read_byte();
    let second = read_byte();
    unsafe {
        DATA.write_volatile(Data::SCK);
        DATA.write_volatile(Data::SCK);
    }

    // Restore the previous interrupt enable value.
    unsafe {
        IME.write_volatile(previous_ime);
    }

    // Check whether the test flag is set.
    second & 0b1000_0000 != 0
}

pub(crate) fn set_status(status: Status) {
    // Disable interrupts, storing the previous value.
    //
    // This prevents interrupts while reading data from the device. This is necessary because GPIO
    // reads data one bit at a time.
    let previous_ime = unsafe { IME.read_volatile() };
    unsafe { IME.write_volatile(false) };

    // Request status write.
    unsafe {
        DATA.write_volatile(Data::SCK);
        DATA.write_volatile(Data::CS | Data::SCK);
        RW_MODE.write_volatile(RwMode::Write);
    }
    send_command(Command::WriteStatus);

    // Write the status.
    write_byte(status.0);
    unsafe {
        DATA.write_volatile(Data::SCK);
        DATA.write_volatile(Data::SCK);
    }

    // Restore the previous interrupt enable value.
    unsafe {
        IME.write_volatile(previous_ime);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Data,
        Status,
    };
    use crate::Error;
    use claims::{
        assert_err_eq,
        assert_ok_eq,
    };
    use gba_test::test;

    #[test]
    fn data_bit_or_empty() {
        assert_eq!(Data(0) | Data(0), Data(0));
    }

    #[test]
    fn data_bit_or_sck_sio() {
        assert_eq!(Data::SCK | Data::SIO, Data(0b0000_0000_0000_0011));
    }

    #[test]
    fn data_bit_or_sck_cs() {
        assert_eq!(Data::SCK | Data::CS, Data(0b0000_0000_0000_0101));
    }

    #[test]
    fn data_bit_or_sio_cs() {
        assert_eq!(Data::SIO | Data::CS, Data(0b0000_0000_0000_0110));
    }

    #[test]
    fn data_bit_or_all() {
        assert_eq!(
            Data::SCK | Data::SIO | Data::CS,
            Data(0b0000_0000_0000_0111)
        );
    }

    #[test]
    fn data_bit_or_u8_empty() {
        assert_eq!(Data(0) | 0, Data(0));
    }

    #[test]
    fn data_bit_or_u8_sck_sio() {
        assert_eq!(Data::SCK | 2, Data(0b0000_0000_0000_0011));
    }

    #[test]
    fn data_bit_or_u8_sck_cs() {
        assert_eq!(Data::SCK | 4, Data(0b0000_0000_0000_0101));
    }

    #[test]
    fn data_bit_or_u8_sio_cs() {
        assert_eq!(Data::SIO | 4, Data(0b0000_0000_0000_0110));
    }

    #[test]
    fn data_bit_or_u8_all() {
        assert_eq!(Data::SCK | 6, Data(0b0000_0000_0000_0111));
    }

    #[test]
    fn u8_bit_or_data_empty() {
        assert_eq!(0 | Data(0), Data(0));
    }

    #[test]
    fn u8_bit_or_data_sck_sio() {
        assert_eq!(1 | Data::SIO, Data(0b0000_0000_0000_0011));
    }

    #[test]
    fn u8_bit_or_data_sck_cs() {
        assert_eq!(1 | Data::CS, Data(0b0000_0000_0000_0101));
    }

    #[test]
    fn u8_bit_or_data_sio_cs() {
        assert_eq!(2 | Data::CS, Data(0b0000_0000_0000_0110));
    }

    #[test]
    fn u8_bit_or_data_all() {
        assert_eq!(3 | Data::CS, Data(0b0000_0000_0000_0111));
    }

    #[test]
    fn data_bit_and_empty() {
        assert_eq!(Data(0) & Data(0), Data(0));
    }

    #[test]
    fn data_bit_and_empty_sck() {
        assert_eq!(Data(0) & Data::SCK, Data(0));
    }

    #[test]
    fn data_bit_and_empty_sio() {
        assert_eq!(Data(0) & Data::SIO, Data(0));
    }

    #[test]
    fn data_bit_and_empty_cs() {
        assert_eq!(Data(0) & Data::CS, Data(0));
    }

    #[test]
    fn data_bit_and_all_sck() {
        assert_eq!(Data(7) & Data::SCK, Data::SCK);
    }

    #[test]
    fn data_bit_and_all_sio() {
        assert_eq!(Data(7) & Data::SIO, Data::SIO);
    }

    #[test]
    fn data_bit_and_all_cs() {
        assert_eq!(Data(7) & Data::CS, Data::CS);
    }

    #[test]
    fn data_bit_and_all() {
        assert_eq!(Data(7) & Data(7), Data(7));
    }

    #[test]
    fn status_contains_power() {
        assert!(Status::POWER.contains(&Status::POWER));
    }

    #[test]
    fn status_contains_no_power() {
        assert!(!Status(0).contains(&Status::POWER));
    }

    #[test]
    fn status_contains_hour_24() {
        assert!(Status::HOUR_24.contains(&Status::HOUR_24));
    }

    #[test]
    fn status_contains_no_hour_24() {
        assert!(!Status(0).contains(&Status::HOUR_24));
    }

    #[test]
    fn status_from_empty() {
        assert_ok_eq!(Status::try_from(0), Status(0));
    }

    #[test]
    fn status_contains_invalid_bit_0() {
        assert_err_eq!(
            Status::try_from(0b0000_0001),
            Error::InvalidStatus(0b0000_0001)
        );
    }

    #[test]
    fn status_contains_invalid_bit_2() {
        assert_err_eq!(
            Status::try_from(0b0000_0100),
            Error::InvalidStatus(0b0000_0100)
        );
    }

    #[test]
    fn status_contains_invalid_bit_4() {
        assert_err_eq!(
            Status::try_from(0b0001_0000),
            Error::InvalidStatus(0b0001_0000)
        );
    }

    #[test]
    fn status_from_all_bits_set_is_invalid() {
        assert_err_eq!(Status::try_from(0xff), Error::InvalidStatus(0xff));
    }

    #[test]
    fn status_from_power() {
        assert_ok_eq!(Status::try_from(0b1000_0000), Status::POWER);
    }

    #[test]
    fn status_from_hour_24() {
        assert_ok_eq!(Status::try_from(0b0100_0000), Status::HOUR_24);
    }

    #[test]
    fn status_from_all_valid_bits() {
        assert_ok_eq!(Status::try_from(0b1110_1010), Status(0b1110_1010));
    }
}
