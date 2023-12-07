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
    InvalidStatus,
    InvalidMonth,
    InvalidDay,
    InvalidHour,
    InvalidMinute,
    InvalidSecond,
    InvalidBinaryCodedDecimal,
    Overflow,
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        match self {
            Self::PowerFailure => formatter.write_str("RTC power failure"),
            Self::TestMode => formatter.write_str("RTC is in test mode"),
            Self::AmPmBitPresent => formatter.write_str("RTC is not in 24-hour mode"),
            Self::InvalidStatus => formatter.write_str("RTC returned an invalid status"),
            Self::InvalidMonth => formatter.write_str("RTC returned an invalid month"),
            Self::InvalidDay => formatter.write_str("RTC returned an invalid day"),
            Self::InvalidHour => formatter.write_str("RTC returned an invalid hour"),
            Self::InvalidMinute => formatter.write_str("RTC returned an invalid minute"),
            Self::InvalidSecond => formatter.write_str("RTC returned an invalid second"),
            Self::InvalidBinaryCodedDecimal => {
                formatter.write_str("RTC returned a value that was not a binary coded decimal")
            }
            Self::Overflow => formatter.write_str("the stored time is too large to be represented"),
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

                    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
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

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
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
