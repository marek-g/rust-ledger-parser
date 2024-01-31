use crate::parser;
use crate::serializer::*;
use crate::ParseError;
use chrono::{NaiveDate, NaiveDateTime};
use nom::{error::convert_error, Finish};
use ordered_float::NotNan;
use rust_decimal::Decimal;
use std::fmt;
use std::str::FromStr;

///
/// Main document. Contains transactions and/or commodity prices.
///
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Ledger {
    pub items: Vec<LedgerItem>,
}

impl FromStr for Ledger {
    type Err = ParseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let result = parser::parse_ledger(input);
        match result.finish() {
            Ok((_, result)) => Ok(result),
            Err(error) => Err(ParseError::String(convert_error(input, error))),
        }
    }
}

impl fmt::Display for Ledger {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.to_string_pretty(&SerializerSettings::default())
        )?;
        Ok(())
    }
}

#[non_exhaustive]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum LedgerItem {
    EmptyLine,
    LineComment(String),
    Transaction(Transaction),
    CommodityPrice(CommodityPrice),
    Include(String),
}

impl fmt::Display for LedgerItem {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.to_string_pretty(&SerializerSettings::default())
        )?;
        Ok(())
    }
}

///
/// Transaction.
///
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Transaction {
    pub status: Option<TransactionStatus>,
    pub code: Option<String>,
    pub description: String,
    pub comment: Option<String>,
    pub date: NaiveDate,
    pub effective_date: Option<NaiveDate>,
    pub posting_metadata: PostingMetadata,
    pub postings: Vec<Posting>,
}

impl fmt::Display for Transaction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.to_string_pretty(&SerializerSettings::default())
        )?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TransactionStatus {
    Pending,
    Cleared,
}

impl fmt::Display for TransactionStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.to_string_pretty(&SerializerSettings::default())
        )?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Posting {
    pub account: String,
    pub reality: Reality,
    pub amount: Option<PostingAmount>,
    pub balance: Option<Balance>,
    pub status: Option<TransactionStatus>,
    pub comment: Option<String>,
    pub metadata: PostingMetadata,
}

impl fmt::Display for Posting {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.to_string_pretty(&SerializerSettings::default())
        )?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Reality {
    Real,
    BalancedVirtual,
    UnbalancedVirtual,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct PostingAmount {
    pub amount: Amount,
    pub lot_price: Option<Price>,
    pub price: Option<Price>,
}

impl fmt::Display for PostingAmount {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.to_string_pretty(&SerializerSettings::default())
        )?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Amount {
    pub quantity: Decimal,
    pub commodity: Commodity,
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.to_string_pretty(&SerializerSettings::default())
        )?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Commodity {
    pub name: String,
    pub position: CommodityPosition,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CommodityPosition {
    Left,
    Right,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Price {
    Unit(Amount),
    Total(Amount),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Balance {
    Zero,
    Amount(Amount),
}

impl fmt::Display for Balance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.to_string_pretty(&SerializerSettings::default())
        )?;
        Ok(())
    }
}

///
/// Commodity price.
///
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CommodityPrice {
    pub datetime: NaiveDateTime,
    pub commodity_name: String,
    pub amount: Amount,
}

impl fmt::Display for CommodityPrice {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.to_string_pretty(&SerializerSettings::default())
        )?;
        Ok(())
    }
}

///
/// Posting metadata. Also appears on Transaction
///
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct PostingMetadata {
    pub date: Option<NaiveDate>,
    pub effective_date: Option<NaiveDate>,
    pub tags: Vec<Tag>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tag {
    pub name: String,
    pub value: Option<TagValue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TagValue {
    String(String),
    Integer(i64),
    Float(NotNan<f64>),
    Date(NaiveDate),
}

impl fmt::Display for TagValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TagValue::String(v) => v.fmt(f),
            TagValue::Integer(v) => v.fmt(f),
            TagValue::Float(v) => v.fmt(f),
            TagValue::Date(v) => write!(f, "[{v}]"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal::Decimal;

    #[test]
    fn display_transaction_status() {
        assert_eq!(format!("{}", TransactionStatus::Pending), "!");
        assert_eq!(format!("{}", TransactionStatus::Cleared), "*");
    }

    #[test]
    fn display_amount() {
        assert_eq!(
            format!(
                "{}",
                Amount {
                    quantity: Decimal::new(4200, 2),
                    commodity: Commodity {
                        name: "€".to_owned(),
                        position: CommodityPosition::Right,
                    }
                }
            ),
            "42.00 €"
        );
        assert_eq!(
            format!(
                "{}",
                Amount {
                    quantity: Decimal::new(4200, 2),
                    commodity: Commodity {
                        name: "USD".to_owned(),
                        position: CommodityPosition::Left,
                    }
                }
            ),
            "USD42.00"
        );
    }

    #[test]
    fn display_commodity_price() {
        let actual = format!(
            "{}",
            CommodityPrice {
                datetime: NaiveDate::from_ymd_opt(2017, 11, 12)
                    .unwrap()
                    .and_hms_opt(12, 00, 00)
                    .unwrap(),
                commodity_name: "mBH".to_owned(),
                amount: Amount {
                    quantity: Decimal::new(500, 2),
                    commodity: Commodity {
                        name: "PLN".to_owned(),
                        position: CommodityPosition::Right
                    }
                }
            }
        );
        let expected = "P 2017-11-12 12:00:00 mBH 5.00 PLN";
        assert_eq!(actual, expected);
    }

    #[test]
    fn display_balance() {
        assert_eq!(
            format!(
                "{}",
                Balance::Amount(Amount {
                    quantity: Decimal::new(4200, 2),
                    commodity: Commodity {
                        name: "€".to_owned(),
                        position: CommodityPosition::Right,
                    }
                })
            ),
            "42.00 €"
        );
        assert_eq!(format!("{}", Balance::Zero), "0");
    }

    #[test]
    fn display_posting() {
        assert_eq!(
            format!(
                "{}",
                Posting {
                    account: "Assets:Checking".to_owned(),
                    reality: Reality::Real,
                    amount: Some(PostingAmount {
                        amount: Amount {
                            quantity: Decimal::new(4200, 2),
                            commodity: Commodity {
                                name: "USD".to_owned(),
                                position: CommodityPosition::Left,
                            }
                        },
                        lot_price: None,
                        price: None,
                    }),
                    balance: Some(Balance::Amount(Amount {
                        quantity: Decimal::new(5000, 2),
                        commodity: Commodity {
                            name: "USD".to_owned(),
                            position: CommodityPosition::Left,
                        }
                    })),
                    status: Some(TransactionStatus::Cleared),
                    comment: Some("asdf".to_owned()),
                    metadata: PostingMetadata {
                        date: None,
                        effective_date: None,
                        tags: vec![],
                    },
                }
            ),
            "* Assets:Checking  USD42.00 = USD50.00\n  ; asdf"
        );
    }

    #[test]
    fn display_transaction() {
        let actual = format!(
            "{}",
            Transaction {
                comment: Some("Comment Line 1\nComment Line 2".to_owned()),
                date: NaiveDate::from_ymd_opt(2018, 10, 01).unwrap(),
                effective_date: Some(NaiveDate::from_ymd_opt(2018, 10, 14).unwrap()),
                status: Some(TransactionStatus::Pending),
                code: Some("123".to_owned()),
                description: "Marek Ogarek".to_owned(),
                posting_metadata: PostingMetadata {
                    date: None,
                    effective_date: None,
                    tags: vec![],
                },
                postings: vec![
                    Posting {
                        account: "TEST:ABC 123".to_owned(),
                        reality: Reality::Real,
                        amount: Some(PostingAmount {
                            amount: Amount {
                                quantity: Decimal::new(120, 2),
                                commodity: Commodity {
                                    name: "$".to_owned(),
                                    position: CommodityPosition::Left
                                }
                            },
                            lot_price: None,
                            price: None
                        }),
                        balance: None,
                        status: None,
                        comment: Some("dd".to_owned()),
                        metadata: PostingMetadata {
                            date: None,
                            effective_date: None,
                            tags: vec![],
                        },
                    },
                    Posting {
                        account: "TEST:ABC 123".to_owned(),
                        reality: Reality::Real,
                        amount: Some(PostingAmount {
                            amount: Amount {
                                quantity: Decimal::new(120, 2),
                                commodity: Commodity {
                                    name: "$".to_owned(),
                                    position: CommodityPosition::Left
                                }
                            },
                            lot_price: None,
                            price: None
                        }),
                        balance: None,
                        status: None,
                        comment: None,
                        metadata: PostingMetadata {
                            date: None,
                            effective_date: None,
                            tags: vec![],
                        },
                    }
                ]
            },
        );
        let expected = r#"2018-10-01=2018-10-14 ! (123) Marek Ogarek
  ; Comment Line 1
  ; Comment Line 2
  TEST:ABC 123  $1.20
  ; dd
  TEST:ABC 123  $1.20"#;
        assert_eq!(actual, expected);
    }

    #[test]
    fn display_ledger() {
        let actual = format!(
            "{}",
            Ledger {
                items: vec![
                    LedgerItem::Transaction(Transaction {
                        comment: Some("Comment Line 1\nComment Line 2".to_owned()),
                        date: NaiveDate::from_ymd_opt(2018, 10, 01).unwrap(),
                        effective_date: Some(NaiveDate::from_ymd_opt(2018, 10, 14).unwrap()),
                        status: Some(TransactionStatus::Pending),
                        code: Some("123".to_owned()),
                        description: "Marek Ogarek".to_owned(),
                        posting_metadata: PostingMetadata {
                            date: None,
                            effective_date: None,
                            tags: vec![],
                        },
                        postings: vec![
                            Posting {
                                account: "TEST:ABC 123".to_owned(),
                                reality: Reality::Real,
                                amount: Some(PostingAmount {
                                    amount: Amount {
                                        quantity: Decimal::new(120, 2),
                                        commodity: Commodity {
                                            name: "$".to_owned(),
                                            position: CommodityPosition::Left
                                        }
                                    },
                                    lot_price: None,
                                    price: None
                                }),
                                balance: None,
                                status: None,
                                comment: Some("dd".to_owned()),
                                metadata: PostingMetadata {
                                    date: None,
                                    effective_date: None,
                                    tags: vec![],
                                },
                            },
                            Posting {
                                account: "TEST:ABC 123".to_owned(),
                                reality: Reality::Real,
                                amount: Some(PostingAmount {
                                    amount: Amount {
                                        quantity: Decimal::new(120, 2),
                                        commodity: Commodity {
                                            name: "$".to_owned(),
                                            position: CommodityPosition::Left
                                        }
                                    },
                                    lot_price: None,
                                    price: None
                                }),
                                balance: None,
                                status: None,
                                comment: None,
                                metadata: PostingMetadata {
                                    date: None,
                                    effective_date: None,
                                    tags: vec![],
                                },
                            }
                        ]
                    }),
                    LedgerItem::EmptyLine,
                    LedgerItem::Transaction(Transaction {
                        comment: None,
                        date: NaiveDate::from_ymd_opt(2018, 10, 01).unwrap(),
                        effective_date: Some(NaiveDate::from_ymd_opt(2018, 10, 14).unwrap()),
                        posting_metadata: PostingMetadata {
                            date: None,
                            effective_date: None,
                            tags: vec![],
                        },
                        status: Some(TransactionStatus::Pending),
                        code: Some("123".to_owned()),
                        description: "Marek Ogarek".to_owned(),
                        postings: vec![
                            Posting {
                                account: "TEST:ABC 123".to_owned(),
                                reality: Reality::Real,
                                amount: Some(PostingAmount {
                                    amount: Amount {
                                        quantity: Decimal::new(120, 2),
                                        commodity: Commodity {
                                            name: "$".to_owned(),
                                            position: CommodityPosition::Left
                                        }
                                    },
                                    lot_price: Some(Price::Unit(Amount {
                                        quantity: Decimal::new(500, 2),
                                        commodity: Commodity {
                                            name: "PLN".to_owned(),
                                            position: CommodityPosition::Right
                                        }
                                    })),
                                    price: Some(Price::Unit(Amount {
                                        quantity: Decimal::new(600, 2),
                                        commodity: Commodity {
                                            name: "PLN".to_owned(),
                                            position: CommodityPosition::Right
                                        }
                                    }))
                                }),
                                balance: None,
                                status: None,
                                comment: None,
                                metadata: PostingMetadata {
                                    date: None,
                                    effective_date: None,
                                    tags: vec![],
                                },
                            },
                            Posting {
                                account: "TEST:ABC 123".to_owned(),
                                reality: Reality::Real,
                                amount: Some(PostingAmount {
                                    amount: Amount {
                                        quantity: Decimal::new(120, 2),
                                        commodity: Commodity {
                                            name: "$".to_owned(),
                                            position: CommodityPosition::Left
                                        }
                                    },
                                    lot_price: Some(Price::Total(Amount {
                                        quantity: Decimal::new(500, 2),
                                        commodity: Commodity {
                                            name: "PLN".to_owned(),
                                            position: CommodityPosition::Right
                                        }
                                    })),
                                    price: Some(Price::Total(Amount {
                                        quantity: Decimal::new(600, 2),
                                        commodity: Commodity {
                                            name: "PLN".to_owned(),
                                            position: CommodityPosition::Right
                                        }
                                    }))
                                }),
                                balance: None,
                                status: None,
                                comment: None,
                                metadata: PostingMetadata {
                                    date: None,
                                    effective_date: None,
                                    tags: vec![],
                                },
                            }
                        ]
                    }),
                    LedgerItem::EmptyLine,
                    LedgerItem::CommodityPrice(CommodityPrice {
                        datetime: NaiveDate::from_ymd_opt(2017, 11, 12)
                            .unwrap()
                            .and_hms_opt(12, 00, 00)
                            .unwrap(),
                        commodity_name: "mBH".to_owned(),
                        amount: Amount {
                            quantity: Decimal::new(500, 2),
                            commodity: Commodity {
                                name: "PLN".to_owned(),
                                position: CommodityPosition::Right
                            }
                        }
                    }),
                ]
            }
        );
        let expected = r#"2018-10-01=2018-10-14 ! (123) Marek Ogarek
  ; Comment Line 1
  ; Comment Line 2
  TEST:ABC 123  $1.20
  ; dd
  TEST:ABC 123  $1.20

2018-10-01=2018-10-14 ! (123) Marek Ogarek
  TEST:ABC 123  $1.20 {5.00 PLN} @ 6.00 PLN
  TEST:ABC 123  $1.20 {{5.00 PLN}} @@ 6.00 PLN

P 2017-11-12 12:00:00 mBH 5.00 PLN
"#;
        assert_eq!(actual, expected);
    }
}
