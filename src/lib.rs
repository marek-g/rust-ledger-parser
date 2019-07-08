//! Rust library for parsing [Ledger-cli](https://www.ledger-cli.org/) input files.
//!
//! Only a subset of the ledger-cli's file format is implemented.
//!
//! Supported elements:
//!
//! * Line comments (starting with: ``; # % | *``)
//!
//! * Inline comments (starting with ``;``)
//!
//! * Transaction headers with format:
//!
//!   ```ignore
//!   DATE[=EDATE] [*|!] [(CODE)] DESC
//!   ```
//!
//! * Transaction postings with format (minimum two spaces or one tab between ``ACCOUNT`` and ``AMOUNT``):
//!
//!   ```ignore
//!     ACCOUNT  AMOUNT [= BALANCE] [; NOTE]
//!   ```
//!
//!   Note that the ``AMOUNT`` field is always required.
//!
//! * Commodity prices with format:
//!
//!   ```ignore
//!   P DATE SYMBOL PRICE
//!   ```

extern crate chrono;
extern crate nom;
extern crate rust_decimal;

pub mod ast;
pub mod common;
pub mod simple;
