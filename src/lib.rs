#![no_std]

mod bcd;
mod date_time;
mod gpio;

use date_time::RtcOffset;
use deranged::RangedU32;
use gpio::{enable, is_test_mode, reset, set_status, try_read_datetime, try_read_status, Status};
use time::{Date, PrimitiveDateTime};

/// Errors that may occur when interacting with the RTC.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
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
    /// The base date from which dates and times are calculated.
    ///
    /// Dates and times are read by calculating the amount of time that has elapsed from midnight
    /// on this date, using the RTC's value and the stored `rtc_offset`.
    base_date: Date,

    /// The amount of time elapsed on the RTC at which point the `base_date` was set.
    ///
    /// This is used to calculate the current date and time by calculating how much time has
    /// elapsed on the RTC past this offset and adding this value to the `base_date`.
    rtc_offset: RtcOffset,
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
        let rtc_offset = RtcOffset::new(year, month, day, hour, minute, second);

        Ok(Self {
            base_date: datetime.date(),
            rtc_offset: rtc_offset - datetime.time().into(),
        })
    }

    /// Reads the currently stored date and time.
    pub fn read_datetime(&self) -> Result<PrimitiveDateTime, Error> {
        let (year, month, day, hour, minute, second) = try_read_datetime()?;
        let rtc_offset = RtcOffset::new(year, month, day, hour, minute, second);

        let duration = if rtc_offset.0 >= self.rtc_offset.0 {
            RtcOffset(unsafe { rtc_offset.0.unchecked_sub(self.rtc_offset.0.get()) }).into()
        } else {
            RtcOffset(unsafe {
                RangedU32::MAX
                    .unchecked_sub(self.rtc_offset.0.get())
                    .unchecked_add(rtc_offset.0.get())
                    .unchecked_add(1)
            })
            .into()
        };

        self.base_date
            .midnight()
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
        let rtc_offset = RtcOffset::new(year, month, day, hour, minute, second);
        self.base_date = datetime.date();
        self.rtc_offset = rtc_offset - datetime.time().into();
        Ok(())
    }
}
