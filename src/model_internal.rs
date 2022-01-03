use crate::model::*;
use std::convert::From;
use std::fmt;

///
/// Main document. Contains transactions and/or commodity prices.
///
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LedgerInternal {
    pub items: Vec<LedgerItem>,
}

impl fmt::Display for LedgerInternal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for item in &self.items {
            write!(f, "{}", item)?;
        }
        Ok(())
    }
}

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
        match self {
            LedgerItem::EmptyLine => writeln!(f)?,
            LedgerItem::LineComment(comment) => writeln!(f, "; {}", comment)?,
            LedgerItem::Transaction(transaction) => writeln!(f, "{}", transaction)?,
            LedgerItem::CommodityPrice(commodity_price) => writeln!(f, "{}", commodity_price)?,
            LedgerItem::Include(file) => writeln!(f, "include {}", file)?,
        }
        Ok(())
    }
}

impl From<LedgerInternal> for Ledger {
    fn from(value: LedgerInternal) -> Self {
        let mut transactions = Vec::<Transaction>::new();
        let mut commodity_prices = Vec::<CommodityPrice>::new();

        let mut current_comment: Option<String> = None;

        for item in value.items {
            match item {
                LedgerItem::EmptyLine => {
                    current_comment = None;
                }
                LedgerItem::LineComment(comment) => {
                    if let Some(ref mut c) = current_comment {
                        c.push('\n');
                        c.push_str(&comment);
                    } else {
                        current_comment = Some(comment);
                    }
                }
                LedgerItem::Transaction(mut transaction) => {
                    if let Some(current_comment) = current_comment {
                        let mut full_comment = current_comment;
                        if let Some(ref transaction_comment) = transaction.comment {
                            full_comment.push('\n');
                            full_comment.push_str(transaction_comment);
                        }
                        transaction.comment = Some(full_comment);
                    }
                    current_comment = None;

                    transactions.push(transaction);
                }
                LedgerItem::CommodityPrice(commodity_price) => {
                    current_comment = None;
                    commodity_prices.push(commodity_price);
                }
                LedgerItem::Include(_file) => {}
            }
        }

        Ledger {
            transactions,
            commodity_prices,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use rust_decimal::Decimal;

    #[test]
    fn display_ledger_internal() {
        let actual = format!(
            "{}",
            LedgerInternal {
                items: vec![
                    LedgerItem::Transaction(Transaction {
                        comment: Some("Comment Line 1\nComment Line 2".to_string()),
                        date: NaiveDate::from_ymd(2018, 10, 01),
                        effective_date: Some(NaiveDate::from_ymd(2018, 10, 14)),
                        status: Some(TransactionStatus::Pending),
                        code: Some("123".to_string()),
                        description: "Marek Ogarek".to_string(),
                        postings: vec![
                            Posting {
                                account: "TEST:ABC 123".to_string(),
                                reality: Reality::Real,
                                amount: Some(Amount {
                                    quantity: Decimal::new(120, 2),
                                    commodity: Commodity {
                                        name: "$".to_string(),
                                        position: CommodityPosition::Left
                                    }
                                }),
                                balance: None,
                                status: None,
                                comment: Some("dd".to_string())
                            },
                            Posting {
                                account: "TEST:ABC 123".to_string(),
                                reality: Reality::Real,
                                amount: Some(Amount {
                                    quantity: Decimal::new(120, 2),
                                    commodity: Commodity {
                                        name: "$".to_string(),
                                        position: CommodityPosition::Left
                                    }
                                }),
                                balance: None,
                                status: None,
                                comment: None
                            }
                        ]
                    }),
                    LedgerItem::EmptyLine,
                    LedgerItem::Transaction(Transaction {
                        comment: None,
                        date: NaiveDate::from_ymd(2018, 10, 01),
                        effective_date: Some(NaiveDate::from_ymd(2018, 10, 14)),
                        status: Some(TransactionStatus::Pending),
                        code: Some("123".to_string()),
                        description: "Marek Ogarek".to_string(),
                        postings: vec![
                            Posting {
                                account: "TEST:ABC 123".to_string(),
                                reality: Reality::Real,
                                amount: Some(Amount {
                                    quantity: Decimal::new(120, 2),
                                    commodity: Commodity {
                                        name: "$".to_string(),
                                        position: CommodityPosition::Left
                                    }
                                }),
                                balance: None,
                                status: None,
                                comment: None
                            },
                            Posting {
                                account: "TEST:ABC 123".to_string(),
                                reality: Reality::Real,
                                amount: Some(Amount {
                                    quantity: Decimal::new(120, 2),
                                    commodity: Commodity {
                                        name: "$".to_string(),
                                        position: CommodityPosition::Left
                                    }
                                }),
                                balance: None,
                                status: None,
                                comment: None
                            }
                        ]
                    }),
                    LedgerItem::EmptyLine,
                    LedgerItem::CommodityPrice(CommodityPrice {
                        datetime: NaiveDate::from_ymd(2017, 11, 12).and_hms(12, 00, 00),
                        commodity_name: "mBH".to_string(),
                        amount: Amount {
                            quantity: Decimal::new(500, 2),
                            commodity: Commodity {
                                name: "PLN".to_string(),
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
  TEST:ABC 123  $1.20
  TEST:ABC 123  $1.20

P 2017-11-12 12:00:00 mBH 5.00 PLN
"#;
        assert_eq!(actual, expected);
    }
}
