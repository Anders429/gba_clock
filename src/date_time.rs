//! Types and logic for representing and understanding the date and time stored within the RTC.

use core::{
    fmt,
    fmt::Debug,
    ops::{
        AddAssign,
        Sub,
        SubAssign,
    },
};
use deranged::{
    RangedU32,
    RangedU8,
};
#[cfg(feature = "serde")]
use serde::{
    de::{
        Deserialize,
        Deserializer,
        Visitor,
    },
    ser::{
        Serialize,
        Serializer,
    },
};
use time::{
    Date,
    Duration,
    Month,
    Time,
};

/// A calendar year.
///
/// Specifically, this is the last two digits of the year. It represents a year in the range
/// 2000-2099.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Year(pub(crate) RangedU8<0, 99>);

/// A day within a month.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Day(pub(crate) RangedU8<1, 31>);

/// An hour of the day.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Hour(pub(crate) RangedU8<0, 23>);

/// A minute within an hour.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Minute(pub(crate) RangedU8<0, 59>);

/// A second within a minute.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct Second(pub(crate) RangedU8<0, 59>);

#[derive(Clone, Copy)]
pub(crate) struct RtcDateTimeOffset(pub(crate) RangedU32<0, 3_155_759_999>);

impl RtcDateTimeOffset {
    pub(crate) fn new(
        year: Year,
        month: Month,
        day: Day,
        hour: Hour,
        minute: Minute,
        second: Second,
    ) -> RtcDateTimeOffset {
        // SAFETY: The output of `calculate_rtc_offset()` is guaranteed to be within the range.
        RtcDateTimeOffset(unsafe {
            RangedU32::new_unchecked(calculate_rtc_offset(year, month, day, hour, minute, second))
        })
    }
}

impl From<Time> for RtcDateTimeOffset {
    fn from(time: Time) -> Self {
        Self(unsafe {
            RangedU32::new_unchecked(
                time.hour() as u32 * 3600 + time.minute() as u32 * 60 + time.second() as u32,
            )
        })
    }
}

impl From<RtcDateTimeOffset> for Duration {
    fn from(rtc_offset: RtcDateTimeOffset) -> Self {
        Self::seconds(rtc_offset.0.get().into())
    }
}

impl AddAssign for RtcDateTimeOffset {
    fn add_assign(&mut self, other: Self) {
        *self = Self(self.0.checked_add(other.0.get()).unwrap_or_else(|| {
            if self.0 > other.0 {
                // SAFETY: Subtracting `self.0` from the max range will always work. Also, since
                // `self.0` is larger, adding `other.0` afterwards will always be within the range.
                unsafe {
                    RangedU32::MAX
                        .unchecked_sub(self.0.get())
                        .unchecked_add(other.0.get())
                }
            } else {
                // SAFETY: Subtracting `other.0` from the max range will always work. Also, since
                // `other.0` is larger, adding `self.0` afterwards will always be within the range.
                unsafe {
                    RangedU32::MAX
                        .unchecked_sub(other.0.get())
                        .unchecked_add(self.0.get())
                }
            }
        }))
    }
}

impl Sub for RtcDateTimeOffset {
    type Output = RtcDateTimeOffset;

    fn sub(self, other: Self) -> Self::Output {
        Self(self.0.checked_sub(other.0.get()).unwrap_or_else(|| {
            // SAFETY: Since the previous `checked_sub` failed, `other` must be greater than
            // `self`. Additionally, both the difference of both values must be less than or equal
            // to the maximum value for the `RangedU32` and must also be greater than 0.
            unsafe {
                RangedU32::<0, 3_155_759_999>::MAX
                    .unchecked_sub(other.0.unchecked_sub(self.0.get()).get())
                    .unchecked_add(1)
            }
        }))
    }
}

impl SubAssign for RtcDateTimeOffset {
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other;
    }
}

impl Debug for RtcDateTimeOffset {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let origin =
            unsafe { Date::from_calendar_date(2000, Month::January, 1).unwrap_unchecked() }
                .midnight();
        let datetime = origin + Duration::seconds(self.0.get().into());

        formatter
            .debug_struct("RtcOffset")
            .field("year", &datetime.year())
            .field("month", &datetime.month())
            .field("day", &datetime.day())
            .field("hours", &datetime.hour())
            .field("minutes", &datetime.minute())
            .field("seconds", &datetime.second())
            .finish()
    }
}

#[cfg(feature = "serde")]
impl Serialize for RtcDateTimeOffset {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_newtype_struct("RtcOffset", &self.0)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for RtcDateTimeOffset {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RtcOffsetVisitor;

        impl<'de> Visitor<'de> for RtcOffsetVisitor {
            type Value = RtcDateTimeOffset;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct RtcOffset")
            }

            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                Ok(RtcDateTimeOffset(RangedU32::deserialize(deserializer)?))
            }
        }

        deserializer.deserialize_newtype_struct("RtcOffset", RtcOffsetVisitor)
    }
}

/// Calculates the number of seconds since the RTC's origin date.
pub(crate) fn calculate_rtc_offset(
    year: Year,
    month: Month,
    day: Day,
    hour: Hour,
    minute: Minute,
    second: Second,
) -> u32 {
    let days = year.0.get() as u32 * 365
        + if year.0.get() > 0 {
            (year.0.get() as u32 - 1) / 4 + 1
        } else {
            0
        }
        + match month {
            Month::January => 0,
            Month::February => 31,
            Month::March => 59,
            Month::April => 90,
            Month::May => 120,
            Month::June => 151,
            Month::July => 181,
            Month::August => 212,
            Month::September => 243,
            Month::October => 273,
            Month::November => 304,
            Month::December => 334,
        }
        + if year.0.get() % 4 == 0 && u8::from(month) > 2 {
            1
        } else {
            0
        }
        + day.0.get() as u32
        - 1;
    second.0.get() as u32 + minute.0.get() as u32 * 60 + hour.0.get() as u32 * 3600 + days * 86400
}

/// The current number of seconds stored in the RTC.
///
/// In other words, this is the number of seconds since midnight according to the RTC's clock.
#[derive(Eq, PartialEq)]
pub(crate) struct RtcTimeOffset(pub(crate) RangedU32<0, 86_399>);

impl RtcTimeOffset {
    /// Create a new offset using the hour, minute, and second read from the RTC.
    pub(crate) fn new(hour: Hour, minute: Minute, second: Second) -> RtcTimeOffset {
        RtcTimeOffset(unsafe {
            RangedU32::new_unchecked(
                hour.0.get() as u32 * 3600 + minute.0.get() as u32 * 60 + second.0.get() as u32,
            )
        })
    }
}

impl From<RtcDateTimeOffset> for RtcTimeOffset {
    fn from(rtc_offset: RtcDateTimeOffset) -> Self {
        // SAFETY: The remainder calculated here is guaranteed to be in the required range.
        RtcTimeOffset(unsafe { RangedU32::new_unchecked(rtc_offset.0.get() % 86400) })
    }
}

impl From<RtcTimeOffset> for Time {
    fn from(rtc_time_offset: RtcTimeOffset) -> Self {
        Time::MIDNIGHT + Duration::seconds(rtc_time_offset.0.get().into())
    }
}

impl Debug for RtcTimeOffset {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let origin = Time::MIDNIGHT;
        let time = origin + Duration::seconds(self.0.get().into());

        formatter
            .debug_struct("RtcTimeOffset")
            .field("hours", &time.hour())
            .field("minutes", &time.minute())
            .field("seconds", &time.second())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        calculate_rtc_offset,
        Day,
        Hour,
        Minute,
        RtcTimeOffset,
        Second,
        Year,
    };
    use deranged::{
        RangedU32,
        RangedU8,
    };
    use time::Month;

    #[test]
    fn rtc_time_offset_min() {
        assert_eq!(
            RtcTimeOffset::new(
                Hour(RangedU8::MIN),
                Minute(RangedU8::MIN),
                Second(RangedU8::MIN)
            ),
            RtcTimeOffset(RangedU32::MIN)
        );
    }

    #[test]
    fn rtc_time_offset_max() {
        assert_eq!(
            RtcTimeOffset::new(
                Hour(RangedU8::MAX),
                Minute(RangedU8::MAX),
                Second(RangedU8::MAX)
            ),
            RtcTimeOffset(RangedU32::MAX)
        );
    }

    #[test]
    fn rtc_time_offset_hours() {
        assert_eq!(
            RtcTimeOffset::new(
                Hour(RangedU8::new_static::<13>()),
                Minute(RangedU8::MIN),
                Second(RangedU8::MIN)
            ),
            RtcTimeOffset(RangedU32::new_static::<46800>())
        );
    }

    #[test]
    fn rtc_time_offset_minutes() {
        assert_eq!(
            RtcTimeOffset::new(
                Hour(RangedU8::MIN),
                Minute(RangedU8::new_static::<42>()),
                Second(RangedU8::MIN)
            ),
            RtcTimeOffset(RangedU32::new_static::<2520>())
        );
    }

    #[test]
    fn rtc_time_offset_seconds() {
        assert_eq!(
            RtcTimeOffset::new(
                Hour(RangedU8::MIN),
                Minute(RangedU8::MIN),
                Second(RangedU8::new_static::<42>())
            ),
            RtcTimeOffset(RangedU32::new_static::<42>())
        );
    }

    #[test]
    fn calculate_rtc_offset_min() {
        assert_eq!(
            calculate_rtc_offset(
                Year(RangedU8::new_static::<0>()),
                Month::January,
                Day(RangedU8::new_static::<1>()),
                Hour(RangedU8::new_static::<0>()),
                Minute(RangedU8::new_static::<0>()),
                Second(RangedU8::new_static::<0>())
            ),
            0
        );
    }

    #[test]
    fn calculate_rtc_offset_max() {
        assert_eq!(
            calculate_rtc_offset(
                Year(RangedU8::new_static::<99>()),
                Month::December,
                Day(RangedU8::new_static::<31>()),
                Hour(RangedU8::new_static::<23>()),
                Minute(RangedU8::new_static::<59>()),
                Second(RangedU8::new_static::<59>())
            ),
            3_155_759_999
        );
    }

    #[test]
    fn calculate_rtc_offset_seconds() {
        assert_eq!(
            calculate_rtc_offset(
                Year(RangedU8::new_static::<0>()),
                Month::January,
                Day(RangedU8::new_static::<1>()),
                Hour(RangedU8::new_static::<0>()),
                Minute(RangedU8::new_static::<0>()),
                Second(RangedU8::new_static::<42>())
            ),
            42
        );
    }

    #[test]
    fn calculate_rtc_offset_minutes() {
        assert_eq!(
            calculate_rtc_offset(
                Year(RangedU8::new_static::<0>()),
                Month::January,
                Day(RangedU8::new_static::<1>()),
                Hour(RangedU8::new_static::<0>()),
                Minute(RangedU8::new_static::<42>()),
                Second(RangedU8::new_static::<0>())
            ),
            2_520
        );
    }

    #[test]
    fn calculate_rtc_offset_hours() {
        assert_eq!(
            calculate_rtc_offset(
                Year(RangedU8::new_static::<0>()),
                Month::January,
                Day(RangedU8::new_static::<1>()),
                Hour(RangedU8::new_static::<18>()),
                Minute(RangedU8::new_static::<0>()),
                Second(RangedU8::new_static::<0>())
            ),
            64_800
        );
    }

    #[test]
    fn calculate_rtc_offset_days() {
        assert_eq!(
            calculate_rtc_offset(
                Year(RangedU8::new_static::<0>()),
                Month::January,
                Day(RangedU8::new_static::<27>()),
                Hour(RangedU8::new_static::<0>()),
                Minute(RangedU8::new_static::<0>()),
                Second(RangedU8::new_static::<0>())
            ),
            2_246_400
        );
    }

    #[test]
    fn calculate_rtc_offset_months() {
        assert_eq!(
            calculate_rtc_offset(
                Year(RangedU8::new_static::<0>()),
                Month::October,
                Day(RangedU8::new_static::<1>()),
                Hour(RangedU8::new_static::<0>()),
                Minute(RangedU8::new_static::<0>()),
                Second(RangedU8::new_static::<0>())
            ),
            23_673_600
        );
    }

    #[test]
    fn calculate_rtc_offset_years() {
        assert_eq!(
            calculate_rtc_offset(
                Year(RangedU8::new_static::<42>()),
                Month::January,
                Day(RangedU8::new_static::<1>()),
                Hour(RangedU8::new_static::<0>()),
                Minute(RangedU8::new_static::<0>()),
                Second(RangedU8::new_static::<0>())
            ),
            1_325_462_400
        );
    }
}
