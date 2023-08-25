//! Types and logic for representing and understanding the date and time stored within the RTC.

use deranged::RangedU8;
use time::Month;

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
        + (year.0.get() as u32 - 1) / 4
        + 1
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
        + if year.0.get() % 4 == 0 { 1 } else { 0 }
        + day.0.get() as u32;
    second.0.get() as u32 + minute.0.get() as u32 * 60 + hour.0.get() as u32 + days * 86400
}
