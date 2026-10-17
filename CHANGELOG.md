# Changelog

All notable changes to this crate are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate follows
[Semantic Versioning](https://semver.org/) (pre-1.0: minor versions may break).

## [0.4.0] - Unreleased

API review against the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/).

### Breaking

- `Message`, `Error` and `AlarmEvent` are now `#[non_exhaustive]`. Matches on
  them need a wildcard arm, so new protocols and alarm codes can be added
  without another breaking release.
- `Login`, `Fix`, `Location`, `Status` and `Alarm` are now `#[non_exhaustive]`.
  Their fields are still public and readable, but they can no longer be
  constructed or exhaustively destructured outside the crate, so new fields can
  be added without another breaking release.

### Added

- `Hash` for `Error`, `Login`, `Status`, `StatusFlags`, `AlarmTerminalInfo`,
  `TerminalAlarm`, `AlarmEvent`, `Language`, `VoltageLevel` and `GsmSignal`.
- `Clone` for `Decoder`.
- Docs for every public item, plus examples on `Decoder`, `parse_packet`,
  `build_ack` and `Message::ack_bytes`, and an `# Errors` section on
  `parse_packet` and `Decoder::push`.

### Changed

- Removed the redundant `homepage` field from `Cargo.toml` (it duplicated
  `repository`).
