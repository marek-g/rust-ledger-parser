mod model;
pub use self::model::*;

mod parser;
use self::parser::parse_ledger;

use ast;

#[derive(Debug)]
pub enum ParseError {
    String(String),
}

/// Parses ledger-cli source to AST tree.
///
/// # Examples
///
/// ```rust,ignore
/// let result = ledger_parser::ast::parse(r#"; Example 1
/// 2018-10-01=2018-10-14 ! (123) Description
///   ; Transaction comment
///   TEST:Account 123  $1.20
///   ; Posting comment
///   TEST:Account 345  -$1.20"#);
/// ```
pub fn parse(input: &str) -> Result<ast::Ledger, ParseError> {
    use nom::types::CompleteStr;

    let result = parse_ledger(CompleteStr(input));
    match result {
        Ok((CompleteStr(""), result)) => Ok(result),
        Ok((rest, _)) => Err(ParseError::String(rest.0.to_string())),
        Err(error) => Err(ParseError::String(format!("{:?}", error))),
    }
}
