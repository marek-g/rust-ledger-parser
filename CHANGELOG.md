# Changelog

## [Unreleased]

### Added

- Support for 'include' directives (thanks to Tim Bates)
- Support for virtual postings (thanks to Tim Bates)
- Expose more accurate Ledger model (keep the order of items and expose 'include' directives)
- Configurable end of line character for Serializer

### Changed

- Transition to Rust 2021 edition (thanks to Tim Bates)
- Move the simplified model and conversion code to the new 'ledger-utils' crate
- Make SerializerSettings and LedgerItem non-exhaustive
- Fix numerical comment being parsed as an amount (thanks to Tim Bates)
- Ensure date & datetime is always serialized properly

## [3.1.0] - 2020-04-30

### Added

- Serializer with configurable indent

### Changed

- Transition to Rust 2018 edition

## [3.0.0] - 2019-07-08

### Added

- Support for optional `amounts` (thanks to Emmanuel Surleau)
- Support for `balance` assertion field in posting (thanks to John Beisle)

### Changed

- Update rust_decimal crate
- Support for `,` as thousands separator (thanks to Emmanuel Surleau)
- Support for commodities starting with a non-ASCII character (thanks to Emmanuel Surleau)
- Fix model::Posting not displaying its status (thanks to John Beisle)

## [2.2.0] - 2019-02-17

### Added

- Display traits (thanks to Zoran Zaric)
- Parsing of line comments attached to (following) transactions and postings

### Changed

- Fix tests by ignoring code blocks in docs (thanks to Zoran Zaric)
- Upgrade rust_decimal crate
