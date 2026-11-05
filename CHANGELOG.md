# Changelog

## 0.2.2 - 2026-11-05

### Compatibility

- Raise the minimum supported Rust version from 1.74 to **1.81** to implement
  `core::error::Error` without requiring `std`. The `std` feature remains for
  compatibility. Optional time providers have their own compiler requirements.

### Added

- Contextual parser expectations and error displays with borrowed source input.
- Optional `fancy-diagnostics` source-line and caret rendering without a
  diagnostic framework dependency.

### Fixed

- Resolve Chrono local date-times using the offset at the parsed date, including
  daylight-saving changes. Nonexistent local times return an error. Ambiguous
  local times choose the earliest instant unless an explicit offset selects the
  other occurrence.
- Convert Jiff microseconds to nanoseconds and return errors for out-of-range
  timestamps instead of panicking.
- Scale short fractional seconds and truncate excess digits to nanoseconds.
- Reject oversized numeric fields and over-permissive day/month names.
