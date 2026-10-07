# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0] - Unreleased

### Changed
- **Breaking:** driver methods and backend traits now return `Error<I2C::Error>`:
  `I2c` preserves bus failures, `InvalidData` reports invalid input/register values,
  and `InconsistentRead` reports a counter that did not stabilize within four reads.
- Date/time and alarm setters validate inputs before bus access. Calendar setters
  require years 2000–2099. Invalid disabled alarm register fields are normalized
  to zero (or day 1 for date alarms) when read; invalid enabled fields return an error.
- **Breaking:** ported to `embedded-hal` 1.0. `RV3028<I2C>` now requires
  `I2C: embedded_hal::i2c::I2c` instead of the `embedded-hal` 0.2
  `Write + Read + WriteRead` traits. HALs that only implement `embedded-hal` 0.2
  are no longer supported.
- **Breaking:** updated `rtcc` to 0.4. The `DateTimeAccess` trait is unchanged, but it is
  a different trait from the one in `rtcc` 0.3. Import it via `rv3028c7_rtc::DateTimeAccess`
  or depend on `rtcc` 0.4.
- **Breaking:** moved to Rust edition 2024; the minimum supported Rust version is now 1.85.
- Removed the redundant error type parameter from the driver's `impl` blocks. Bus errors are
  preserved inside `Error::I2c`.
- Require `chrono` 0.4.45 and replace deprecated `chrono` APIs.
- Examples share the I2C bus with `embedded-hal-bus` instead of `shared-bus`, and use
  `linux-embedded-hal` 0.5, `ds323x` 0.7, `gpiocdev` 0.8 and `spin_sleep` 1.3.
- **Breaking:** `config_countdown_timer` takes and returns `core::time::Duration` instead of
  `chrono::Duration`, and `Duration` is no longer re-exported. Durations too long for the timer
  are clamped to its maximum instead of panicking.
- **Breaking:** `set_alarm`, `get_alarm_datetime_wday_matches` and `get_event_count_and_datetime`
  moved into the new `ChronoAccess` trait. `get_event_count_and_datetime` is no longer part of
  `EventTimeStampLogger`. Add `use rv3028c7_rtc::ChronoAccess;` to keep calling them.
- `chrono` and `rtcc` are optional dependencies, enabled by the new `chrono` feature (on by default).
  Without it, the driver depends only on `embedded-hal`.

### Added
- `check_and_clear_backup_event()` to check and clear the Backup Switch Flag (BSF).
- Optional `jiff` feature with the `JiffDateTimeAccess` trait: the clock, alarm and event timestamp
  using `jiff::civil` types. The `jiff` crate is re-exported as `rv3028c7_rtc::jiff`.
- `DateTimeParts` and `AlarmParts`, with `set_datetime_parts`, `get_event_count_and_parts`,
  `set_alarm_parts` and `get_alarm_parts`, for use without any date/time library.
- `jiff_alarm` example.
- Crate-level documentation, and docs for all public items.
- Tests that check the exact I2C traffic for setting the date and time, the alarm and the
  event timestamp, for each date/time backend.

### Fixed
- `datetime()` reads the unix time counter twice, as the application manual recommends, so a counter
  increment during the read can no longer return a torn value.
- The doc comments of `TS_EVENT_SOURCE_EVI` and `TS_EVENT_SOURCE_BSF` were attached to the wrong items.
- The `long_clock_setup` example now compiles.
- `test_get_unix_time` now tests the read-twice `get_unix_time_blocking()` it was written for.

## [0.5.2] and earlier

See the git history.
