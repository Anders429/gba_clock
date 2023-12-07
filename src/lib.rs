//! A real-time clock library for the GBA.
//!
//! Provides access to the RTC for programs running on a Game Boy Advance, returning dates and
//! times that are interoperable with the [`time`](https://crates.io/crates/time) library.
//!
//! # Example
//! Access to the RTC is done through the [`Clock`](https://docs.rs/gba_clock/latest/gba_clock/struct.Clock.html) type. Create a `Clock` using the current time and use the returned instance to access the current time.
//!
//! ``` no_run
//! use gba_clock::Clock;
//! use time::{
//!     Date,
//!     Month,
//!     PrimitiveDateTime,
//!     Time,
//! };
//!
//! let current_time = PrimitiveDateTime::new(
//!     Date::from_calendar_date(2001, Month::March, 21).expect("invalid date"),
//!     Time::from_hms(11, 30, 0).expect("invalid time"),
//! );
//! let clock = Clock::new(current_time).expect("could not communicate with the RTC");
//!
//! // Read the current time whenever you need.
//! let time = clock
//!     .read_datetime()
//!     .expect("could not read the current time");
//! ```

#![no_std]

mod bcd;
mod date_time;
mod error;
mod gpio;

pub use error::Error;

#[cfg(feature = "serde")]
use core::{
    fmt,
    fmt::Formatter,
    str,
};
use date_time::{
    RtcDateTimeOffset,
    RtcTimeOffset,
};
use deranged::RangedU32;
use gpio::{
    enable,
    is_test_mode,
    reset,
    set_status,
    try_read_datetime_offset,
    try_read_status,
    try_read_time_offset,
    Status,
};
#[cfg(feature = "serde")]
use serde::{
    de,
    de::{
        Deserialize,
        Deserializer,
        MapAccess,
        SeqAccess,
        Unexpected,
        Visitor,
    },
    ser::{
        Serialize,
        SerializeStruct,
        Serializer,
    },
};
use time::{
    Date,
    PrimitiveDateTime,
    Time,
};

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
    rtc_offset: RtcDateTimeOffset,
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

        let rtc_offset = try_read_datetime_offset()?;

        Ok(Self {
            base_date: datetime.date(),
            rtc_offset: rtc_offset - datetime.time().into(),
        })
    }

    /// Reads the currently stored date and time.
    pub fn read_datetime(&self) -> Result<PrimitiveDateTime, Error> {
        let rtc_offset = try_read_datetime_offset()?;

        let duration = if rtc_offset.0 >= self.rtc_offset.0 {
            RtcDateTimeOffset(unsafe { rtc_offset.0.unchecked_sub(self.rtc_offset.0.get()) }).into()
        } else {
            RtcDateTimeOffset(unsafe {
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
        let rtc_offset = try_read_datetime_offset()?;
        self.base_date = datetime.date();
        self.rtc_offset = rtc_offset - datetime.time().into();
        Ok(())
    }

    /// Reads the currently stored time.
    ///
    /// This is always faster than using [`Clock::read_datetime()`], as it only requires reading
    /// three bytes from the RTC instead of seven.
    pub fn read_time(&self) -> Result<Time, Error> {
        let rtc_time_offset = try_read_time_offset()?;
        let stored_time_offset: RtcTimeOffset = self.rtc_offset.into();

        Ok(if rtc_time_offset.0 >= stored_time_offset.0 {
            RtcTimeOffset(unsafe { rtc_time_offset.0.unchecked_sub(stored_time_offset.0.get()) })
                .into()
        } else {
            RtcTimeOffset(unsafe {
                RangedU32::MAX
                    .unchecked_sub(stored_time_offset.0.get())
                    .unchecked_add(rtc_time_offset.0.get())
                    .unchecked_add(1)
            })
            .into()
        })
    }

    /// Writes a new time.
    ///
    /// This preserves the stored date.
    ///
    /// Note that this does not actually change the stored time in the RTC itself. While RTC values
    /// are writable on real hardware, they are often not writable in GBA emulators. Therefore, the
    /// date and time are stored as being offset from the current RTC date and time to maintain
    /// maximum compatibility.
    pub fn write_time(&mut self, time: Time) -> Result<(), Error> {
        let rtc_time_offset = try_read_time_offset()?;
        let stored_time_offset = RtcTimeOffset::from(self.rtc_offset);

        let current_time: Time = if rtc_time_offset.0 >= stored_time_offset.0 {
            RtcTimeOffset(unsafe { rtc_time_offset.0.unchecked_sub(stored_time_offset.0.get()) })
                .into()
        } else {
            RtcTimeOffset(unsafe {
                RangedU32::MAX
                    .unchecked_sub(stored_time_offset.0.get())
                    .unchecked_add(rtc_time_offset.0.get())
                    .unchecked_add(1)
            })
            .into()
        };

        // This difference will be within ±86,399. It can therefore fit within an i32.
        let delta = (current_time - time).whole_seconds() as i32;
        if delta.is_negative() {
            self.rtc_offset -=
                RtcDateTimeOffset(unsafe { RangedU32::new_unchecked(delta.unsigned_abs()) });
        } else {
            self.rtc_offset +=
                RtcDateTimeOffset(unsafe { RangedU32::new_unchecked(delta.unsigned_abs()) });
        }

        Ok(())
    }
}

#[cfg(feature = "serde")]
impl Serialize for Clock {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut r#struct = serializer.serialize_struct("Clock", 2)?;
        r#struct.serialize_field("base_date", &self.base_date)?;
        r#struct.serialize_field("rtc_offset", &self.rtc_offset)?;
        r#struct.end()
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Clock {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum Field {
            BaseDate,
            RtcOffset,
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct FieldVisitor;

                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                        formatter.write_str("`base_date` or `rtc_offset`")
                    }

                    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            0 => Ok(Field::BaseDate),
                            1 => Ok(Field::RtcOffset),
                            _ => Err(de::Error::invalid_value(Unexpected::Unsigned(value), &self)),
                        }
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            "base_date" => Ok(Field::BaseDate),
                            "rtc_offset" => Ok(Field::RtcOffset),
                            _ => Err(de::Error::unknown_field(value, FIELDS)),
                        }
                    }

                    fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            b"base_date" => Ok(Field::BaseDate),
                            b"rtc_offset" => Ok(Field::RtcOffset),
                            _ => {
                                let utf8_value =
                                    str::from_utf8(value).unwrap_or("\u{fffd}\u{fffd}\u{fffd}");
                                Err(de::Error::unknown_field(utf8_value, FIELDS))
                            }
                        }
                    }
                }

                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct ClockVisitor;

        impl<'de> Visitor<'de> for ClockVisitor {
            type Value = Clock;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("struct Clock")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let base_date = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let rtc_offset = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;
                Ok(Clock {
                    base_date,
                    rtc_offset,
                })
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut base_date = None;
                let mut rtc_offset = None;

                while let Some(field) = map.next_key()? {
                    match field {
                        Field::BaseDate => {
                            if base_date.is_some() {
                                return Err(de::Error::duplicate_field("base_date"));
                            }
                            base_date = Some(map.next_value()?);
                        }
                        Field::RtcOffset => {
                            if rtc_offset.is_some() {
                                return Err(de::Error::duplicate_field("rtc_offset"));
                            }
                            rtc_offset = Some(map.next_value()?);
                        }
                    }
                }

                Ok(Clock {
                    base_date: base_date.ok_or_else(|| de::Error::missing_field("base_date"))?,
                    rtc_offset: rtc_offset.ok_or_else(|| de::Error::missing_field("rtc_offset"))?,
                })
            }
        }

        const FIELDS: &[&str] = &["base_date", "rtc_offset"];
        let result = deserializer.deserialize_struct("Clock", FIELDS, ClockVisitor);
        if result.is_ok() {
            // Enable operations with the RTC via General Purpose I/O (GPIO).
            enable();
            set_status(Status::HOUR_24);
        }
        result
    }
}
