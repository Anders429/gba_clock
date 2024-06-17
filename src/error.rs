//! Errors that may occur when interacting with the RTC.

#[cfg(feature = "serde")]
use core::str;
use core::{
    fmt,
    fmt::{
        Display,
        Formatter,
    },
};
#[cfg(feature = "serde")]
use serde::{
    de,
    de::{
        EnumAccess,
        Unexpected,
        VariantAccess,
        Visitor,
    },
    Deserialize,
    Deserializer,
    Serialize,
    Serializer,
};

/// Errors that may occur when interacting with the RTC.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Error {
    PowerFailure,
    TestMode,
    AmPmBitPresent,
    InvalidStatus(u8),
    InvalidMonth(u8),
    InvalidDay(u8),
    InvalidHour(u8),
    InvalidMinute(u8),
    InvalidSecond(u8),
    InvalidBinaryCodedDecimal(u8),
    Overflow,
    NotEnabled,
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::PowerFailure => formatter.write_str("RTC power failure"),
            Self::TestMode => formatter.write_str("RTC is in test mode"),
            Self::AmPmBitPresent => formatter.write_str("RTC is not in 24-hour mode"),
            Self::InvalidStatus(value) => {
                write!(formatter, "RTC returned an invalid status: {}", value)
            }
            Self::InvalidMonth(value) => {
                write!(formatter, "RTC returned an invalid month: {}", value)
            }
            Self::InvalidDay(value) => write!(formatter, "RTC returned an invalid day: {}", value),
            Self::InvalidHour(value) => {
                write!(formatter, "RTC returned an invalid hour: {}", value)
            }
            Self::InvalidMinute(value) => {
                write!(formatter, "RTC returned an invalid minute: {}", value)
            }
            Self::InvalidSecond(value) => {
                write!(formatter, "RTC returned an invalid second: {}", value)
            }
            Self::InvalidBinaryCodedDecimal(value) => {
                write!(
                    formatter,
                    "RTC returned a value that was not a binary coded decimal: {}",
                    value
                )
            }
            Self::Overflow => formatter.write_str("the stored time is too large to be represented"),
            Self::NotEnabled => formatter.write_str("the RTC GPIO port is not enabled"),
        }
    }
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
            Self::InvalidStatus(value) => {
                serializer.serialize_newtype_variant("Error", 3, "InvalidStatus", value)
            }
            Self::InvalidMonth(value) => {
                serializer.serialize_newtype_variant("Error", 4, "InvalidMonth", value)
            }
            Self::InvalidDay(value) => {
                serializer.serialize_newtype_variant("Error", 5, "InvalidDay", value)
            }
            Self::InvalidHour(value) => {
                serializer.serialize_newtype_variant("Error", 6, "InvalidHour", value)
            }
            Self::InvalidMinute(value) => {
                serializer.serialize_newtype_variant("Error", 7, "InvalidMinute", value)
            }
            Self::InvalidSecond(value) => {
                serializer.serialize_newtype_variant("Error", 8, "InvalidSecond", value)
            }
            Self::InvalidBinaryCodedDecimal(value) => {
                serializer.serialize_newtype_variant("Error", 9, "InvalidBinaryCodedDecimal", value)
            }
            Self::Overflow => serializer.serialize_unit_variant("Error", 10, "Overflow"),
            Self::NotEnabled => serializer.serialize_unit_variant("Error", 11, "NotEnabled"),
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
            NotEnabled,
        }

        impl<'de> Deserialize<'de> for Variant {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct VariantVisitor;

                impl<'de> Visitor<'de> for VariantVisitor {
                    type Value = Variant;

                    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                        formatter.write_str("`PowerFailure`, `TestMode`, `AmPmBitPresent`, `InvalidStatus`, `InvalidMonth`, `InvalidDay`, `InvalidHour`, `InvalidMinute`, `InvalidSecond`, `InvalidBinaryCodedDecimal`, `Overflow`, or `NotEnabled`")
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
                            11 => Ok(Variant::NotEnabled),
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
                            "NotEnabled" => Ok(Variant::NotEnabled),
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
                            b"NotEnabled" => Ok(Variant::NotEnabled),
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

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("enum Error")
            }

            fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
            where
                A: EnumAccess<'de>,
            {
                let (variant, access) = data.variant()?;

                Ok(match variant {
                    Variant::PowerFailure => {
                        access.unit_variant()?;
                        Error::PowerFailure
                    }
                    Variant::TestMode => {
                        access.unit_variant()?;
                        Error::TestMode
                    }
                    Variant::AmPmBitPresent => {
                        access.unit_variant()?;
                        Error::AmPmBitPresent
                    }
                    Variant::InvalidStatus => Error::InvalidStatus(access.newtype_variant()?),
                    Variant::InvalidMonth => Error::InvalidMonth(access.newtype_variant()?),
                    Variant::InvalidDay => Error::InvalidDay(access.newtype_variant()?),
                    Variant::InvalidHour => Error::InvalidHour(access.newtype_variant()?),
                    Variant::InvalidMinute => Error::InvalidMinute(access.newtype_variant()?),
                    Variant::InvalidSecond => Error::InvalidSecond(access.newtype_variant()?),
                    Variant::InvalidBinaryCodedDecimal => {
                        Error::InvalidBinaryCodedDecimal(access.newtype_variant()?)
                    }
                    Variant::Overflow => {
                        access.unit_variant()?;
                        Error::Overflow
                    }
                    Variant::NotEnabled => {
                        access.unit_variant()?;
                        Error::NotEnabled
                    }
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
            "NotEnabled",
        ];
        deserializer.deserialize_enum("Error", VARIANTS, ErrorVisitor)
    }
}
