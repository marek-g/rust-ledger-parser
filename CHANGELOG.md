# Changelog

## [6.0.0] - 2024-02-26

- Support for dates and metadata tags in posting comments (thanks to Tim Bates)
- Add FromStr impl for Ledger struct

## [5.1.1] - 2022-04-21

- Fix no indent if only balance (thanks to Cory Forsstrom)

## [5.1.0] - 2022-01-23

- Make SerializerSettings fields public

## [5.0.0] - 2022-01-22

- Implement posting commodity and lot prices

## [4.1.0] - 2022-01-18

### Changes

- Rewrite parser to use nom v7 (thanks to Tim Bates)
- Require hard separator between payee name and transaction inline comment (thanks to Tim Bates)

## [4.0.0] - 2022-01-04

### Added

- Support for 'include' directives (thanks to Tim Bates)
- Support for virtual postings (thanks to Tim Bates)
- Configurable end of line characters for Serializer

### Changed

- Transition to Rust 2021 edition (thanks to Tim Bates)
- Expose more accurate Ledger model (keep the order of items and expose 'include' directives)
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
