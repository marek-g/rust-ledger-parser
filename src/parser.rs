// These lints show up in nom macros
#![allow(clippy::double_comparisons)]
#![allow(clippy::manual_range_contains)]

use chrono::{NaiveDate, NaiveDateTime};
use nom::types::CompleteStr;
use nom::*;
use rust_decimal::Decimal;
use std::str::FromStr;

use crate::model::*;

enum CustomError {
    NonExistingDate,
    MoreThanOnePostingWithoutAmount,
    NoPostingWithAnAmount,
}

fn is_digit(c: char) -> bool {
    ('0'..='9').contains(&c)
}

fn is_commodity_char(c: char) -> bool {
    (c != '-') && !is_digit(c) && !is_white_char(c) && is_not_eol_or_comment_char(c)
}

fn is_white_char(c: char) -> bool {
    c == ' ' || c == '\t'
}

fn is_not_eol_char(c: char) -> bool {
    c != '\r' && c != '\n'
}

fn is_not_eol_or_comment_char(c: char) -> bool {
    c != '\r' && c != '\n' && c != ';'
}

fn join_comments(inline_comment: Option<&str>, line_comments: Vec<&str>) -> Option<String> {
    if let Some(ref inline) = inline_comment {
        if line_comments.is_empty() {
            inline_comment.map(|s| s.to_string())
        } else {
            let mut full = inline.to_string();
            full.push('\n');
            full.push_str(&line_comments.join("\n"));
            Some(full)
        }
    } else if line_comments.is_empty() {
        None
    } else {
        Some(line_comments.join("\n"))
    }
}

named!(white_spaces<CompleteStr, CompleteStr>,
    take_while1!(is_white_char)
);

named!(eol_or_eof<CompleteStr, CompleteStr>,
    alt!(eol | eof!())
);

named_args!(number_n(n: usize)<CompleteStr, i32>,
    map_res!(take_while_m_n!(n, n, is_digit), |s: CompleteStr| { i32::from_str(s.0) })
);

named!(parse_date_internal<CompleteStr, (i32, i32, i32)>,
    do_parse!(
        year: call!(number_n, 4) >>
        alt!(tag!("-") | tag!("/") | tag!(".")) >>
        month: call!(number_n, 2) >>
        alt!(tag!("-") | tag!("/") | tag!(".")) >>
        day: call!(number_n, 2) >>
        ((year, month, day))
    )
);

named!(parse_time_internal<CompleteStr, (i32, i32, i32)>,
    do_parse!(
        hour: call!(number_n, 2) >>
        tag!(":") >>
        min: call!(number_n, 2) >>
        tag!(":") >>
        sec: call!(number_n, 2) >>
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

fn parse_date(text: CompleteStr) -> IResult<CompleteStr, NaiveDate> {
    let res = parse_date_internal(text)?;

    let rest = res.0;
    let value = res.1;

    let date_opt = NaiveDate::from_ymd_opt(value.0, value.1 as u32, value.2 as u32);
    if let Some(date) = date_opt {
        Ok((rest, date))
    } else {
        Err(Err::Error(error_position!(
            CompleteStr(&text.0[0..10]),
            ErrorKind::Custom(CustomError::NonExistingDate as u32)
        )))
    }
}

fn parse_datetime(text: CompleteStr) -> IResult<CompleteStr, NaiveDateTime> {
    let res = parse_datetime_internal(text)?;

    let rest = res.0;
    let value = res.1;

    let date_opt = NaiveDate::from_ymd_opt(value.0, value.1 as u32, value.2 as u32);
    if let Some(date) = date_opt {
        let datetime_opt = date.and_hms_opt(value.3 as u32, value.4 as u32, value.5 as u32);
        if let Some(datetime) = datetime_opt {
            return Ok((rest, datetime));
        }
    }

    let len = text.len() - rest.len();
    Err(Err::Error(error_position!(
        CompleteStr(&text.0[0..len]),
        ErrorKind::Custom(CustomError::NonExistingDate as u32)
    )))
}

named!(parse_quantity<CompleteStr, Decimal>,
    map_res!(
        do_parse!(
                sign: opt!(tag!("-")) >>
                decimal: do_parse!(
                    leading: take_while_m_n!(1, 3, is_digit) >>
                    rest: alt!(
                        map!(
                            many1!(
                                preceded!(tag!(","),
                                    map!(take_while_m_n!(3, 3, is_digit), |group| group.to_string()))), |groups| groups.join("")) |
                        map!(digit0, |d| d.to_string())) >>
                    (format!("{}{}", leading, rest))
                ) >>
                fractional: opt!(recognize!(preceded!(tag!("."), digit))) >>
                (format!("{}{}{}", sign.unwrap_or(CompleteStr("")), decimal, fractional.unwrap_or(CompleteStr(""))))
        ),
        |s: String| Decimal::from_str(&s)
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
        take_while1!(is_commodity_char),
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
                quantity: if neg_opt.is_some() {
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
                quantity,
                commodity: Commodity {
                    name: commodity.to_string(),
                    position: CommodityPosition::Right
                }
            })
        )
    )
);

named!(parse_balance<CompleteStr, Balance>,
    alt!(
        do_parse!(
            amount: parse_amount >>
            (Balance::Amount(amount))
        )
        |
        do_parse!(
            tag!("0") >>
            (Balance::Zero)
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
        opt!(white_spaces) >>
        opt!(parse_inline_comment) >>
        (CommodityPrice { datetime, commodity_name: name.to_string(), amount })
    )
);

named!(parse_empty_line<CompleteStr, CompleteStr>,
    recognize!(pair!(opt!(white_spaces), peek!(eol_or_eof)))
);

named!(parse_line_comment<CompleteStr, &str>,
    do_parse!(
        opt!(white_spaces) >>
        alt!(tag!(";") | tag!("#") | tag!("%") | tag!("|") | tag!("*")) >>
        opt!(white_spaces) >>
        comment: take_while!(is_not_eol_char) >>
        (comment.0)
    )
);

named!(parse_inline_comment<CompleteStr, &str>,
    do_parse!(
        tag!(";") >>
        opt!(white_spaces) >>
        comment: take_while!(is_not_eol_char) >>
        (comment.0)
    )
);

named!(parse_include_file<CompleteStr, &str>,
    do_parse!(
        opt!(white_spaces) >>
        tag!("include") >>
        white_spaces >>
        filename: take_while!(is_not_eol_or_comment_char) >>
        (filename.0)
    )
);

fn parse_account(text: CompleteStr) -> IResult<CompleteStr, (&str, Reality)> {
    let mut second_space = false;
    for (pos, c) in text.iter_indices() {
        if c == '\t' || c == '\r' || c == '\n' || c == ';' {
            if pos > 0 {
                let (rest, found) = text.take_split(pos);
                return Ok((rest, parse_account_reality(found.0.trim_end())));
            } else {
                return Err(Err::Incomplete(Needed::Size(1)));
            }
        }

        if c == ' ' {
            if second_space {
                let (rest, found) = text.take_split(pos - 1);
                return Ok((rest, parse_account_reality(found.0)));
            } else {
                second_space = true;
            }
        } else {
            second_space = false;

            if pos == text.len() - 1 && pos > 0 {
                return Ok((CompleteStr(""), parse_account_reality(text.0)));
            }
        }
    }

    Err(Err::Incomplete(Needed::Size(1)))
}

fn parse_account_reality(name: &str) -> (&str, Reality) {
    if let Some(n1) = name.strip_prefix('[') {
        if let Some(n2) = n1.strip_suffix(']') {
            return (n2, Reality::BalancedVirtual);
        }
    }

    if let Some(n1) = name.strip_prefix('(') {
        if let Some(n2) = n1.strip_suffix(')') {
            return (n2, Reality::UnbalancedVirtual);
        }
    }

    (name, Reality::Real)
}

named!(parse_transaction_status<CompleteStr, TransactionStatus>,
    do_parse!(
        status: alt!(tag!("*") | tag!("!")) >>
        (if status == CompleteStr("*") { TransactionStatus::Cleared } else { TransactionStatus::Pending })
    )
);

named!(parse_posting<CompleteStr, Posting>,
    complete!(do_parse!(
        white_spaces >>
        status: opt!(parse_transaction_status) >>
        opt!(white_spaces) >>
        account: parse_account >>
        amount: alt!(
            do_parse!(
                white_spaces >>
                amount: parse_amount >>
                opt!(white_spaces) >>
                (Some(amount))) |
            do_parse!(
                opt!(white_spaces) >>
                (None))
        ) >>
        balance:  opt!(do_parse!(
            tag!("=") >>
            opt!(white_spaces) >>
            balance: parse_balance >>
            (balance)
        )) >>
        opt!(white_spaces) >>
        inline_comment: opt!(parse_inline_comment) >>
        line_comments: many0!(
            preceded!(opt!(eol_or_eof), parse_line_comment)
        ) >>
        (Posting {
            account: account.0.to_string(),
            reality: account.1,
            amount,
            balance,
            status,
            comment: join_comments(inline_comment, line_comments),
        })
    ))
);

fn validate_transaction(
    input: CompleteStr,
    transaction: Transaction,
) -> IResult<CompleteStr, Transaction> {
    let mut seen_empty_posting = false;
    for posting in &transaction.postings {
        if posting.amount.is_none() && posting.balance.is_none() {
            if seen_empty_posting {
                return Err(Err::Error(error_position!(
                    input,
                    ErrorKind::Custom(CustomError::MoreThanOnePostingWithoutAmount as u32)
                )));
            } else {
                seen_empty_posting = true;
            }
        }
    }
    if seen_empty_posting && transaction.postings.len() == 1 {
        return Err(Err::Error(error_position!(
            input,
            ErrorKind::Custom(CustomError::NoPostingWithAnAmount as u32)
        )));
    }
    Ok((input, transaction))
}

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
        description: map!(take_while!(is_not_eol_or_comment_char),
            |s: CompleteStr| { s.0.trim_end().to_string() }) >>
        inline_comment: opt!(parse_inline_comment) >>
        line_comments: many0!(
            preceded!(opt!(eol_or_eof), parse_line_comment)
        ) >>
        postings: many1!(
            preceded!(opt!(eol_or_eof), parse_posting)
        ) >>
        transaction: apply!(validate_transaction,
            Transaction {
                comment: join_comments(inline_comment, line_comments),
                date,
                effective_date,
                status,
                code,
                description,
                postings
            }
        ) >>
        (transaction)
    )
);

named!(parse_ledger_item<CompleteStr, LedgerItem>,
    alt!(
        map!(terminated!(parse_empty_line, eol_or_eof), |_| { LedgerItem::EmptyLine }) |
        map!(terminated!(parse_line_comment, eol_or_eof), |comment: &str| { LedgerItem::LineComment(comment.to_string()) }) |
        map!(terminated!(parse_transaction, eol_or_eof), |transaction: Transaction| { LedgerItem::Transaction(transaction) }) |
        map!(terminated!(parse_commodity_price, eol_or_eof), |cm: CommodityPrice| { LedgerItem::CommodityPrice(cm) }) |
        map!(terminated!(parse_include_file, eol_or_eof), |file: &str| { LedgerItem::Include(file.to_string()) })
    )
);

named!(pub parse_ledger<CompleteStr, Ledger>,
    do_parse!(
        items: many0!(parse_ledger_item) >>
        (Ledger { items })
    )
);

#[cfg(test)]
mod tests {
    use super::*;
    use nom::types::CompleteStr;
    use nom::Context::Code;
    use nom::Err::Error;
    use nom::ErrorKind::Custom;

    #[test]
    fn parse_date_test() {
        assert_eq!(
            parse_date(CompleteStr("2017-03-24")),
            Ok((CompleteStr(""), NaiveDate::from_ymd(2017, 03, 24)))
        );
        assert_eq!(
            parse_date(CompleteStr("2017/03/24")),
            Ok((CompleteStr(""), NaiveDate::from_ymd(2017, 03, 24)))
        );
        assert_eq!(
            parse_date(CompleteStr("2017.03.24")),
            Ok((CompleteStr(""), NaiveDate::from_ymd(2017, 03, 24)))
        );
        assert_eq!(
            parse_date(CompleteStr("2017-13-24")),
            Err(Error(Code(
                CompleteStr("2017-13-24"),
                Custom(CustomError::NonExistingDate as u32)
            )))
        );
    }

    #[test]
    fn parse_datetime_test() {
        assert_eq!(
            parse_datetime(CompleteStr("2017-03-24 17:15:23")),
            Ok((
                CompleteStr(""),
                NaiveDate::from_ymd(2017, 03, 24).and_hms(17, 15, 23)
            ))
        );
        assert_eq!(
            parse_datetime(CompleteStr("2017-13-24 22:11:22")),
            Err(Error(Code(
                CompleteStr("2017-13-24 22:11:22"),
                Custom(CustomError::NonExistingDate as u32)
            )))
        );
        assert_eq!(
            parse_datetime(CompleteStr("2017-03-24 25:11:22")),
            Err(Error(Code(
                CompleteStr("2017-03-24 25:11:22"),
                Custom(CustomError::NonExistingDate as u32)
            )))
        );
    }

    #[test]
    fn parse_quantity_test() {
        assert_eq!(
            parse_quantity(CompleteStr("1000")),
            Ok((CompleteStr(""), Decimal::new(1000, 0)))
        );
        assert_eq!(
            parse_quantity(CompleteStr("2.02")),
            Ok((CompleteStr(""), Decimal::new(202, 2)))
        );
        assert_eq!(
            parse_quantity(CompleteStr("-12.13")),
            Ok((CompleteStr(""), Decimal::new(-1213, 2)))
        );
        assert_eq!(
            parse_quantity(CompleteStr("0.1")),
            Ok((CompleteStr(""), Decimal::new(1, 1)))
        );
        assert_eq!(
            parse_quantity(CompleteStr("3")),
            Ok((CompleteStr(""), Decimal::new(3, 0)))
        );
        assert_eq!(
            parse_quantity(CompleteStr("1")),
            Ok((CompleteStr(""), Decimal::new(1, 0)))
        );
        assert_eq!(
            parse_quantity(CompleteStr("1,000")),
            Ok((CompleteStr(""), Decimal::new(1000, 0)))
        );
        assert_eq!(
            parse_quantity(CompleteStr("12,456,132.14")),
            Ok((CompleteStr(""), Decimal::new(1245613214, 2)))
        );
    }

    #[test]
    fn parse_commodity_test() {
        assert_eq!(
            parse_commodity(CompleteStr("\"ABC 123\"")),
            Ok((CompleteStr(""), "ABC 123"))
        );
        assert_eq!(
            parse_commodity(CompleteStr("ABC ")),
            Ok((CompleteStr(" "), "ABC"))
        );
        assert_eq!(
            parse_commodity(CompleteStr("$1")),
            Ok((CompleteStr("1"), "$"))
        );
        assert_eq!(
            parse_commodity(CompleteStr("€1")),
            Ok((CompleteStr("1"), "€"))
        );
        assert_eq!(
            parse_commodity(CompleteStr("€ ")),
            Ok((CompleteStr(" "), "€"))
        );
        assert_eq!(
            parse_commodity(CompleteStr("€-1")),
            Ok((CompleteStr("-1"), "€"))
        );
    }

    #[test]
    fn parse_amount_test() {
        assert_eq!(
            parse_amount(CompleteStr("$1.20")),
            Ok((
                CompleteStr(""),
                Amount {
                    quantity: Decimal::new(120, 2),
                    commodity: Commodity {
                        name: "$".to_string(),
                        position: CommodityPosition::Left
                    }
                }
            ))
        );
        assert_eq!(
            parse_amount(CompleteStr("$-1.20")),
            Ok((
                CompleteStr(""),
                Amount {
                    quantity: Decimal::new(-120, 2),
                    commodity: Commodity {
                        name: "$".to_string(),
                        position: CommodityPosition::Left
                    }
                }
            ))
        );
        assert_eq!(
            parse_amount(CompleteStr("-$1.20")),
            Ok((
                CompleteStr(""),
                Amount {
                    quantity: Decimal::new(-120, 2),
                    commodity: Commodity {
                        name: "$".to_string(),
                        position: CommodityPosition::Left
                    }
                }
            ))
        );
        assert_eq!(
            parse_amount(CompleteStr("- $ 1.20")),
            Ok((
                CompleteStr(""),
                Amount {
                    quantity: Decimal::new(-120, 2),
                    commodity: Commodity {
                        name: "$".to_string(),
                        position: CommodityPosition::Left
                    }
                }
            ))
        );
        assert_eq!(
            parse_amount(CompleteStr("1.20USD")),
            Ok((
                CompleteStr(""),
                Amount {
                    quantity: Decimal::new(120, 2),
                    commodity: Commodity {
                        name: "USD".to_string(),
                        position: CommodityPosition::Right
                    }
                }
            ))
        );
        assert_eq!(
            parse_amount(CompleteStr("-1.20 USD")),
            Ok((
                CompleteStr(""),
                Amount {
                    quantity: Decimal::new(-120, 2),
                    commodity: Commodity {
                        name: "USD".to_string(),
                        position: CommodityPosition::Right
                    }
                }
            ))
        );
    }

    #[test]
    fn parse_balance_test() {
        assert_eq!(
            parse_balance(CompleteStr("$1.20")),
            Ok((
                CompleteStr(""),
                Balance::Amount(Amount {
                    quantity: Decimal::new(120, 2),
                    commodity: Commodity {
                        name: "$".to_string(),
                        position: CommodityPosition::Left
                    }
                })
            ))
        );
        assert_eq!(
            parse_balance(CompleteStr("0 PLN")),
            Ok((
                CompleteStr(""),
                Balance::Amount(Amount {
                    quantity: Decimal::new(0, 0),
                    commodity: Commodity {
                        name: "PLN".to_string(),
                        position: CommodityPosition::Right
                    }
                })
            ))
        );
        assert_eq!(
            parse_balance(CompleteStr("0")),
            Ok((CompleteStr(""), Balance::Zero))
        );
    }

    #[test]
    fn parse_commodity_price_test() {
        assert_eq!(
            parse_commodity_price(CompleteStr("P 2017-11-12 12:00:00 mBH 5.00 PLN")),
            Ok((
                CompleteStr(""),
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
            ))
        );
    }

    #[test]
    fn parse_account_test() {
        assert_eq!(
            parse_account(CompleteStr("TEST:ABC 123  ")),
            Ok((CompleteStr("  "), ("TEST:ABC 123", Reality::Real)))
        );
        assert_eq!(
            parse_account(CompleteStr("TEST:ABC 123\t")),
            Ok((CompleteStr("\t"), ("TEST:ABC 123", Reality::Real)))
        );
        assert_eq!(
            parse_account(CompleteStr("TEST:ABC 123")),
            Ok((CompleteStr(""), ("TEST:ABC 123", Reality::Real)))
        );
        assert_eq!(
            parse_account(CompleteStr("[TEST:ABC 123]")),
            Ok((CompleteStr(""), ("TEST:ABC 123", Reality::BalancedVirtual)))
        );
        assert_eq!(
            parse_account(CompleteStr("(TEST:ABC 123)")),
            Ok((
                CompleteStr(""),
                ("TEST:ABC 123", Reality::UnbalancedVirtual)
            ))
        );
    }

    #[test]
    fn parse_transaction_status_test() {
        assert_eq!(
            parse_transaction_status(CompleteStr("!")),
            Ok((CompleteStr(""), TransactionStatus::Pending))
        );
        assert_eq!(
            parse_transaction_status(CompleteStr("*")),
            Ok((CompleteStr(""), TransactionStatus::Cleared))
        );
    }

    #[test]
    fn parse_posting_test() {
        assert_eq!(
            parse_posting(CompleteStr(" TEST:ABC 123  $1.20")),
            Ok((
                CompleteStr(""),
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
                    comment: None,
                }
            ))
        );
        assert_eq!(
            parse_posting(CompleteStr(" ! TEST:ABC 123  $1.20;test\n;comment line 2")),
            Ok((
                CompleteStr(""),
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
                    status: Some(TransactionStatus::Pending),
                    comment: Some("test\ncomment line 2".to_string())
                }
            ))
        );
        assert_eq!(
            parse_posting(CompleteStr(" ! TEST:ABC 123;test\n;comment line 2")),
            Ok((
                CompleteStr(""),
                Posting {
                    account: "TEST:ABC 123".to_string(),
                    reality: Reality::Real,
                    amount: None,
                    balance: None,
                    status: Some(TransactionStatus::Pending),
                    comment: Some("test\ncomment line 2".to_string())
                }
            ))
        );
        assert_eq!(
            parse_posting(CompleteStr(" ! TEST:ABC 123 ;test\n;comment line 2")),
            Ok((
                CompleteStr(""),
                Posting {
                    account: "TEST:ABC 123".to_string(),
                    reality: Reality::Real,
                    amount: None,
                    balance: None,
                    status: Some(TransactionStatus::Pending),
                    comment: Some("test\ncomment line 2".to_string())
                }
            ))
        );
        assert_eq!(
            parse_posting(CompleteStr(" TEST:ABC 123  $1.20 = $2.40 ;comment")),
            Ok((
                CompleteStr(""),
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
                    balance: Some(Balance::Amount(Amount {
                        quantity: Decimal::new(240, 2),
                        commodity: Commodity {
                            name: "$".to_string(),
                            position: CommodityPosition::Left
                        }
                    })),
                    status: None,
                    comment: Some("comment".to_string())
                }
            ))
        );
        assert_eq!(
            parse_posting(CompleteStr(" TEST:ABC 123")),
            Ok((
                CompleteStr(""),
                Posting {
                    account: "TEST:ABC 123".to_string(),
                    reality: Reality::Real,
                    amount: None,
                    balance: None,
                    status: None,
                    comment: None
                }
            ))
        );
        assert_eq!(
            parse_posting(CompleteStr(" TEST:ABC 123   ; 456")),
            Ok((
                CompleteStr(""),
                Posting {
                    account: "TEST:ABC 123".to_string(),
                    reality: Reality::Real,
                    amount: None,
                    balance: None,
                    status: None,
                    comment: Some("456".to_string()),
                }
            ))
        );
    }

    #[test]
    fn parse_transaction_test() {
        assert_eq!(
            parse_transaction(CompleteStr(
                r#"2018-10-01=2018-10-14 ! (123) Marek Ogarek
 TEST:ABC 123  $1.20 ; test
 TEST:ABC 123  $1.20"#
            )),
            Ok((
                CompleteStr(""),
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
                            comment: Some("test".to_string()),
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
                            comment: None,
                        }
                    ]
                }
            ))
        );
        assert_eq!(
            parse_transaction(CompleteStr(
                r#"2018-10-01=2018-10-14 ! (123) Marek Ogarek
 TEST:ABC 123  $1.20 ; test
 TEST:DEF 123  EUR-1.20
 TEST:GHI 123
 TEST:JKL 123  EUR-2.00"#
            )),
            Ok((
                CompleteStr(""),
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
                            comment: Some("test".to_string()),
                        },
                        Posting {
                            balance: None,
                            account: "TEST:DEF 123".to_string(),
                            reality: Reality::Real,
                            amount: Some(Amount {
                                quantity: Decimal::new(-120, 2),
                                commodity: Commodity {
                                    name: "EUR".to_string(),
                                    position: CommodityPosition::Left
                                }
                            }),
                            status: None,
                            comment: None,
                        },
                        Posting {
                            account: "TEST:GHI 123".to_string(),
                            reality: Reality::Real,
                            amount: None,
                            balance: None,
                            status: None,
                            comment: None,
                        },
                        Posting {
                            account: "TEST:JKL 123".to_string(),
                            reality: Reality::Real,
                            amount: Some(Amount {
                                quantity: Decimal::new(-200, 2),
                                commodity: Commodity {
                                    name: "EUR".to_string(),
                                    position: CommodityPosition::Left
                                }
                            }),
                            balance: None,
                            status: None,
                            comment: None,
                        },
                    ]
                }
            ))
        );
        assert_eq!(
            parse_transaction(CompleteStr(
                r#"2018-10-01=2018-10-14 ! (123) Marek Ogarek
 TEST:ABC 123  $1.20 ; test
 TEST:DEF 123
 TEST:GHI 123
 TEST:JKL 123  EUR-2.00"#
            )),
            Err(Error(Code(
                CompleteStr(""),
                ErrorKind::Custom(CustomError::MoreThanOnePostingWithoutAmount as u32)
            )))
        );
        assert_eq!(
            parse_transaction(CompleteStr(
                r#"2018-10-01=2018-10-14 ! (123) Marek Ogarek
 TEST:ABC 123   ; test"#
            )),
            Err(Error(Code(
                CompleteStr(""),
                ErrorKind::Custom(CustomError::NoPostingWithAnAmount as u32)
            )))
        );
    }

    #[test]
    fn parse_include_test() {
        assert_eq!(
            parse_include_file(CompleteStr(r#"include other_file.ledger"#)),
            Ok((CompleteStr(""), "other_file.ledger"))
        );
    }

    #[test]
    fn parse_ledger_test() {
        let res = parse_ledger(CompleteStr(
            r#"; Example 1

include other_file.ledger

P 2017-11-12 12:00:00 mBH 5.00 PLN

; Comment
2018-10-01=2018-10-14 ! (123) Marek Ogarek
 TEST:ABC 123  $1.20
 TEST:ABC 123  $1.20

2018-10-01=2018-10-14 ! (123) Marek Ogarek
 TEST:ABC 123  $1.20
 TEST:ABC 123  $1.20
"#,
        ))
        .unwrap()
        .1;
        assert_eq!(res.items.len(), 10);
        assert!(matches!(res.items[0], LedgerItem::LineComment(_)));
        assert!(matches!(res.items[1], LedgerItem::EmptyLine));
        assert!(matches!(res.items[2], LedgerItem::Include(_)));
        assert!(matches!(res.items[3], LedgerItem::EmptyLine));
        assert!(matches!(res.items[4], LedgerItem::CommodityPrice(_)));
        assert!(matches!(res.items[5], LedgerItem::EmptyLine));
        assert!(matches!(res.items[6], LedgerItem::LineComment(_)));
        assert!(matches!(res.items[7], LedgerItem::Transaction(_)));
        assert!(matches!(res.items[8], LedgerItem::EmptyLine));
        assert!(matches!(res.items[9], LedgerItem::Transaction(_)));
    }
}
