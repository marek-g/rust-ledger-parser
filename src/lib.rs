//! Rust library for parsing [Ledger-cli](https://www.ledger-cli.org/) input files.
//!
//! Only a subset of the ledger-cli's file format is implemented.
//!
//! Supported elements:
//!
//! - Line comments (starting with: ``; # % | *``)
//!
//! - Inline comments (starting with ``;``)
//!
//! - Transaction headers with format:
//!
//!   ```ledger-cli,ignore
//!   DATE[=EDATE] [*|!] [(CODE)] DESC
//!   ```
//!
//! - Transaction postings with format (minimum two spaces or one tab between ``ACCOUNT`` and ``AMOUNT``):
//!
//!   ```ledger-cli,ignore
//!     ACCOUNT  [AMOUNT] [= BALANCE] [; NOTE]
//!   ```
//!
//!     - Virtual accounts are supported
//!
//! - Commodity prices with format:
//!
//!   ```ledger-cli,ignore
//!   P DATE SYMBOL PRICE
//!   ```
//! - Command directives: `include`

mod model;
pub use model::*;

mod serializer;
pub use serializer::*;

mod parser;

use nom::{error::convert_error, Finish};
use std::fmt;

#[derive(Debug)]
pub enum ParseError {
    String(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ParseError::String(ref err) => err.fmt(f),
        }
    }
}

impl std::error::Error for ParseError {
    fn description(&self) -> &str {
        match *self {
            ParseError::String(ref err) => err,
        }
    }
}

/// Parses ledger-cli source to AST tree.
///
/// # Examples
///
/// ```
/// let result = ledger_parser::parse(r#"; Example 1
/// 2018-10-01=2018-10-14 ! (123) Description
///   ; Transaction comment
///   TEST:Account 123  $1.20
///   ; Posting comment
///   TEST:Account 345  -$1.20"#);
/// ```
pub fn parse(input: &str) -> Result<Ledger, ParseError> {
    let result = parser::parse_ledger(input);
    match result.finish() {
        Ok((_, result)) => Ok(result),
        Err(error) => Err(ParseError::String(convert_error(input, error))),
    }
}
