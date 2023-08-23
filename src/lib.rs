#![no_std]

use core::ops::{BitAnd, BitOr};
use time::{Date, PrimitiveDateTime, Time};

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
/// By setting this to `true`, the General Purpose I/O (GPIO) will be both readable and writable.
const ENABLE: *mut bool = 0x080000c8 as *mut bool;

/// Interrupt Master Enable.
///
/// This register allows enabling and disabling interrupts.
const IME: *mut bool = 0x0400_0208 as *mut bool;

/// Errors that may occur when interacting with the RTC.
#[derive(Debug)]
pub enum Error {
    PowerFailure,
    InvalidStatus,
    InvalidYear,
    InvalidMonth,
    InvalidDay,
    InvalidWeekday,
    InvalidHour,
    InvalidMinute,
    InvalidSecond,
    InvalidBinaryCodedDecimal,
    TimeComponentRange(time::error::ComponentRange),
}

/// A command used to interact with the RTC.
///
/// These commands are defined in the S-3511A specification.
enum Command {
    ReadStatus = 0x63,
    WriteDateTime = 0x64,
    ReadDateTime = 0x65,
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
            let bit = (byte << i) & 1;
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
struct Status(u8);

impl Status {
    const POWER: Status = Status(0b1000_0000);
    const HOUR_24: Status = Status(0b0100_0000);

    fn contains(&self, other: &Self) -> bool {
        self.0 & other.0 != 0
    }
}

impl TryFrom<u8> for Status {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        // Check for unused bits.
        if value & 0b0001_0101 != 0 {
            Err(Error::InvalidStatus)
        } else {
            Ok(Status(value))
        }
    }
}

/// Attempt to obtain the `Status` register from the RTC.
fn try_read_status() -> Result<Status, Error> {
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

/// Converts from binary coded decimal to binary.
///
/// The S-3511A stores values as BCD, meaning each half-byte represents a digit. For example, the
/// value `12` is not represented as `0x0c`, but is instead represented as `0x12`.
fn bcd_to_binary(bcd: u8) -> Result<u8, Error> {
    if bcd < 0xa0 && (bcd & 0x0f < 0xa) {
        Ok(10 * (bcd >> 4 & 0x0f) + (bcd & 0x0f))
    } else {
        // Cannot interpret any half-byte greater than 0x9.
        Err(Error::InvalidBinaryCodedDecimal)
    }
}

/// Converts from binary to binary coded decimal.
///
/// For example, this converts `0x0c` (the value `12`) to `0x12`.
fn binary_to_bcd(binary: u8) -> u8 {
    ((binary / 10) << 4) + (binary % 10)
}

/// A calendar year.
#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
struct Year(u8);

impl TryFrom<u8> for Year {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let year = bcd_to_binary(value)?;
        if year > 99 {
            Err(Error::InvalidYear)
        } else {
            Ok(Self(year))
        }
    }
}

impl From<Year> for u8 {
    fn from(year: Year) -> Self {
        binary_to_bcd(year.0)
    }
}

/// A calendar month.
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
enum Month {
    January = 0x01,
    February = 0x02,
    March = 0x03,
    April = 0x04,
    May = 0x05,
    June = 0x06,
    July = 0x07,
    August = 0x08,
    September = 0x09,
    October = 0x10,
    November = 0x11,
    December = 0x12,
}

impl From<Month> for time::Month {
    fn from(month: Month) -> Self {
        match month {
            Month::January => time::Month::January,
            Month::February => time::Month::February,
            Month::March => time::Month::March,
            Month::April => time::Month::April,
            Month::May => time::Month::May,
            Month::June => time::Month::June,
            Month::July => time::Month::July,
            Month::August => time::Month::August,
            Month::September => time::Month::September,
            Month::October => time::Month::October,
            Month::November => time::Month::November,
            Month::December => time::Month::December,
        }
    }
}

impl From<time::Month> for Month {
    fn from(month: time::Month) -> Self {
        match month {
            time::Month::January => Month::January,
            time::Month::February => Month::February,
            time::Month::March => Month::March,
            time::Month::April => Month::April,
            time::Month::May => Month::May,
            time::Month::June => Month::June,
            time::Month::July => Month::July,
            time::Month::August => Month::August,
            time::Month::September => Month::September,
            time::Month::October => Month::October,
            time::Month::November => Month::November,
            time::Month::December => Month::December,
        }
    }
}

impl TryFrom<u8> for Month {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(Self::January),
            0x02 => Ok(Self::February),
            0x03 => Ok(Self::March),
            0x04 => Ok(Self::April),
            0x05 => Ok(Self::May),
            0x06 => Ok(Self::June),
            0x07 => Ok(Self::July),
            0x08 => Ok(Self::August),
            0x09 => Ok(Self::September),
            0x10 => Ok(Self::October),
            0x11 => Ok(Self::November),
            0x12 => Ok(Self::December),
            _ => Err(Error::InvalidMonth),
        }
    }
}

impl From<Month> for u8 {
    fn from(month: Month) -> Self {
        month as _
    }
}

/// A day within a month.
#[derive(Eq, PartialEq, PartialOrd, Ord)]
struct Day(u8);

impl TryFrom<u8> for Day {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let day = bcd_to_binary(value)?;
        if day == 0 || day > 31 {
            Err(Error::InvalidDay)
        } else {
            Ok(Self(day))
        }
    }
}

impl From<Day> for u8 {
    fn from(day: Day) -> Self {
        binary_to_bcd(day.0)
    }
}

/// A specific day within a week.
enum Weekday {
    Monday = 0x00,
    Tuesday = 0x01,
    Wednesday = 0x02,
    Thursday = 0x03,
    Friday = 0x04,
    Saturday = 0x05,
    Sunday = 0x06,
}

impl From<time::Weekday> for Weekday {
    fn from(weekday: time::Weekday) -> Self {
        match weekday {
            time::Weekday::Monday => Self::Monday,
            time::Weekday::Tuesday => Self::Tuesday,
            time::Weekday::Wednesday => Self::Wednesday,
            time::Weekday::Thursday => Self::Thursday,
            time::Weekday::Friday => Self::Friday,
            time::Weekday::Saturday => Self::Saturday,
            time::Weekday::Sunday => Self::Sunday,
        }
    }
}

impl TryFrom<u8> for Weekday {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Self::Monday),
            0x01 => Ok(Self::Tuesday),
            0x02 => Ok(Self::Wednesday),
            0x03 => Ok(Self::Thursday),
            0x04 => Ok(Self::Friday),
            0x05 => Ok(Self::Saturday),
            0x06 => Ok(Self::Sunday),
            _ => Err(Error::InvalidWeekday),
        }
    }
}

impl From<Weekday> for u8 {
    fn from(weekday: Weekday) -> Self {
        weekday as _
    }
}

/// An hour of the day.
struct Hour(u8);

impl TryFrom<u8> for Hour {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        // Ignore the am/pm bit.
        let hour = bcd_to_binary(value & 0b0111_1111)?;
        if hour > 23 {
            Err(Error::InvalidHour)
        } else {
            Ok(Self(hour))
        }
    }
}

impl From<Hour> for u8 {
    fn from(hour: Hour) -> Self {
        binary_to_bcd(hour.0)
    }
}

/// A minute within an hour.
struct Minute(u8);

impl TryFrom<u8> for Minute {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let minute = bcd_to_binary(value)?;
        if minute > 59 {
            Err(Error::InvalidMinute)
        } else {
            Ok(Self(minute))
        }
    }
}

impl From<Minute> for u8 {
    fn from(minute: Minute) -> Self {
        binary_to_bcd(minute.0)
    }
}

/// A second within a minute.
struct Second(u8);

impl TryFrom<u8> for Second {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let second = bcd_to_binary(value)?;
        if second > 59 {
            Err(Error::InvalidSecond)
        } else {
            Ok(Self(second))
        }
    }
}

impl From<Second> for u8 {
    fn from(second: Second) -> Self {
        binary_to_bcd(second.0)
    }
}

/// Attempt to read the date and time from the RTC.
fn try_read_datetime() -> Result<(Year, Month, Day, Weekday, Hour, Minute, Second), Error> {
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
    let weekday = read_byte();
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

    Ok((
        year.try_into()?,
        month.try_into()?,
        day.try_into()?,
        weekday.try_into()?,
        hour.try_into()?,
        minute.try_into()?,
        second.try_into()?,
    ))
}

/// Write date and time values to the RTC.
///
/// These should be validated before calling this function. However, behavior with invalid values
/// is defined, and will technically result in a valid state, so this function is not marked as
/// `unsafe` and does not attempt to validate the data.
fn write_datetime(
    year: Year,
    month: Month,
    day: Day,
    weekday: Weekday,
    hour: Hour,
    minute: Minute,
    second: Second,
) {
    // Disable interrupts, storing the previous value.
    //
    // This prevents interrupts while reading data from the device. This is necessary because GPIO
    // reads data one bit at a time.
    let previous_ime = unsafe { IME.read_volatile() };
    unsafe { IME.write_volatile(false) };

    // Request datetime write.
    unsafe {
        DATA.write_volatile(Data::SCK);
        DATA.write_volatile(Data::CS | Data::SCK);
        RW_MODE.write_volatile(RwMode::Write);
    }
    send_command(Command::WriteDateTime);

    // Send datetime.
    write_byte(year.into());
    write_byte(month.into());
    write_byte(day.into());
    write_byte(weekday.into());
    write_byte(hour.into());
    write_byte(minute.into());
    write_byte(second.into());
    unsafe {
        DATA.write_volatile(Data::SCK);
        DATA.write_volatile(Data::SCK);
    }

    // Restore the previous interrupt enable value.
    unsafe {
        IME.write_volatile(previous_ime);
    }
}

/// Access to the Real Time Clock.
///
/// Instantiating a `Clock` initializes the relevant registers for interacting with the RTC,
/// allowing subsequent reads and writes on the RTC's stored date and time. Dates and times are
/// represented using types from the [`time`] crate.
///
/// This struct stores some state to represent the current century. As such, reads using a `Clock`
/// are not limited to the years 2000-2099; instead, any year that can be represented by the `time`
/// crate can be stored here.
#[derive(Debug)]
pub struct Clock {
    /// The current century.
    ///
    /// The GBA RTC does not actually store the current century, instead storing the year as a
    /// value between 0 and 99. Storing the century allows years outside of the range 2000-2099 to
    /// be represented.
    century: i8,
    /// The previously-read year.
    ///
    /// This is used to detect if the RTC has wrapped back around to the year 0, which indicates
    /// that the century should be incremented.
    prev_year: Year,
    /// The previuosly-read month.
    ///
    /// This is used, together with `prev_year`, to detect incorrect leap days. Because the S-3511A
    /// was designed to only store years between 2000-2099, it will always count a leap day in the
    /// first year. Therefore, the date must be corrected when this leap day is encountered when
    /// the century is not divisble by 4 (for example, the year 1900 is not a leap year).
    prev_month: Month,
}

impl Clock {
    /// Creates a new `Clock`.
    ///
    /// Reading from a newly-instantiated `Clock` before writing will return whatever date and time
    /// is currently stored in the RTC, interpreting the year as being in the 21st century (i.e.
    /// being in the range 2000-2099).
    pub fn new() -> Result<Self, Error> {
        // Enable operations with the RTC via General Purpose I/O (GPIO).
        unsafe {
            ENABLE.write_volatile(true);
        }

        // Check status.
        let status = try_read_status()?;
        if status.contains(&Status::POWER) {
            return Err(Error::PowerFailure);
        }

        Ok(Self {
            century: 20,
            prev_year: Year(0),
            prev_month: Month::January,
        })
    }

    /// Read the currently stored date and time.
    pub fn read_datetime(&mut self) -> Result<PrimitiveDateTime, Error> {
        let (mut year, mut month, mut day, weekday, mut hour, mut minute, mut second) =
            try_read_datetime()?;

        // Check whether we have entered a new century.
        if self.prev_year > year {
            if self.century == 99 {
                self.century = -99;
            } else {
                self.century += 1;
            }
        }

        // Check whether we need to adjust for the clock's built-in turn-of-the-century leap year.
        if (self.prev_year > year
            || (self.prev_year == Year(0) && self.prev_month <= Month::February))
            && (year > Year(0)
                || month > Month::February
                || (month == Month::February && day > Day(28)))
            && self.century % 4 != 0
        {
            // Adjust for leap year.
            // Note that the S-3511A will do end-of-month correction. This means that if we pass in
            // a day such as February 30, it will be corrected to March 1.
            write_datetime(year, month, Day(day.0 + 1), weekday, hour, minute, second);
            // Read the time again. This is easier than trying to recalculate the correct month and
            // year when the day has been changed.
            (year, month, day, _, hour, minute, second) = try_read_datetime()?;
        }

        let date = Date::from_calendar_date(
            self.century as i32 * 100 + year.0 as i32,
            month.into(),
            day.0,
        )
        .map_err(Error::TimeComponentRange)?;

        let time = Time::from_hms(hour.0, minute.0, second.0).map_err(Error::TimeComponentRange)?;

        self.prev_year = year;
        self.prev_month = month;

        Ok(PrimitiveDateTime::new(date, time))
    }

    /// Write a new date and time to the RTC.
    pub fn write_datetime(&mut self, datetime: PrimitiveDateTime) {
        let date = datetime.date();
        let century = date.year() / 100;
        let year = Year(date.year().rem_euclid(100) as u8);
        let month = date.month().into();
        let day = Day(date.day());
        let weekday = date.weekday().into();

        let time = datetime.time();
        let hour = Hour(time.hour());
        let minute = Minute(time.minute());
        let second = Second(time.second());

        write_datetime(year, month, day, weekday, hour, minute, second);

        // TODO: Should ensure that this doesn't fail, if, for example, `time`'s `large-dates`
        // feature is enabled.
        self.century = century as i8;
        self.prev_year = year;
        self.prev_month = month;
    }
}
