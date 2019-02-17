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
//!     ACCOUNT  AMOUNT [; NOTE]
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

mod model;
mod model_internal;
mod parser;

pub use model::*;

/// Parses ledger-cli source.
///
/// # Examples
///
/// let result = ledger_parser::parse(r#"; Example 1
/// 2018-10-01=2018-10-14 ! (123) Description
///   ; Transaction comment
///   TEST:Account 123  $1.20
///   ; Posting comment
///   TEST:Account 345  -$1.20"#);
/// ```
pub fn parse(input: &str) -> Result<Ledger, String> {
    use nom::types::CompleteStr;

    let result = parser::parse_ledger_items(CompleteStr(input));
    match result {
        Ok((CompleteStr(""), result)) => Ok(model_internal::convert_items_to_ledger(result)),
        Ok((rest, _)) => Err(rest.0.to_string()),
        Err(error) => Err(format!("{:?}", error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal::Decimal;

    #[test]
    fn parse_ledger_test() {
        assert_eq!(
            parse(
                r#"; Example 1

P 2017-11-12 12:00:00 mBH 5.00 PLN

; Comment Line 1
; Comment Line 2
2018-10-01=2018-10-14 ! (123) Marek Ogarek ; Comment Line 3
; Comment Line 4
; Comment Line 5
 TEST:ABC 123  $1.20; Posting comment line 1
 ; Posting comment line 2
 TEST:ABC 123  $1.20

2018-10-01=2018-10-14 ! (123) Marek Ogarek
 TEST:ABC 123  $1.20
 TEST:ABC 123  $1.20
"#
            ),
            Ok(Ledger {
                transactions: vec![
                    Transaction {
                        comment: Some("Comment Line 1\nComment Line 2\nComment Line 3\nComment Line 4\nComment Line 5".to_string()),
                        date: NaiveDate::from_ymd(2018, 10, 01),
                        effective_date: Some(NaiveDate::from_ymd(2018, 10, 14)),
                        status: Some(TransactionStatus::Pending),
                        code: Some("123".to_string()),
                        description: "Marek Ogarek".to_string(),
                        postings: vec![
                            Posting {
                                account: "TEST:ABC 123".to_string(),
                                amount: Amount {
                                    quantity: Decimal::new(120, 2),
                                    commodity: Commodity {
                                        name: "$".to_string(),
                                        position: CommodityPosition::Left
                                    }
                                },
                                status: None,
                                comment: Some("Posting comment line 1\nPosting comment line 2".to_string())
                            },
                            Posting {
                                account: "TEST:ABC 123".to_string(),
                                amount: Amount {
                                    quantity: Decimal::new(120, 2),
                                    commodity: Commodity {
                                        name: "$".to_string(),
                                        position: CommodityPosition::Left
                                    }
                                },
                                status: None,
                                comment: None
                            }
                        ]
                    },
                    Transaction {
                        comment: None,
                        date: NaiveDate::from_ymd(2018, 10, 01),
                        effective_date: Some(NaiveDate::from_ymd(2018, 10, 14)),
                        status: Some(TransactionStatus::Pending),
                        code: Some("123".to_string()),
                        description: "Marek Ogarek".to_string(),
                        postings: vec![
                            Posting {
                                account: "TEST:ABC 123".to_string(),
                                amount: Amount {
                                    quantity: Decimal::new(120, 2),
                                    commodity: Commodity {
                                        name: "$".to_string(),
                                        position: CommodityPosition::Left
                                    }
                                },
                                status: None,
                                comment: None
                            },
                            Posting {
                                account: "TEST:ABC 123".to_string(),
                                amount: Amount {
                                    quantity: Decimal::new(120, 2),
                                    commodity: Commodity {
                                        name: "$".to_string(),
                                        position: CommodityPosition::Left
                                    }
                                },
                                status: None,
                                comment: None
                            }
                        ]
                    }
                ],
                commodity_prices: vec![CommodityPrice {
                    datetime: NaiveDate::from_ymd(2017, 11, 12).and_hms(12, 00, 00),
                    commodity_name: "mBH".to_string(),
                    amount: Amount {
                        quantity: Decimal::new(500, 2),
                        commodity: Commodity {
                            name: "PLN".to_string(),
                            position: CommodityPosition::Right
                        }
                    }
                }]
            })
        );
    }

    #[test]
    fn parse_ledger_err_test() {
        assert_eq!(parse("wrong input"), Err("wrong input".to_string()));
    }
}
