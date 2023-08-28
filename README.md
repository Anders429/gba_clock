# gba_clock

[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/Anders429/gba_clock/ci.yml?branch=master)](https://github.com/Anders429/gba_clock/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/gba_clock)](https://crates.io/crates/gba_clock)
[![docs.rs](https://docs.rs/gba_clock/badge.svg)](https://docs.rs/gba_clock)
[![License](https://img.shields.io/crates/l/gba_clock)](#license)

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

## License
This project is licensed under either of

* Apache License, Version 2.0
([LICENSE-APACHE](https://github.com/Anders429/gba_clock/blob/HEAD/LICENSE-APACHE) or
http://www.apache.org/licenses/LICENSE-2.0)
* MIT license
([LICENSE-MIT](https://github.com/Anders429/gba_clock/blob/HEAD/LICENSE-MIT) or
http://opensource.org/licenses/MIT)

at your option.

### Contribution
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
