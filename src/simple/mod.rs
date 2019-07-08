mod model;
pub use self::model::*;

mod from_ast;
pub use self::from_ast::*;

use ast;
use std::convert::TryInto;

#[derive(Debug)]
pub enum Error {
    ParseError(ast::ParseError),
    ConvertError(ConvertError),
}

impl From<ast::ParseError> for Error {
    fn from(value: ast::ParseError) -> Self {
        Error::ParseError(value)
    }
}

impl From<ConvertError> for Error {
    fn from(value: ConvertError) -> Self {
        Error::ConvertError(value)
    }
}

/// Parses ledger-cli source to simplified AST tree (with some preprocessed calculations).
///
/// # Examples
///
/// ```rust,ignore
/// let result = ledger_parser::simple::parse(r#"; Example 1
/// 2018-10-01=2018-10-14 ! (123) Description
///   ; Transaction comment
///   TEST:Account 123  $1.20
///   ; Posting comment
///   TEST:Account 345  -$1.20"#);
/// ```
pub fn parse(input: &str) -> Result<Ledger, Error> {
    Ok(ast::parse(input)?.try_into()?)
}
