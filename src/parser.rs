use std::str::FromStr;
use nom::*;
use chrono::NaiveDate;

pub enum CustomError {
    NonExistingDate
}

fn is_digit(c: char) -> bool {
    (c >= '0' && c <= '9')
}

named!(number4<&str, i32>,
    map_res!(take_while_m_n!(4, 4, is_digit), i32::from_str)
);

named!(number2<&str, i32>,
    map_res!(take_while_m_n!(2, 2, is_digit), i32::from_str)
);

named!(parse_date_internal<&str, (i32, i32, i32)>,
    do_parse!(
        year: number4 >>
        alt!(tag!("-") | tag!("/") | tag!(".")) >>
        month: number2 >>
        alt!(tag!("-") | tag!("/") | tag!(".")) >>
        day: number2 >>
        ((year, month, day))
    )
);

pub fn parse_date(text: &str) -> IResult<&str, NaiveDate> {
    let res = parse_date_internal(text)?;

    let rest = res.0;
    let value = res.1;

    let parsed_opt = NaiveDate::from_ymd_opt(value.0, value.1 as u32, value.2 as u32);
    if let Some(parsed) = parsed_opt {
        Ok((rest, parsed))
    } else {
        Err(Err::Error(error_position!(&text[0..10], ErrorKind::Custom(CustomError::NonExistingDate as u32))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::ErrorKind::Custom;
    use nom::Context::Code;
    use nom::Err::Error;

    #[test]
    fn parse_date_test() {
        assert_eq!(Ok(("", 2010)), number4("2010"));
        assert_eq!(Ok(("", 20)), number2("20"));
        assert_eq!(Ok(("", NaiveDate::from_ymd(2017, 03, 24))), parse_date("2017-03-24"));
        assert_eq!(Ok(("", NaiveDate::from_ymd(2017, 03, 24))), parse_date("2017/03/24"));
        assert_eq!(Ok(("", NaiveDate::from_ymd(2017, 03, 24))), parse_date("2017.03.24"));
        assert_eq!(Err(Error(Code("2017-13-24", Custom(CustomError::NonExistingDate as u32)))), parse_date("2017-13-24"));
    }
}
