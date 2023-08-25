#![no_std]

mod bcd;
mod date_time;
mod gpio;

use date_time::calculate_rtc_offset;
use gpio::{enable, is_test_mode, reset, set_status, try_read_datetime, try_read_status, Status};
use time::{Duration, PrimitiveDateTime};

/// Errors that may occur when interacting with the RTC.
#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    PowerFailure,
    TestMode,
    AmPmBitPresent,
    InvalidStatus,
    InvalidMonth,
    InvalidDay,
    InvalidHour,
    InvalidMinute,
    InvalidSecond,
    InvalidBinaryCodedDecimal,
    Overflow,
}

/// Access to the Real Time Clock.
///
/// Instantiating a `Clock` initializes the relevant registers for interacting with the RTC,
/// allowing subsequent reads of the RTC's stored date and time. Dates and times are represented
/// using types from the [`time`] crate.
#[derive(Debug)]
pub struct Clock {
    /// The base date and time.
    ///
    /// The date and time are read by calculating the amount of time that has elapsed from this value.
    datetime_offset: PrimitiveDateTime,
    /// The RTC's time, in seconds, corresponding to the stored `datetime_offset`.
    ///
    /// When calculating the current date and time, the current RTC value is offset by this value,
    /// and the difference is added to the stored `datetime_offset`.
    rtc_offset: u32,
}

impl Clock {
    /// Creates a new `Clock` set at the given `datetime`.
    ///
    /// Note that this does not actually change the stored date and time in the RTC itself. While
    /// RTC values are writable on real hardware, they are often not writable in GBA emulators.
    /// Therefore, the date and time are stored as being offset from the current RTC date and time
    /// to maintain maximum compatibility.
    pub fn new(datetime: PrimitiveDateTime) -> Result<Self, Error> {
        // Enable operations with the RTC via General Purpose I/O (GPIO).
        enable();

        // Initialize the RTC itself.
        reset();
        // If the power bit is active, we need to reset.
        let status = try_read_status()?;
        if status.contains(&Status::POWER) {
            reset();
        }
        // If we are in test mode, we need to reset.
        if is_test_mode() {
            reset();
        }
        // Set to 24-hour time.
        set_status(Status::HOUR_24);

        let (year, month, day, hour, minute, second) = try_read_datetime()?;
        let rtc_offset = calculate_rtc_offset(year, month, day, hour, minute, second);

        Ok(Self {
            datetime_offset: datetime,
            rtc_offset,
        })
    }

    /// Reads the currently stored date and time.
    pub fn read_datetime(&self) -> Result<PrimitiveDateTime, Error> {
        let (year, month, day, hour, minute, second) = try_read_datetime()?;
        let rtc_offset = calculate_rtc_offset(year, month, day, hour, minute, second);
        let duration = if rtc_offset >= self.rtc_offset {
            Duration::seconds((rtc_offset - self.rtc_offset).into())
        } else {
            Duration::seconds((3_155_760_000 - self.rtc_offset + rtc_offset).into())
        };

        self.datetime_offset
            .checked_add(duration)
            .ok_or(Error::Overflow)
    }

    /// Writes a new date and time.
    ///
    /// Note that this does not actually change the stored date and time in the RTC itself. While
    /// RTC values are writable on real hardware, they are often not writable in GBA emulators.
    /// Therefore, the date and time are stored as being offset from the current RTC date and time
    /// to maintain maximum compatibility.
    pub fn write_datetime(&mut self, datetime: PrimitiveDateTime) -> Result<(), Error> {
        let (year, month, day, hour, minute, second) = try_read_datetime()?;
        let rtc_offset = calculate_rtc_offset(year, month, day, hour, minute, second);
        self.datetime_offset = datetime;
        self.rtc_offset = rtc_offset;
        Ok(())
    }
}
