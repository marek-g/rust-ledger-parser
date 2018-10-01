use std::str::FromStr;
use nom::*;
use nom::types::CompleteStr;
use chrono::{ NaiveDate, NaiveDateTime };
use rust_decimal::Decimal;

#[derive(Debug,PartialEq,Eq)]
pub enum CommodityPosition {
    Left,
    Right
}

#[derive(Debug,PartialEq,Eq)]
pub struct Commodity {
    pub name: String,
    pub position: CommodityPosition
}

#[derive(Debug,PartialEq,Eq)]
pub struct Amount {
    pub quantity: Decimal,
    pub commodity: Commodity,
}

#[derive(Debug,PartialEq,Eq)]
pub struct CommodityPrice {
    pub datetime: NaiveDateTime,
    pub commodity_name: String,
    pub amount: Amount,
}

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
    (c == ' ' || c == '\t' || c == '\r' || c == '\n')
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
    ws!(
        alt!(
            do_parse!(
                neg_opt: opt!(tag!("-")) >>
                commodity: parse_commodity >>
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

named!(parse_line_comment<CompleteStr, CompleteStr>,
    recognize!(tuple!(
        alt!(tag!(";") | tag!("#") | tag!("%") | tag!("|") | tag!("*")),
        take_while!(is_not_eol_char),
        eol_or_eof
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
        assert_eq!(Ok((CompleteStr(""), NaiveDate::from_ymd(2017, 03, 24))), parse_date(CompleteStr("2017-03-24")));
        assert_eq!(Ok((CompleteStr(""), NaiveDate::from_ymd(2017, 03, 24))), parse_date(CompleteStr("2017/03/24")));
        assert_eq!(Ok((CompleteStr(""), NaiveDate::from_ymd(2017, 03, 24))), parse_date(CompleteStr("2017.03.24")));
        assert_eq!(Err(Error(Code(CompleteStr("2017-13-24"), Custom(CustomError::NonExistingDate as u32)))), parse_date(CompleteStr("2017-13-24")));
    }

    #[test]
    fn parse_datetime_test() {
        assert_eq!(Ok((CompleteStr(""), NaiveDate::from_ymd(2017, 03, 24).and_hms(17, 15, 23))), parse_datetime(CompleteStr("2017-03-24 17:15:23")));
        assert_eq!(Err(Error(Code(CompleteStr("2017-13-24 22:11:22"), Custom(CustomError::NonExistingDate as u32)))), parse_datetime(CompleteStr("2017-13-24 22:11:22")));
        assert_eq!(Err(Error(Code(CompleteStr("2017-03-24 25:11:22"), Custom(CustomError::NonExistingDate as u32)))), parse_datetime(CompleteStr("2017-03-24 25:11:22")));
    }

    #[test]
    fn parse_quantity_test() {
        assert_eq!(Ok((CompleteStr(""), Decimal::new(202, 2))), parse_quantity(CompleteStr("2.02")));
        assert_eq!(Ok((CompleteStr(""), Decimal::new(-1213, 2))), parse_quantity(CompleteStr("-12.13")));
        assert_eq!(Ok((CompleteStr(""), Decimal::new(1, 1))), parse_quantity(CompleteStr("0.1")));
        assert_eq!(Ok((CompleteStr(""), Decimal::new(3, 0))), parse_quantity(CompleteStr("3")));
    }

    #[test]
    fn parse_commodity_test() {
        assert_eq!(Ok((CompleteStr(""), "ABC 123")), parse_commodity(CompleteStr("\"ABC 123\"")));
        assert_eq!(Ok((CompleteStr(" "), "ABC")), parse_commodity(CompleteStr("ABC ")));
        assert_eq!(Ok((CompleteStr("1"), "$")), parse_commodity(CompleteStr("$1")));
    }

    #[test]
    fn parse_amount_test() {
        assert_eq!(Ok((CompleteStr(""), Amount { quantity: Decimal::new(120, 2), commodity: Commodity { name: "$".to_string(), position: CommodityPosition::Left }})), parse_amount(CompleteStr("$1.20")));
        assert_eq!(Ok((CompleteStr(""), Amount { quantity: Decimal::new(-120, 2), commodity: Commodity { name: "$".to_string(), position: CommodityPosition::Left }})), parse_amount(CompleteStr("$-1.20")));
        assert_eq!(Ok((CompleteStr(""), Amount { quantity: Decimal::new(-120, 2), commodity: Commodity { name: "$".to_string(), position: CommodityPosition::Left }})), parse_amount(CompleteStr("-$1.20")));
        assert_eq!(Ok((CompleteStr(""), Amount { quantity: Decimal::new(-120, 2), commodity: Commodity { name: "$".to_string(), position: CommodityPosition::Left }})), parse_amount(CompleteStr("- $ 1.20")));
        assert_eq!(Ok((CompleteStr(""), Amount { quantity: Decimal::new(120, 2), commodity: Commodity { name: "USD".to_string(), position: CommodityPosition::Right }})), parse_amount(CompleteStr("1.20USD")));
        assert_eq!(Ok((CompleteStr(""), Amount { quantity: Decimal::new(-120, 2), commodity: Commodity { name: "USD".to_string(), position: CommodityPosition::Right }})), parse_amount(CompleteStr("-1.20 USD")));
    }

    #[test]
    fn parse_commodity_price_test() {
        assert_eq!(Ok((CompleteStr(""),
            CommodityPrice {
                datetime: NaiveDate::from_ymd(2017, 11, 12).and_hms(12, 00, 00),
                commodity_name: "mBH".to_string(),
                amount: Amount { quantity: Decimal::new(500, 2), commodity: Commodity { name: "PLN".to_string(), position: CommodityPosition::Right }} })),
            parse_commodity_price(CompleteStr("P 2017-11-12 12:00:00 mBH 5.00 PLN\r\n")));
    }
}
