//! Binary coded decimal.
//!
//! This module contains a wrapper for a byte that is a BCD, as well as logic for converting a BCD
//! to other types.

use crate::{
    date_time::{
        Day,
        Hour,
        Minute,
        Second,
        Year,
    },
    Error,
};
use deranged::RangedU8;
use time::Month;

/// Binary coded decimal.
///
/// The S-3511A stores values as BCD, meaning each half-byte represents a digit. For example, the
/// value `12` is not represented as `0x0c`, but is instead represented as `0x12`.
///
/// The contained value must be a valid BCD value, meaning neither half-byte can be greater than
/// `0x9`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Bcd(u8);

impl Bcd {
    /// Converts the binary coded decimal to its equivalent binary form.
    ///
    /// This is guaranteed to result in a value less than `100`.
    fn to_binary(self) -> RangedU8<0, 99> {
        // SAFETY: This conversion is guaranteed to result in a value between 0 and 99, since the
        // original value is guaranteed to be a valid BCD value.
        unsafe { RangedU8::new_unchecked(10 * (self.0 >> 4 & 0x0f) + (self.0 & 0x0f)) }
    }
}

/// Directly wraps a byte as a BCD, or returns an error if the byte is not a valid BCD.
impl TryFrom<u8> for Bcd {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value < 0xa0 && (value & 0x0f < 0x0a) {
            Ok(Self(value))
        } else {
            Err(Error::InvalidBinaryCodedDecimal(value))
        }
    }
}

/// Interprets the BCD as a year.
impl From<Bcd> for Year {
    fn from(bcd: Bcd) -> Self {
        Year(bcd.to_binary())
    }
}

/// Interprets the BCD as a month.
impl TryFrom<Bcd> for Month {
    type Error = Error;

    fn try_from(bcd: Bcd) -> Result<Self, Self::Error> {
        let value = bcd.to_binary().get();
        value.try_into().map_err(|_| Error::InvalidMonth(value))
    }
}

/// Interprets the BCD as a day.
impl TryFrom<Bcd> for Day {
    type Error = Error;

    fn try_from(bcd: Bcd) -> Result<Self, Self::Error> {
        Ok(Self(
            bcd.to_binary()
                .narrow()
                .ok_or(Error::InvalidDay(bcd.to_binary().get()))?,
        ))
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
        Ok(Self(
            bcd.to_binary()
                .narrow()
                .ok_or(Error::InvalidHour(bcd.to_binary().get()))?,
        ))
    }
}

/// Interprets the BCD as a minute.
impl TryFrom<Bcd> for Minute {
    type Error = Error;

    fn try_from(bcd: Bcd) -> Result<Self, Self::Error> {
        Ok(Self(
            bcd.to_binary()
                .narrow()
                .ok_or(Error::InvalidMinute(bcd.to_binary().get()))?,
        ))
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
        Ok(Self(
            bcd.to_binary()
                .narrow()
                .ok_or(Error::InvalidSecond(bcd.to_binary().get()))?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::Bcd;
    use crate::{
        date_time::{
            Day,
            Hour,
            Minute,
            Second,
            Year,
        },
        Error,
    };
    use claims::{
        assert_err_eq,
        assert_ok_eq,
    };
    use deranged::RangedU8;
    use gba_test::test;
    use time::Month;

    #[test]
    fn to_binary() {
        assert_eq!(Bcd(0x12).to_binary(), RangedU8::<0, 99>::new_static::<12>());
    }

    #[test]
    fn to_binary_min() {
        assert_eq!(Bcd(0x00).to_binary(), RangedU8::<0, 99>::new_static::<0>());
    }

    #[test]
    fn to_binary_max() {
        assert_eq!(Bcd(0x99).to_binary(), RangedU8::<0, 99>::new_static::<99>());
    }

    #[test]
    fn from_byte() {
        assert_ok_eq!(Bcd::try_from(0x12), Bcd(0x12));
    }

    #[test]
    fn from_byte_min() {
        assert_ok_eq!(Bcd::try_from(0x00), Bcd(0x00));
    }

    #[test]
    fn from_byte_max() {
        assert_ok_eq!(Bcd::try_from(0x99), Bcd(0x99));
    }

    #[test]
    fn from_byte_upper_out_of_bounds() {
        assert_err_eq!(Bcd::try_from(0xc5), Error::InvalidBinaryCodedDecimal(0xc5));
    }

    #[test]
    fn from_byte_lower_out_of_bounds() {
        assert_err_eq!(Bcd::try_from(0x5c), Error::InvalidBinaryCodedDecimal(0x5c));
    }

    #[test]
    fn into_year_single_digit() {
        assert_eq!(Year::from(Bcd(0x08)), Year(RangedU8::new_static::<8>()));
    }

    #[test]
    fn into_year_double_digit() {
        assert_eq!(Year::from(Bcd(0x23)), Year(RangedU8::new_static::<23>()));
    }

    #[test]
    fn try_into_month_single_digit() {
        assert_ok_eq!(Month::try_from(Bcd(0x07)), Month::July);
    }

    #[test]
    fn try_into_month_double_digit() {
        assert_ok_eq!(Month::try_from(Bcd(0x12)), Month::December);
    }

    #[test]
    fn try_into_month_fails_zero() {
        assert_err_eq!(Month::try_from(Bcd(0x00)), Error::InvalidMonth(0));
    }

    #[test]
    fn try_into_month_fails_too_high() {
        assert_err_eq!(Month::try_from(Bcd(0x13)), Error::InvalidMonth(13));
    }

    #[test]
    fn try_into_day_single_digit() {
        assert_ok_eq!(Day::try_from(Bcd(0x05)), Day(RangedU8::new_static::<5>()));
    }

    #[test]
    fn try_into_day_double_digit() {
        assert_ok_eq!(Day::try_from(Bcd(0x31)), Day(RangedU8::new_static::<31>()));
    }

    #[test]
    fn try_into_day_fails_zero() {
        assert_err_eq!(Day::try_from(Bcd(0x00)), Error::InvalidDay(0));
    }

    #[test]
    fn try_into_day_fails_too_high() {
        assert_err_eq!(Day::try_from(Bcd(0x32)), Error::InvalidDay(32));
    }

    #[test]
    fn try_into_hour_single_digit() {
        assert_ok_eq!(Hour::try_from(Bcd(0x03)), Hour(RangedU8::new_static::<3>()));
    }

    #[test]
    fn try_into_hour_double_digit() {
        assert_ok_eq!(
            Hour::try_from(Bcd(0x19)),
            Hour(RangedU8::new_static::<19>())
        );
    }

    #[test]
    fn try_into_hour_fails_too_high() {
        assert_err_eq!(Hour::try_from(Bcd(0x24)), Error::InvalidHour(24));
    }

    #[test]
    fn try_into_hour_fails_am_pm_bit() {
        assert_err_eq!(Hour::try_from(Bcd(0x94)), Error::AmPmBitPresent);
    }

    #[test]
    fn try_into_minute_single_digit() {
        assert_ok_eq!(
            Minute::try_from(Bcd(0x08)),
            Minute(RangedU8::new_static::<8>())
        );
    }

    #[test]
    fn try_into_minute_double_digit() {
        assert_ok_eq!(
            Minute::try_from(Bcd(0x57)),
            Minute(RangedU8::new_static::<57>())
        );
    }

    #[test]
    fn try_into_minute_fails_too_high() {
        assert_err_eq!(Minute::try_from(Bcd(0x60)), Error::InvalidMinute(60));
    }

    #[test]
    fn try_into_second_single_digit() {
        assert_ok_eq!(
            Second::try_from(Bcd(0x02)),
            Second(RangedU8::new_static::<2>())
        );
    }

    #[test]
    fn try_into_second_double_digit() {
        assert_ok_eq!(
            Second::try_from(Bcd(0x44)),
            Second(RangedU8::new_static::<44>())
        );
    }

    #[test]
    fn try_into_second_fails_too_high() {
        assert_err_eq!(Second::try_from(Bcd(0x60)), Error::InvalidSecond(60));
    }

    #[test]
    fn try_into_second_fails_test_bit() {
        assert_err_eq!(Second::try_from(Bcd(0x80)), Error::TestMode);
    }
}
