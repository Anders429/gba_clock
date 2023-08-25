//! Binary coded decimal.
//!
//! This module contains a wrapper for a byte that is a BCD, as well as logic for converting a BCD
//! to other types.

use crate::{Day, Error, Hour, Minute, Second, Year};
use time::Month;

/// Binary coded decimal.
///
/// The S-3511A stores values as BCD, meaning each half-byte represents a digit. For example, the
/// value `12` is not represented as `0x0c`, but is instead represented as `0x12`.
///
/// The contained value must be a valid BCD value, meaning neither half-byte can be greater than
/// `0x9`.
#[derive(Clone, Copy)]
pub(crate) struct Bcd(u8);

impl Bcd {
    /// Converts the binary coded decimal to its equivalent binary form.
    ///
    /// This is guaranteed to result in a value less than `100`.
    fn to_binary(self) -> u8 {
        10 * (self.0 >> 4 & 0x0f) + (self.0 & 0x0f)
    }
}

/// Directly wraps a byte as a BCD, or returns an error if the byte is not a valid BCD.
impl TryFrom<u8> for Bcd {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value < 0xa0 || (value & 0x0f < 0x0a) {
            Ok(Self(value))
        } else {
            Err(Error::InvalidBinaryCodedDecimal)
        }
    }
}

/// Interprets the BCD as a year.
impl From<Bcd> for Year {
    fn from(bcd: Bcd) -> Self {
        // `Bcd::to_binary()` will always return a value less than 99.
        Year(bcd.to_binary())
    }
}

/// Interprets the BCD as a month.
impl TryFrom<Bcd> for Month {
    type Error = Error;

    fn try_from(value: Bcd) -> Result<Self, Self::Error> {
        value
            .to_binary()
            .try_into()
            .map_err(Error::TimeComponentRange)
    }
}

/// Interprets the BCD as a day.
impl TryFrom<Bcd> for Day {
    type Error = Error;

    fn try_from(bcd: Bcd) -> Result<Self, Self::Error> {
        let day = bcd.to_binary();
        if day == 0 || day > 31 {
            Err(Error::InvalidDay)
        } else {
            Ok(Self(day))
        }
    }
}

/// Interprets the BCD as an hour.
impl TryFrom<Bcd> for Hour {
    type Error = Error;

    fn try_from(bcd: Bcd) -> Result<Self, Self::Error> {
        // Check for the am/pm bit.
        if bcd.0 & 0b1000_0000 != 0 {
            return Err(Error::AmPmBitPresent);
        }
        let hour = bcd.to_binary();
        if hour > 23 {
            Err(Error::InvalidHour)
        } else {
            Ok(Self(hour))
        }
    }
}

/// Interprets the BCD as a minute.
impl TryFrom<Bcd> for Minute {
    type Error = Error;

    fn try_from(bcd: Bcd) -> Result<Self, Self::Error> {
        let minute = bcd.to_binary();
        if minute > 59 {
            Err(Error::InvalidMinute)
        } else {
            Ok(Self(minute))
        }
    }
}

/// Interprets the BCD as a second.
impl TryFrom<Bcd> for Second {
    type Error = Error;

    fn try_from(bcd: Bcd) -> Result<Self, Self::Error> {
        // Check for test bit.
        if bcd.0 & 0b1000_0000 != 0 {
            return Err(Error::TestMode);
        }
        let second = bcd.to_binary();
        if second > 59 {
            Err(Error::InvalidSecond)
        } else {
            Ok(Self(second))
        }
    }
}
