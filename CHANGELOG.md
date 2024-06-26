# Changelog

## 0.4.0 - 2024-06-17
### Added
- `Clock::read_date()` method for reading the current date by itself.
- `Clock::write_date()` method for changing only the date.
- `Error::NotEnabled` error variant to indicate the GPIO communication with the RTC is not properly enabled.
### Fixed
- All interactions with the RTC now check that the GPIO port is enabled, fixing issues with disabled reads being interpreted as correct values.

## 0.3.1 - 2023-12-11
### Fixed
- Deserializing a `Clock` now checks for valid status and test mode, returning an error if the RTC is in an unusable state.

## 0.3.0 - 2023-12-06
### Changed
- `Error` now includes invalid values for `InvalidStatus`, `InvalidMonth`, `InvalidDay`, `InvalidHour`, `InvalidMinute`, `InvalidSecond`, and `InvalidBinaryCodedDecimal` variants.
### Fixed
- Deserializing a `Clock` now correctly enables the RTC.

## 0.2.0 - 2023-10-11
### Added
- Implemented `Display` for `Error`.
- `Clock` methods for reading and writing time by itself. 

## 0.1.0 - 2023-08-27
### Added
- `Clock` struct to store the current time's offset from the RTC and read the current date and time.
- `Error` enum to represent potential errors.
- `serde` `Serialize` and `Deserialize` implementations.
