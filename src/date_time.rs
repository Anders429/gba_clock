//! Types and logic for representing and understanding the date and time stored within the RTC.

use core::{fmt, fmt::Debug, ops::Sub};
use deranged::{RangedU32, RangedU8};
#[cfg(feature = "serde")]
use serde::{
    de::{Deserialize, Deserializer, Visitor},
    ser::{Serialize, Serializer},
};
use time::{Date, Duration, Month, Time};

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
pub(crate) struct RtcOffset(pub(crate) RangedU32<0, 3_155_759_999>);

impl RtcOffset {
    pub(crate) fn new(
        year: Year,
        month: Month,
        day: Day,
        hour: Hour,
        minute: Minute,
        second: Second,
    ) -> RtcOffset {
        // SAFETY: The output of `calculate_rtc_offset()` is guaranteed to be within the range.
        RtcOffset(unsafe {
            RangedU32::new_unchecked(calculate_rtc_offset(year, month, day, hour, minute, second))
        })
    }
}

impl From<Time> for RtcOffset {
    fn from(time: Time) -> Self {
        Self(unsafe {
            RangedU32::new_unchecked(
                time.hour() as u32 * 3600 + time.minute() as u32 * 60 + time.second() as u32,
            )
        })
    }
}

impl From<RtcOffset> for Duration {
    fn from(rtc_offset: RtcOffset) -> Self {
        Self::seconds(rtc_offset.0.get().into())
    }
}

impl Sub for RtcOffset {
    type Output = RtcOffset;

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

impl Debug for RtcOffset {
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
impl Serialize for RtcOffset {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_newtype_struct("RtcOffset", &self.0)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for RtcOffset {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RtcOffsetVisitor;

        impl<'de> Visitor<'de> for RtcOffsetVisitor {
            type Value = RtcOffset;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct RtcOffset")
            }

            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: Deserializer<'de>,
            {
                Ok(RtcOffset(RangedU32::deserialize(deserializer)?))
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

#[cfg(test)]
mod tests {
    use super::{calculate_rtc_offset, Day, Hour, Minute, Second, Year};
    use deranged::RangedU8;
    use time::Month;

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
