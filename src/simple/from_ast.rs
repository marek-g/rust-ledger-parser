use ast;
use common;
use simple;
use std::convert::TryFrom;
use std::convert::TryInto;

#[derive(Debug)]
pub enum ConvertError {}

impl TryFrom<ast::Ledger> for simple::Ledger {
    type Error = ConvertError;

    fn try_from(value: ast::Ledger) -> Result<Self, Self::Error> {
        let mut transactions = Vec::<simple::Transaction>::new();
        let mut commodity_prices = Vec::<common::CommodityPrice>::new();

        let mut current_comment: Option<String> = None;

        for item in value.items {
            match item {
                ast::LedgerItem::EmptyLine => {
                    current_comment = None;
                }
                ast::LedgerItem::LineComment(comment) => {
                    if let Some(ref mut c) = current_comment {
                        c.push_str("\n");
                        c.push_str(&comment);
                    } else {
                        current_comment = Some(comment);
                    }
                }
                ast::LedgerItem::Transaction(mut transaction) => {
                    if let Some(current_comment) = current_comment {
                        let mut full_comment = current_comment;
                        if let Some(ref transaction_comment) = transaction.comment {
                            full_comment.push_str("\n");
                            full_comment.push_str(&transaction_comment);
                        }
                        transaction.comment = Some(full_comment);
                    }
                    current_comment = None;

                    transactions.push(transaction.try_into()?);
                }
                ast::LedgerItem::CommodityPrice(commodity_price) => {
                    current_comment = None;
                    commodity_prices.push(commodity_price);
                }
            }
        }

        Ok(simple::Ledger {
            transactions: transactions,
            commodity_prices: commodity_prices,
        })
    }
}

impl TryFrom<ast::Transaction> for simple::Transaction {
    type Error = ConvertError;

    fn try_from(value: ast::Transaction) -> Result<Self, Self::Error> {
        Ok(simple::Transaction {
            comment: value.comment,
            date: value.date,
            effective_date: value.effective_date,
            status: value.status,
            code: value.code,
            description: value.description,
            postings: value
                .postings
                .into_iter()
                .map(|p| simple::Posting {
                    account: p.account,
                    amount: p.amount.unwrap(),
                    status: p.status,
                    comment: p.comment,
                })
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use common::*;
    use rust_decimal::Decimal;
    use simple::*;
    use std::convert::TryInto;

    #[test]
    fn from_ast_test() {
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
 TEST:ABC 123  $1.20 = $2.40

2018-10-01=2018-10-14 ! (123) Marek Ogarek
 TEST:ABC 123  $1.20
 TEST:ABC 123  $1.20
"#
            ).unwrap().try_into(),
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
}
