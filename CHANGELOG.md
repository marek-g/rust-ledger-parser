# Changelog

## [3.0.0] - 2019-07-08

### Added

- Support for optional `amounts` (thanks to Emmanuel Surleau).
- Support for `balance` assertion field in posting (thanks to John Beisle).

### Changed

- Updated rust_decimal crate.
- Support for `,` as thousands separator (thanks to Emmanuel Surleau).
- Support for commodities starting with a non-ASCII character (thanks to Emmanuel Surleau).
- Fixed model::Posting not displaying its status (thanks to John Beisle).

## [2.2.0] - 2019-02-17

### Added

- Display traits (thanks to Zoran Zaric).
- Parsing line comments attached to (following) transactions and postings.

### Changed

- Fixed tests by ignoring code blocks in docs (thanks to Zoran Zaric).
- Upgraded rust_decimal crate.
