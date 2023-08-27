//! A real-time clock library for the GBA.
//!
//! Provides access to the RTC for programs running on a Game Boy Advance, returning dates and
//! times that are interoperable with the [`time`](https://crates.io/crates/time) library.
//!
//! #Example
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
mod gpio;

#[cfg(feature = "serde")]
use core::{
    fmt,
    str,
};
use date_time::RtcOffset;
use deranged::RangedU32;
use gpio::{
    enable,
    is_test_mode,
    reset,
    set_status,
    try_read_datetime,
    try_read_status,
    Status,
};
#[cfg(feature = "serde")]
use serde::{
    de,
    de::{
        Deserialize,
        Deserializer,
        EnumAccess,
        MapAccess,
        SeqAccess,
        Unexpected,
        VariantAccess,
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
};

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

#[cfg(feature = "serde")]
impl Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::PowerFailure => serializer.serialize_unit_variant("Error", 0, "PowerFailure"),
            Self::TestMode => serializer.serialize_unit_variant("Error", 1, "TestMode"),
            Self::AmPmBitPresent => serializer.serialize_unit_variant("Error", 2, "AmPmBitPresent"),
            Self::InvalidStatus => serializer.serialize_unit_variant("Error", 3, "InvalidStatus"),
            Self::InvalidMonth => serializer.serialize_unit_variant("Error", 4, "InvalidMonth"),
            Self::InvalidDay => serializer.serialize_unit_variant("Error", 5, "InvalidDay"),
            Self::InvalidHour => serializer.serialize_unit_variant("Error", 6, "InvalidHour"),
            Self::InvalidMinute => serializer.serialize_unit_variant("Error", 7, "InvalidMinute"),
            Self::InvalidSecond => serializer.serialize_unit_variant("Error", 8, "InvalidSecond"),
            Self::InvalidBinaryCodedDecimal => {
                serializer.serialize_unit_variant("Error", 9, "InvalidBinaryCodedDecimal")
            }
            Self::Overflow => serializer.serialize_unit_variant("Error", 10, "Overflow"),
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Error {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum Variant {
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

        impl<'de> Deserialize<'de> for Variant {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct VariantVisitor;

                impl<'de> Visitor<'de> for VariantVisitor {
                    type Value = Variant;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("`PowerFailure`, `TestMode`, `AmPmBitPresent`, `InvalidStatus`, `InvalidMonth`, `InvalidDay`, `InvalidHour`, `InvalidMinute`, `InvalidSecond`, `InvalidBinaryCodedDecimal`, or `Overflow`")
                    }

                    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            0 => Ok(Variant::PowerFailure),
                            1 => Ok(Variant::TestMode),
                            2 => Ok(Variant::AmPmBitPresent),
                            3 => Ok(Variant::InvalidStatus),
                            4 => Ok(Variant::InvalidMonth),
                            5 => Ok(Variant::InvalidDay),
                            6 => Ok(Variant::InvalidHour),
                            7 => Ok(Variant::InvalidMinute),
                            8 => Ok(Variant::InvalidSecond),
                            9 => Ok(Variant::InvalidBinaryCodedDecimal),
                            10 => Ok(Variant::Overflow),
                            _ => Err(de::Error::invalid_value(Unexpected::Unsigned(value), &self)),
                        }
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            "PowerFailure" => Ok(Variant::PowerFailure),
                            "TestMode" => Ok(Variant::TestMode),
                            "AmPmBitPresent" => Ok(Variant::AmPmBitPresent),
                            "InvalidStatus" => Ok(Variant::InvalidStatus),
                            "InvalidMonth" => Ok(Variant::InvalidMonth),
                            "InvalidDay" => Ok(Variant::InvalidDay),
                            "InvalidHour" => Ok(Variant::InvalidHour),
                            "InvalidMinute" => Ok(Variant::InvalidMinute),
                            "InvalidSecond" => Ok(Variant::InvalidSecond),
                            "InvalidBinaryCodedDecimal" => Ok(Variant::InvalidBinaryCodedDecimal),
                            "Overflow" => Ok(Variant::Overflow),
                            _ => Err(de::Error::unknown_variant(value, VARIANTS)),
                        }
                    }

                    fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        match value {
                            b"PowerFailure" => Ok(Variant::PowerFailure),
                            b"TestMode" => Ok(Variant::TestMode),
                            b"AmPmBitPresent" => Ok(Variant::AmPmBitPresent),
                            b"InvalidStatus" => Ok(Variant::InvalidStatus),
                            b"InvalidMonth" => Ok(Variant::InvalidMonth),
                            b"InvalidDay" => Ok(Variant::InvalidDay),
                            b"InvalidHour" => Ok(Variant::InvalidHour),
                            b"InvalidMinute" => Ok(Variant::InvalidMinute),
                            b"InvalidSecond" => Ok(Variant::InvalidSecond),
                            b"InvalidBinaryCodedDecimal" => Ok(Variant::InvalidBinaryCodedDecimal),
                            b"Overflow" => Ok(Variant::Overflow),
                            _ => {
                                let utf8_value =
                                    str::from_utf8(value).unwrap_or("\u{fffd}\u{fffd}\u{fffd}");
                                Err(de::Error::unknown_variant(utf8_value, VARIANTS))
                            }
                        }
                    }
                }

                deserializer.deserialize_identifier(VariantVisitor)
            }
        }

        struct ErrorVisitor;

        impl<'de> Visitor<'de> for ErrorVisitor {
            type Value = Error;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("enum Error")
            }

            fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
            where
                A: EnumAccess<'de>,
            {
                let (variant, access) = data.variant()?;
                access.unit_variant()?;
                Ok(match variant {
                    Variant::PowerFailure => Error::PowerFailure,
                    Variant::TestMode => Error::TestMode,
                    Variant::AmPmBitPresent => Error::AmPmBitPresent,
                    Variant::InvalidStatus => Error::InvalidStatus,
                    Variant::InvalidMonth => Error::InvalidMonth,
                    Variant::InvalidDay => Error::InvalidDay,
                    Variant::InvalidHour => Error::InvalidHour,
                    Variant::InvalidMinute => Error::InvalidMinute,
                    Variant::InvalidSecond => Error::InvalidSecond,
                    Variant::InvalidBinaryCodedDecimal => Error::InvalidBinaryCodedDecimal,
                    Variant::Overflow => Error::Overflow,
                })
            }
        }

        const VARIANTS: &[&str] = &[
            "PowerFailure",
            "TestMode",
            "AmPmBitPresent",
            "InvalidStatus",
            "InvalidMonth",
            "InvalidDay",
            "InvalidHour",
            "InvalidMinute",
            "InvalidSecond",
            "InvalidBinaryCodedDecimal",
            "Overflow",
        ];
        deserializer.deserialize_enum("Error", VARIANTS, ErrorVisitor)
    }
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

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
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

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
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
        deserializer.deserialize_struct("Clock", FIELDS, ClockVisitor)
    }
}
