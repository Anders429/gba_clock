#gba_clock

A real-time clock library for the GBA.

Provides access to the RTC for programs running on a Game Boy Advance, returning dates and times that are interoperable with the [`time`](https://crates.io/crates/time) library.

# Features
- Storing and reading of any valid time representable by the time crate (i.e. any year within the range ±9999, or ±999,999 if `time`'s `large-dates` feature is enabled).
- Works out of the box on real hardware and popular emulators (including [mGBA](https://mgba.io/)).
- Serializable with the [`serde`](https://crates.io/crates/serde) library (by enabling the `serde` feature).

# Usage
Access to the RTC is done through the [`Clock`](https://docs.rs/gba_clock/latest/gba_clock/struct.Clock.html) type. Create a `Clock` using the current time and use the returned instance to access the current time.

``` rust
use gba_clock::Clock;
use time::{
    Date,
    Month,
    PrimitiveDateTime,
    Time,
};

let current_time = PrimitiveDateTime::new(
    Date::from_calendar_date(2001, Month::March, 21).expect("invalid date"),
    Time::from_hms(11, 30, 0).expect("invalid time"),
);
let clock = Clock::new(current_time).expect("could not communicate with the RTC");

// Read the current time whenever you need.
let time = clock
    .read_datetime()
    .expect("could not read the current time");
```
