# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0] - Unreleased

### Changed
- **Breaking:** ported to `embedded-hal` 1.0. `RV3028<I2C>` now requires
  `I2C: embedded_hal::i2c::I2c` instead of the `embedded-hal` 0.2
  `Write + Read + WriteRead` traits. HALs that only implement `embedded-hal` 0.2
  are no longer supported.
- **Breaking:** updated `rtcc` to 0.4. The `DateTimeAccess` trait is unchanged, but it is
  a different trait from the one in `rtcc` 0.3. Import it via `rv3028c7_rtc::DateTimeAccess`
  or depend on `rtcc` 0.4.
- **Breaking:** moved to Rust edition 2024; the minimum supported Rust version is now 1.85.
- Removed the redundant error type parameter from the driver's `impl` blocks. Errors are
  `I2C::Error`, which is the same type as before.
- Require `chrono` 0.4.45 and replace deprecated `chrono` APIs.
- Examples share the I2C bus with `embedded-hal-bus` instead of `shared-bus`, and use
  `linux-embedded-hal` 0.5, `ds323x` 0.7, `gpiocdev` 0.8 and `spin_sleep` 1.3.

### Added
- `check_and_clear_backup_event()` to check and clear the Backup Switch Flag (BSF).

### Fixed
- The `long_clock_setup` example now compiles.
- `test_get_unix_time` now tests the read-twice `get_unix_time_blocking()` it was written for.

## [0.5.2] and earlier

See the git history.
