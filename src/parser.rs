use std::str::FromStr;
use nom::*;
use nom::types::CompleteStr;
use chrono::{ NaiveDate, NaiveDateTime };
use rust_decimal::Decimal;

use model::*;
use model_internal::*;

pub enum CustomError {
    NonExistingDate
}

fn is_digit(c: char) -> bool {
    (c >= '0' && c <= '9')
}

fn is_commodity_first_char(c: char) -> bool {
    (c >= 'a' && c <= 'z' || c >= 'A' && c <= 'Z' || c == '$' || c > 0x7F as char)
}

fn is_commodity_char(c: char) -> bool {
    (c >= 'a' && c <= 'z' || c >= 'A' && c <= 'Z' || c == '$' || c > 0x7F as char)
}

fn is_white_char(c: char) -> bool {
    (c == ' ' || c == '\t')
}

fn is_not_eol_char(c: char) -> bool {
    (c != '\r' && c != '\n')
}

named!(white_spaces<CompleteStr, CompleteStr>,
    take_while1!(is_white_char)
);

named!(eol_or_eof<CompleteStr, CompleteStr>,
    alt!(eol | eof!())
);

named_args!(numberN(n: usize)<CompleteStr, i32>,
    map_res!(take_while_m_n!(n, n, is_digit), |s: CompleteStr| { i32::from_str(s.0) })
);

named!(parse_date_internal<CompleteStr, (i32, i32, i32)>,
    do_parse!(
        year: call!(numberN, 4) >>
        alt!(tag!("-") | tag!("/") | tag!(".")) >>
        month: call!(numberN, 2) >>
        alt!(tag!("-") | tag!("/") | tag!(".")) >>
        day: call!(numberN, 2) >>
        ((year, month, day))
    )
);

named!(parse_time_internal<CompleteStr, (i32, i32, i32)>,
    do_parse!(
        hour: call!(numberN, 2) >>
        tag!(":") >>
        min: call!(numberN, 2) >>
        tag!(":") >>
        sec: call!(numberN, 2) >>
        ((hour, min, sec))
    )
);

named!(parse_datetime_internal<CompleteStr, (i32, i32, i32, i32, i32, i32)>,
    do_parse!(
        date: parse_date_internal >>
        white_spaces >>
        time: parse_time_internal >>
        ((date.0, date.1, date.2, time.0, time.1, time.2))
    )
);

pub fn parse_date(text: CompleteStr) -> IResult<CompleteStr, NaiveDate> {
    let res = parse_date_internal(text)?;

    let rest = res.0;
    let value = res.1;

    let date_opt = NaiveDate::from_ymd_opt(value.0, value.1 as u32, value.2 as u32);
    if let Some(date) = date_opt {
        Ok((rest, date))
    } else {
        Err(Err::Error(error_position!(CompleteStr(&text.0[0..10]), ErrorKind::Custom(CustomError::NonExistingDate as u32))))
    }
}

pub fn parse_datetime(text: CompleteStr) -> IResult<CompleteStr, NaiveDateTime> {
    let res = parse_datetime_internal(text)?;

    let rest = res.0;
    let value = res.1;

    let date_opt = NaiveDate::from_ymd_opt(value.0, value.1 as u32, value.2 as u32);
    if let Some(date) = date_opt {
        let datetime_opt = date.and_hms_opt(value.3 as u32, value.4 as u32, value.5 as u32);
        if let Some(datetime) = datetime_opt {
            return Ok((rest, datetime))
        }
    }

    let len = text.len() - rest.len();
    Err(Err::Error(error_position!(CompleteStr(&text.0[0..len]), ErrorKind::Custom(CustomError::NonExistingDate as u32))))
}

named!(parse_quantity<CompleteStr, Decimal>,
    map_res!(
        recognize!(
            tuple!(
                opt!(tag!("-")),
                digit,
                opt!(tuple!(tag!("."), digit))
            )
        ),
        |s: CompleteStr| { Decimal::from_str(s.0) }
    )
);

named!(string_between_quotes<CompleteStr, &str>,
    map!(
        delimited!(char!('\"'), is_not!("\""), char!('\"')),
        |s: CompleteStr| { s.0 }
    )
);

named!(commodity_without_quotes<CompleteStr, &str>,
    map!(
        recognize!(
            tuple!(
                take_while_m_n!(1, 1, is_commodity_first_char),
                take_while!(is_commodity_char)
            )
        ),
        |s: CompleteStr| { s.0 }
    )
);

named!(parse_commodity<CompleteStr, &str>,
    alt!(string_between_quotes | commodity_without_quotes)
);

named!(parse_amount<CompleteStr, Amount>,
    alt!(
        do_parse!(
            neg_opt: opt!(tag!("-")) >>
            opt!(white_spaces) >>
            commodity: parse_commodity >>
            opt!(white_spaces) >>
            quantity: parse_quantity >>
            (Amount {
                quantity: if let Some(_) = neg_opt {
                    quantity * Decimal::new(-1, 0)
                } else { quantity },
                commodity: Commodity {
                    name: commodity.to_string(),
                    position: CommodityPosition::Left
                }
            })
        )
        |
        do_parse!(
            quantity: parse_quantity >>
            opt!(white_spaces) >>
            commodity: parse_commodity >>
            (Amount {
                quantity: quantity,
                commodity: Commodity {
                    name: commodity.to_string(),
                    position: CommodityPosition::Right
                }
            })
        )
    )
);

named!(parse_commodity_price<CompleteStr, CommodityPrice>,
    do_parse!(
        tag!("P") >>
        white_spaces >>
        datetime: parse_datetime >>
        white_spaces >>
        name: parse_commodity >>
        white_spaces >>
        amount: parse_amount >>
        eol_or_eof >>
        (CommodityPrice { datetime: datetime, commodity_name: name.to_string(), amount: amount })
    )
);

named!(parse_empty_line<CompleteStr, CompleteStr>,
    alt!(eol | recognize!(pair!(white_spaces, eol_or_eof)))
);

named!(parse_line_comment<CompleteStr, &str>,
    do_parse!(
        alt!(tag!(";") | tag!("#") | tag!("%") | tag!("|") | tag!("*")) >>
        opt!(white_spaces) >>
        comment: take_while!(is_not_eol_char) >>
        eol_or_eof >>
        (comment.0)
    )
);

pub fn parse_account(text: CompleteStr) -> IResult<CompleteStr, &str> {
    let mut second_space = false;
    for ind in text.iter_indices() {
        let (pos, c) = ind;

        if c == '\t' || c == '\r' || c == '\n' {
            if pos > 0 {
                let (rest, found) = text.take_split(pos);
                return Ok((rest, found.0))
            } else {
                return Err(Err::Incomplete(Needed::Size(1)))
            }
        }
        
        if c == ' ' {
            if second_space {
                let (rest, found) = text.take_split(pos - 1);
                return Ok((rest, found.0))
            } else {
                second_space = true;
            }
        } else {
            second_space = false;

             if pos == text.len() - 1 && pos > 0 {
                 return Ok((CompleteStr(""), text.0))
             }
        }
    }

    Err(Err::Incomplete(Needed::Size(1)))
}

named!(parse_transaction_status<CompleteStr, TransactionStatus>,
    do_parse!(
        status: alt!(tag!("*") | tag!("!")) >>
        (if status == CompleteStr("*") { TransactionStatus::Cleared } else { TransactionStatus::Pending })
    )
);

named!(parse_posting<CompleteStr, Posting>,
    do_parse!(
        white_spaces >>
        status: opt!(parse_transaction_status) >>
        opt!(white_spaces) >>
        account: parse_account >>
        white_spaces >>
        amount: parse_amount >>
        opt!(white_spaces) >>
        eol_or_eof >>
        (Posting { account: account.to_string(), amount: amount, status: status })
    )
);

named!(parse_transaction<CompleteStr, Transaction>,
    do_parse!(
        date: parse_date >>
        effective_date: opt!(do_parse!(
            tag!("=") >>
            edate: parse_date >>
            (edate)
        )) >>
        white_spaces >>
        status: opt!(parse_transaction_status) >>
        opt!(white_spaces) >>
        code: opt!(map!(delimited!(char!('('), is_not!(")"), char!(')')),
            |s: CompleteStr| { s.0.to_string() })) >>
        opt!(white_spaces) >>
        description: map!(take_while!(is_not_eol_char),
            |s: CompleteStr| { s.0.to_string() }) >>
        eol_or_eof >>
        postings: many1!(parse_posting) >>
        (Transaction {
            comment: None,
            date: date,
            effective_date: effective_date,
            status: status,
            code: code,
            description: description,
            postings: postings
        })
    )
);

named!(pub parse_ledger_items<CompleteStr, Vec<LedgerItem>>,
    many0!(alt!(
        map!(parse_empty_line, |_| { LedgerItem::EmptyLine }) |
        map!(parse_line_comment, |comment: &str| { LedgerItem::LineComment(comment.to_string()) }) |
        map!(parse_transaction, |transaction: Transaction| { LedgerItem::Transaction(transaction) }) |
        map!(parse_commodity_price, |cm: CommodityPrice| { LedgerItem::CommodityPrice(cm) })
    ))
);


#[cfg(test)]
mod tests {
    use super::*;
    use nom::ErrorKind::Custom;
    use nom::Context::Code;
    use nom::Err::Error;
    use nom::types::CompleteStr;

    #[test]
    fn parse_date_test() {
        assert_eq!(parse_date(CompleteStr("2017-03-24")), Ok((CompleteStr(""), NaiveDate::from_ymd(2017, 03, 24))));
        assert_eq!(parse_date(CompleteStr("2017/03/24")), Ok((CompleteStr(""), NaiveDate::from_ymd(2017, 03, 24))));
        assert_eq!(parse_date(CompleteStr("2017.03.24")), Ok((CompleteStr(""), NaiveDate::from_ymd(2017, 03, 24))));
        assert_eq!(parse_date(CompleteStr("2017-13-24")), Err(Error(Code(CompleteStr("2017-13-24"), Custom(CustomError::NonExistingDate as u32)))));
    }

    #[test]
    fn parse_datetime_test() {
        assert_eq!(parse_datetime(CompleteStr("2017-03-24 17:15:23")), Ok((CompleteStr(""), NaiveDate::from_ymd(2017, 03, 24).and_hms(17, 15, 23))));
        assert_eq!(parse_datetime(CompleteStr("2017-13-24 22:11:22")), Err(Error(Code(CompleteStr("2017-13-24 22:11:22"), Custom(CustomError::NonExistingDate as u32)))));
        assert_eq!(parse_datetime(CompleteStr("2017-03-24 25:11:22")), Err(Error(Code(CompleteStr("2017-03-24 25:11:22"), Custom(CustomError::NonExistingDate as u32)))));
    }

    #[test]
    fn parse_quantity_test() {
        assert_eq!(parse_quantity(CompleteStr("2.02")), Ok((CompleteStr(""), Decimal::new(202, 2))));
        assert_eq!(parse_quantity(CompleteStr("-12.13")), Ok((CompleteStr(""), Decimal::new(-1213, 2))));
        assert_eq!(parse_quantity(CompleteStr("0.1")), Ok((CompleteStr(""), Decimal::new(1, 1))));
        assert_eq!(parse_quantity(CompleteStr("3")), Ok((CompleteStr(""), Decimal::new(3, 0))));
    }

    #[test]
    fn parse_commodity_test() {
        assert_eq!(parse_commodity(CompleteStr("\"ABC 123\"")), Ok((CompleteStr(""), "ABC 123")));
        assert_eq!(parse_commodity(CompleteStr("ABC ")), Ok((CompleteStr(" "), "ABC")));
        assert_eq!(parse_commodity(CompleteStr("$1")), Ok((CompleteStr("1"), "$")));
    }

    #[test]
    fn parse_amount_test() {
        assert_eq!(parse_amount(CompleteStr("$1.20")), Ok((CompleteStr(""), Amount { quantity: Decimal::new(120, 2), commodity: Commodity { name: "$".to_string(), position: CommodityPosition::Left }})));
        assert_eq!(parse_amount(CompleteStr("$-1.20")), Ok((CompleteStr(""), Amount { quantity: Decimal::new(-120, 2), commodity: Commodity { name: "$".to_string(), position: CommodityPosition::Left }})));
        assert_eq!(parse_amount(CompleteStr("-$1.20")), Ok((CompleteStr(""), Amount { quantity: Decimal::new(-120, 2), commodity: Commodity { name: "$".to_string(), position: CommodityPosition::Left }})));
        assert_eq!(parse_amount(CompleteStr("- $ 1.20")), Ok((CompleteStr(""), Amount { quantity: Decimal::new(-120, 2), commodity: Commodity { name: "$".to_string(), position: CommodityPosition::Left }})));
        assert_eq!(parse_amount(CompleteStr("1.20USD")), Ok((CompleteStr(""), Amount { quantity: Decimal::new(120, 2), commodity: Commodity { name: "USD".to_string(), position: CommodityPosition::Right }})));
        assert_eq!(parse_amount(CompleteStr("-1.20 USD")), Ok((CompleteStr(""), Amount { quantity: Decimal::new(-120, 2), commodity: Commodity { name: "USD".to_string(), position: CommodityPosition::Right }})));
    }

    #[test]
    fn parse_commodity_price_test() {
        assert_eq!(parse_commodity_price(CompleteStr("P 2017-11-12 12:00:00 mBH 5.00 PLN\r\n")),
            Ok((CompleteStr(""),
                CommodityPrice {
                    datetime: NaiveDate::from_ymd(2017, 11, 12).and_hms(12, 00, 00),
                    commodity_name: "mBH".to_string(),
                    amount: Amount { quantity: Decimal::new(500, 2), commodity: Commodity { name: "PLN".to_string(), position: CommodityPosition::Right }}
                }
            ))
        );
    }

    #[test]
    fn parse_account_test() {
        assert_eq!(parse_account(CompleteStr("TEST:ABC 123  ")), Ok((CompleteStr("  "), "TEST:ABC 123")));
        assert_eq!(parse_account(CompleteStr("TEST:ABC 123\t")), Ok((CompleteStr("\t"), "TEST:ABC 123")));
        assert_eq!(parse_account(CompleteStr("TEST:ABC 123")), Ok((CompleteStr(""), "TEST:ABC 123")));
    }

    #[test]
    fn parse_transaction_status_test() {
        assert_eq!(parse_transaction_status(CompleteStr("!")), Ok((CompleteStr(""), TransactionStatus::Pending)));
        assert_eq!(parse_transaction_status(CompleteStr("*")), Ok((CompleteStr(""), TransactionStatus::Cleared)));
    }

    #[test]
    fn parse_posting_test() {
        assert_eq!(parse_posting(CompleteStr(" TEST:ABC 123  $1.20\n")),
            Ok((CompleteStr(""),
                Posting {
                    account: "TEST:ABC 123".to_string(),
                    amount: Amount { quantity: Decimal::new(120, 2), commodity: Commodity { name: "$".to_string(), position: CommodityPosition::Left }},
                    status: None
                }
            ))
        );
        assert_eq!(parse_posting(CompleteStr(" ! TEST:ABC 123  $1.20")),
            Ok((CompleteStr(""),
                Posting {
                    account: "TEST:ABC 123".to_string(),
                    amount: Amount { quantity: Decimal::new(120, 2), commodity: Commodity { name: "$".to_string(), position: CommodityPosition::Left }},
                    status: Some(TransactionStatus::Pending)
                }
            ))
        );
    }

    #[test]
    fn parse_transaction_test() {
        assert_eq!(parse_transaction(CompleteStr(r#"2018-10-01=2018-10-14 ! (123) Marek Ogarek
 TEST:ABC 123  $1.20
 TEST:ABC 123  $1.20"#)),
            Ok((CompleteStr(""),
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
                            amount: Amount { quantity: Decimal::new(120, 2), commodity: Commodity { name: "$".to_string(), position: CommodityPosition::Left }},
                            status: None
                        },
                        Posting {
                            account: "TEST:ABC 123".to_string(),
                            amount: Amount { quantity: Decimal::new(120, 2), commodity: Commodity { name: "$".to_string(), position: CommodityPosition::Left }},
                            status: None
                        }
                    ]
                }
            ))
        );
    }

    #[test]
    fn parse_ledger_items_test() {
        let res = parse_ledger_items(CompleteStr(r#"; Example 1

P 2017-11-12 12:00:00 mBH 5.00 PLN

; Comment
2018-10-01=2018-10-14 ! (123) Marek Ogarek
 TEST:ABC 123  $1.20
 TEST:ABC 123  $1.20

2018-10-01=2018-10-14 ! (123) Marek Ogarek
 TEST:ABC 123  $1.20
 TEST:ABC 123  $1.20
"#)).unwrap().1;
        assert_eq!(res.len(), 8);
        assert!(match res[0] { LedgerItem::LineComment(_) => true, _ => false } );
        assert!(match res[1] { LedgerItem::EmptyLine => true, _ => false } );
        assert!(match res[2] { LedgerItem::CommodityPrice(_) => true, _ => false } );
        assert!(match res[3] { LedgerItem::EmptyLine => true, _ => false } );
        assert!(match res[4] { LedgerItem::LineComment(_) => true, _ => false } );
        assert!(match res[5] { LedgerItem::Transaction(_) => true, _ => false } );
        assert!(match res[6] { LedgerItem::EmptyLine => true, _ => false } );
        assert!(match res[7] { LedgerItem::Transaction(_) => true, _ => false } );
    }
}
