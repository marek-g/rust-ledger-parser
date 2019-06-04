use chrono::{NaiveDate, NaiveDateTime};
use rust_decimal::Decimal;
use std::fmt;

///
/// Main document. Contains transactions and/or commodity prices.
///
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Ledger {
    pub transactions: Vec<Transaction>,
    pub commodity_prices: Vec<CommodityPrice>,
}

impl fmt::Display for Ledger {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for transaction in &self.transactions {
            write!(f, "{}\n", transaction)?;
        }

        for commodity_price in &self.commodity_prices {
            write!(f, "{}", commodity_price)?;
        }
        Ok(())
    }
}

///
/// Transaction.
///
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Transaction {
    pub comment: Option<String>,
    pub date: NaiveDate,
    pub effective_date: Option<NaiveDate>,
    pub status: Option<TransactionStatus>,
    pub code: Option<String>,
    pub description: String,
    pub postings: Vec<Posting>,
}

impl fmt::Display for Transaction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.date)?;

        if let Some(effective_date) = self.effective_date {
            write!(f, "={}", effective_date)?;
        }

        if let Some(ref status) = self.status {
            write!(f, " {}", status)?;
        }

        if let Some(ref code) = self.code {
            write!(f, " ({})", code)?;
        }

        if !self.description.is_empty() {
            write!(f, " {}", self.description)?;
        }

        if let Some(ref comment) = self.comment {
            for comment in comment.split("\n") {
                write!(f, "\n  ; {}", comment)?;
            }
        }

        write!(f, "\n")?;

        for posting in &self.postings {
            write!(f, "  {}\n", posting)?;
        }

        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TransactionStatus {
    Pending,
    Cleared,
}

impl fmt::Display for TransactionStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TransactionStatus::Pending => write!(f, "!"),
            TransactionStatus::Cleared => write!(f, "*"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Posting {
    pub account: String,
    pub amount: Option<Amount>,
    pub status: Option<TransactionStatus>,
    pub comment: Option<String>,
}

impl fmt::Display for Posting {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.account)?;
        if let Some(ref amount) = self.amount {
            write!(f, "  {}", amount)?;
        }

        if let Some(ref comment) = self.comment {
            for comment in comment.split("\n") {
                write!(f, "\n  ; {}", comment)?;
            }
        }

        Ok(())
    }
}

#[derive(PartialEq, Eq, Clone)]
pub struct Amount {
    pub quantity: Decimal,
    pub commodity: Commodity,
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self.commodity.position {
            CommodityPosition::Left => write!(f, "{}{}", self.commodity.name, self.quantity),
            CommodityPosition::Right => write!(f, "{} {}", self.quantity, self.commodity.name),
        }
    }
}

impl fmt::Debug for Amount {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        fmt::Display::fmt(self, f)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Commodity {
    pub name: String,
    pub position: CommodityPosition,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum CommodityPosition {
    Left,
    Right,
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
            "P {} {} {}",
            self.datetime, self.commodity_name, self.amount
        )
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
                        name: "€".to_string(),
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
                        name: "USD".to_string(),
                        position: CommodityPosition::Left,
                    }
                }
            ),
            "USD42.00"
        );
    }

    #[test]
    fn display_posting() {
        assert_eq!(
            format!(
                "{}",
                Posting {
                    account: "Assets:Checking".to_string(),
                    amount: Some(Amount {
                        quantity: Decimal::new(4200, 2),
                        commodity: Commodity {
                            name: "USD".to_string(),
                            position: CommodityPosition::Left,
                        }
                    }),
                    status: None,
                    comment: Some("asdf".to_string()),
                }
            ),
            "Assets:Checking  USD42.00\n  ; asdf"
        );
    }

    #[test]
    fn display_transaction() {
        let actual = format!(
            "{}",
            Transaction {
                comment: Some("Comment Line 1\nComment Line 2".to_string()),
                date: NaiveDate::from_ymd(2018, 10, 01),
                effective_date: Some(NaiveDate::from_ymd(2018, 10, 14)),
                status: Some(TransactionStatus::Pending),
                code: Some("123".to_string()),
                description: "Marek Ogarek".to_string(),
                postings: vec![
                    Posting {
                        account: "TEST:ABC 123".to_string(),
                        amount: Some(Amount {
                            quantity: Decimal::new(120, 2),
                            commodity: Commodity {
                                name: "$".to_string(),
                                position: CommodityPosition::Left
                            }
                        }),
                        status: None,
                        comment: Some("dd".to_string())
                    },
                    Posting {
                        account: "TEST:ABC 123".to_string(),
                        amount: Some(Amount {
                            quantity: Decimal::new(120, 2),
                            commodity: Commodity {
                                name: "$".to_string(),
                                position: CommodityPosition::Left
                            }
                        }),
                        status: None,
                        comment: None
                    }
                ]
            },
        );
        let expected = r#"2018-10-01=2018-10-14 ! (123) Marek Ogarek
  ; Comment Line 1
  ; Comment Line 2
  TEST:ABC 123  $1.20
  ; dd
  TEST:ABC 123  $1.20
"#;
        assert_eq!(actual, expected);
    }
    #[test]
    fn display_commodity_price() {
        let actual = format!(
            "{}",
            CommodityPrice {
                datetime: NaiveDate::from_ymd(2017, 11, 12).and_hms(12, 00, 00),
                commodity_name: "mBH".to_string(),
                amount: Amount {
                    quantity: Decimal::new(500, 2),
                    commodity: Commodity {
                        name: "PLN".to_string(),
                        position: CommodityPosition::Right
                    }
                }
            }
        );
        let expected = "P 2017-11-12 12:00:00 mBH 5.00 PLN";
        assert_eq!(actual, expected);
    }

    #[test]
    fn display_ledger() {
        let actual = format!(
            "{}",
            Ledger {
                transactions: vec![
                    Transaction {
                        comment: Some("Comment Line 1\nComment Line 2".to_string()),
                        date: NaiveDate::from_ymd(2018, 10, 01),
                        effective_date: Some(NaiveDate::from_ymd(2018, 10, 14)),
                        status: Some(TransactionStatus::Pending),
                        code: Some("123".to_string()),
                        description: "Marek Ogarek".to_string(),
                        postings: vec![
                            Posting {
                                account: "TEST:ABC 123".to_string(),
                                amount: Some(Amount {
                                    quantity: Decimal::new(120, 2),
                                    commodity: Commodity {
                                        name: "$".to_string(),
                                        position: CommodityPosition::Left
                                    }
                                }),
                                status: None,
                                comment: Some("dd".to_string())
                            },
                            Posting {
                                account: "TEST:ABC 123".to_string(),
                                amount: Some(Amount {
                                    quantity: Decimal::new(120, 2),
                                    commodity: Commodity {
                                        name: "$".to_string(),
                                        position: CommodityPosition::Left
                                    }
                                }),
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
                                amount: Some(Amount {
                                    quantity: Decimal::new(120, 2),
                                    commodity: Commodity {
                                        name: "$".to_string(),
                                        position: CommodityPosition::Left
                                    }
                                }),
                                status: None,
                                comment: None
                            },
                            Posting {
                                account: "TEST:ABC 123".to_string(),
                                amount: Some(Amount {
                                    quantity: Decimal::new(120, 2),
                                    commodity: Commodity {
                                        name: "$".to_string(),
                                        position: CommodityPosition::Left
                                    }
                                }),
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

P 2017-11-12 12:00:00 mBH 5.00 PLN"#;
        assert_eq!(actual, expected);
    }
}
