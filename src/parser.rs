use std::str::FromStr;
use nom::*;
use nom::types::CompleteStr;
use chrono::NaiveDate;
use rust_decimal::Decimal;

pub enum CustomError {
    NonExistingDate
}

fn is_digit(c: char) -> bool {
    (c >= '0' && c <= '9')
}

named_args!(numberN(n: usize)<CompleteStr, i32>,
    map_res!(take_while_m_n!(n, n, is_digit), |s: CompleteStr| { i32::from_str(s.0) })
);

named!(parse_date_internal<CompleteStr, (i32, i32, i32)>,
    do_parse!(
        year: call!(numberN,4) >>
        alt!(tag!("-") | tag!("/") | tag!(".")) >>
        month: call!(numberN,2) >>
        alt!(tag!("-") | tag!("/") | tag!(".")) >>
        day: call!(numberN,2) >>
        ((year, month, day))
    )
);

pub fn parse_date(text: CompleteStr) -> IResult<CompleteStr, NaiveDate> {
    let res = parse_date_internal(text)?;

    let rest = res.0;
    let value = res.1;

    let parsed_opt = NaiveDate::from_ymd_opt(value.0, value.1 as u32, value.2 as u32);
    if let Some(parsed) = parsed_opt {
        Ok((rest, parsed))
    } else {
        Err(Err::Error(error_position!(CompleteStr(&text.0[0..10]), ErrorKind::Custom(CustomError::NonExistingDate as u32))))
    }
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
    fn parse_quantity_test() {
        assert_eq!(Ok((CompleteStr(""), Decimal::new(202, 2))), parse_quantity(CompleteStr("2.02")));
    }
}
