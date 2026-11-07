use chrono::{NaiveDate, NaiveDateTime};
use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_while1, take_while_m_n},
    character::complete::{char, digit0, digit1, line_ending, not_line_ending, space0, space1},
    combinator::{eof, map, map_opt, map_res, not, opt, peek, recognize, value, verify},
    error::VerboseError,
    multi::{fold_many0, fold_many1, many0, many1, separated_list1},
    number::complete::double,
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    AsChar, Err, IResult, Needed, Parser,
};
use ordered_float::NotNan;
use rust_decimal::Decimal;
use std::str::FromStr;

use crate::model::*;

type LedgerParseResult<'a, T> = IResult<&'a str, T, VerboseError<&'a str>>;

fn is_commodity_char(c: char) -> bool {
    !"0123456789{}[]()~`!@#%^&*-=+\\'\",./? ;\t\r\n".contains(c)
}

fn eol_or_eof(input: &str) -> LedgerParseResult<'_, &str> {
    alt((line_ending, eof))(input)
}

fn number_n<'a>(n: usize) -> impl FnMut(&'a str) -> IResult<&'a str, i32, VerboseError<&str>> {
    map_res(take_while_m_n(n, n, AsChar::is_dec_digit), i32::from_str)
}

fn parse_date_internal(input: &str) -> LedgerParseResult<'_, (i32, i32, i32)> {
    tuple((
        terminated(number_n(4), alt((char('-'), char('/'), char('.')))),
        terminated(number_n(2), alt((char('-'), char('/'), char('.')))),
        number_n(2),
    ))(input)
}

fn parse_time_internal(input: &str) -> LedgerParseResult<'_, (i32, i32, i32)> {
    tuple((
        terminated(number_n(2), char(':')),
        terminated(number_n(2), char(':')),
        number_n(2),
    ))(input)
}

fn parse_datetime_internal(input: &str) -> LedgerParseResult<'_, (i32, i32, i32, i32, i32, i32)> {
    separated_pair(parse_date_internal, space1, parse_time_internal)
        .map(|(date, time)| (date.0, date.1, date.2, time.0, time.1, time.2))
        .parse(input)
}

fn parse_date(input: &str) -> LedgerParseResult<'_, NaiveDate> {
    map_opt(parse_date_internal, |value| {
        NaiveDate::from_ymd_opt(value.0, value.1 as u32, value.2 as u32)
    })(input)
}

fn parse_datetime(input: &str) -> LedgerParseResult<'_, NaiveDateTime> {
    map_opt(
        parse_datetime_internal,
        |value| match NaiveDate::from_ymd_opt(value.0, value.1 as u32, value.2 as u32) {
            Some(date) => date.and_hms_opt(value.3 as u32, value.4 as u32, value.5 as u32),
            None => None,
        },
    )(input)
}

fn parse_quantity(input: &str) -> LedgerParseResult<'_, Decimal> {
    map_res(
        tuple((
            opt(tag("-")),
            alt((
                pair(
                    take_while_m_n(1, 3, AsChar::is_dec_digit),
                    many1(preceded(
                        char(','),
                        take_while_m_n(3, 3, AsChar::is_dec_digit).map(str::to_owned),
                    )),
                )
                .map(|(leading, rest)| format!("{}{}", leading, rest.join(""))),
                digit0.map(str::to_owned),
            )),
            opt(recognize(preceded(char('.'), digit1))),
        ))
        .map(|(sign, decimal, fractional)| {
            format!(
                "{}{}{}",
                sign.unwrap_or(""),
                decimal,
                fractional.unwrap_or("")
            )
        }),
        |s: String| Decimal::from_str(&s),
    )(input)
}

fn string_fragment(input: &str) -> LedgerParseResult<'_, &str> {
    alt((
        verify(is_not("\\\""), |s: &str| !s.is_empty()),
        value("\"", tag("\\\"")),
    ))(input)
}

fn string_between_quotes(input: &str) -> LedgerParseResult<'_, String> {
    let string_contents = fold_many1(string_fragment, String::new, |mut string, fragment| {
        string.push_str(fragment);
        string
    });

    delimited(char('"'), string_contents, char('"'))(input)
}

fn commodity_without_quotes(input: &str) -> LedgerParseResult<'_, String> {
    take_while1(is_commodity_char)
        .map(str::to_owned)
        .parse(input)
}

fn parse_commodity(input: &str) -> LedgerParseResult<'_, String> {
    alt((string_between_quotes, commodity_without_quotes))(input)
}

fn parse_amount(input: &str) -> LedgerParseResult<'_, Amount> {
    alt((
        tuple((
            opt(terminated(char('-'), space0)),
            terminated(parse_commodity, space0),
            parse_quantity,
        ))
        .map(|(neg_opt, name, quantity)| Amount {
            quantity: if neg_opt.is_some() {
                quantity * Decimal::new(-1, 0)
            } else {
                quantity
            },
            commodity: Commodity {
                name,
                position: CommodityPosition::Left,
            },
        }),
        pair(terminated(parse_quantity, space0), parse_commodity).map(|(quantity, name)| Amount {
            quantity,
            commodity: Commodity {
                name,
                position: CommodityPosition::Right,
            },
        }),
    ))(input)
}

fn parse_posting_amount(input: &str) -> LedgerParseResult<'_, PostingAmount> {
    let (input, amount) = parse_amount(input)?;
    let (input, lot_price) = opt(preceded(space0, parse_lot_price))(input)?;
    let (input, price) = opt(preceded(space0, parse_price))(input)?;
    Ok((
        input,
        PostingAmount {
            amount,
            lot_price,
            price,
        },
    ))
}

fn parse_lot_price(input: &str) -> LedgerParseResult<'_, Price> {
    alt((
        delimited(
            pair(tag("{{"), space0),
            parse_amount,
            pair(space0, tag("}}")),
        )
        .map(Price::Total),
        delimited(
            pair(char('{'), space0),
            parse_amount,
            pair(space0, char('}')),
        )
        .map(Price::Unit),
    ))(input)
}

fn parse_price(input: &str) -> LedgerParseResult<'_, Price> {
    alt((
        preceded(pair(tag("@@"), space0), parse_amount).map(Price::Total),
        preceded(pair(char('@'), space0), parse_amount).map(Price::Unit),
    ))(input)
}

fn parse_balance(input: &str) -> LedgerParseResult<'_, Balance> {
    alt((
        parse_amount.map(Balance::Amount),
        value(Balance::Zero, char('0')),
    ))(input)
}

fn parse_commodity_price(input: &str) -> LedgerParseResult<'_, CommodityPrice> {
    let (input, _) = char('P')(input)?;
    let (input, datetime) = preceded(space1, parse_datetime)(input)?;
    let (input, commodity_name) = preceded(space1, parse_commodity)(input)?;
    let (input, amount) = preceded(space1, parse_amount)(input)?;
    let (input, _) = preceded(space0, opt(preceded(char(';'), not_line_ending)))(input)?;
    let (input, _) = eol_or_eof(input)?;

    Ok((
        input,
        CommodityPrice {
            datetime,
            commodity_name,
            amount,
        },
    ))
}

fn parse_empty_line(input: &str) -> LedgerParseResult<'_, &str> {
    alt((
        terminated(space0, line_ending),
        terminated(space1, eof), // Must consume something or many0 errors to prevent infinite loop
    ))(input)
}

fn parse_global_line_comment(input: &str) -> LedgerParseResult<'_, &str> {
    let (input, _) = delimited(
        space0,
        alt((char(';'), char('#'), char('%'), char('|'), char('*'))),
        space0,
    )(input)?;
    terminated(not_line_ending.map(str::trim_end), eol_or_eof)(input)
}

#[derive(Default)]
struct Metadata {
    comment: Option<String>,
    date: Option<NaiveDate>,
    effective_date: Option<NaiveDate>,
    tags: Vec<Tag>,
}

fn parse_metadata_date(input: &str) -> LedgerParseResult<'_, Metadata> {
    map(
        preceded(
            space0,
            delimited(
                char('['),
                pair(opt(parse_date), opt(preceded(char('='), parse_date))),
                char(']'),
            ),
        ),
        |(date, effective_date)| Metadata {
            date,
            effective_date,
            ..Default::default()
        },
    )(input)
}

///
/// Characters a whitespace delimited token, i.e. a comment word or a tag name, is made of.
/// Ledger-cli splits comments into tokens separated by spaces or tabs, so anything but these
/// and line endings is allowed.
///
fn is_token_char(c: char) -> bool {
    !matches!(c, ' ' | '\t' | '\r' | '\n')
}

///
/// Ledger-cli accepts any character in a tag name, as a name is simply everything it finds
/// before the first space or tab. Only the characters delimiting names are excluded: token
/// characters and `:`, which separates tags from each other and a tag from its value.
///
fn is_tag_name_char(c: char) -> bool {
    is_token_char(c) && c != ':'
}

fn parse_tag_name(input: &str) -> LedgerParseResult<'_, &str> {
    take_while1(is_tag_name_char)(input)
}

fn parse_token(input: &str) -> LedgerParseResult<'_, &str> {
    take_while1(is_token_char)(input)
}

///
/// Skips whitespace and any additional `;` characters preceding a tag. Ledger-cli strips them
/// before looking for the tag name, so comments written as `;;Tag: value` do not produce
/// a tag named `;Tag`.
///
fn parse_tag_prefix(input: &str) -> LedgerParseResult<'_, ()> {
    value((), many0(alt((space1, tag(";")))))(input)
}

fn parse_tag_value(input: &str) -> LedgerParseResult<'_, TagValue> {
    alt((
        map_res(
            recognize(pair(opt(char('-')), terminated(digit1, not(char('.'))))),
            str::parse,
        )
        .map(TagValue::Integer),
        map_res(double, NotNan::new).map(TagValue::Float),
        delimited(char('['), parse_date, char(']')).map(TagValue::Date),
    ))(input)
}

fn parse_metadata_tag_with_value(input: &str) -> LedgerParseResult<'_, Metadata> {
    map(
        preceded(
            parse_tag_prefix,
            alt((
                pair(
                    terminated(parse_tag_name, pair(char(':'), space1)),
                    not_line_ending.map(str::trim_end),
                )
                .map(|(name, value)| (name, TagValue::String(value.to_owned()))),
                pair(
                    terminated(parse_tag_name, pair(tag("::"), space1)),
                    parse_tag_value,
                ),
            )),
        ),
        |(name, value)| Metadata {
            tags: vec![Tag {
                name: name.to_owned(),
                value: Some(value),
            }],
            ..Default::default()
        },
    )(input)
}

///
/// Parses a `:TAG1:TAG2:` series of tags. Ledger-cli looks for tags in whole comment tokens
/// only (it splits comments into tokens separated by spaces or tabs), so a series has to be
/// followed by whitespace or the end of the line and text such as `:tag:extra` stays
/// a part of the comment.
///
fn parse_tags(input: &str) -> LedgerParseResult<'_, Vec<Tag>> {
    terminated(
        delimited(
            char(':'),
            separated_list1(
                char(':'),
                parse_tag_name
                    .map(str::to_owned)
                    .map(|name| Tag { name, value: None }),
            ),
            char(':'),
        ),
        peek(alt((space1, eol_or_eof))),
    )(input)
}

///
/// Parses a comment, which may contain a `:TAG1:TAG2:` series of tags. It is scanned token by
/// token, because ledger-cli separates tags from the comment text using spaces and tabs, so
/// text like `a:b:` or `http://example.com/tag: value` is not mistaken for tags.
///
fn parse_comment_with_tags(input: &str) -> LedgerParseResult<'_, Metadata> {
    map(
        preceded(
            space0,
            tuple((
                opt(verify(
                    recognize(many0(pair(not(parse_tags).and(parse_token), space0))),
                    |s: &str| !s.is_empty(),
                )
                .map(str::trim_end)),
                opt(parse_tags),
                opt(verify(not_line_ending.map(str::trim), |s: &str| {
                    !s.is_empty()
                })),
            )),
        ),
        |(s1, v, s2)| Metadata {
            comment: s1
                .into_iter()
                .chain(s2)
                .map(str::to_owned)
                .reduce(|mut a, b| {
                    a.push(' ');
                    a.push_str(&b);
                    a
                }),
            tags: v.unwrap_or_default(),
            ..Default::default()
        },
    )(input)
}

fn parse_metadata_comments(input: &str) -> LedgerParseResult<'_, Metadata> {
    terminated(
        fold_many0(
            preceded(
                many0(pair(space0, line_ending)),
                preceded(
                    pair(space0, char(';')),
                    alt((
                        parse_metadata_date,
                        parse_metadata_tag_with_value,
                        parse_comment_with_tags,
                    )),
                ),
            ),
            Metadata::default,
            |meta1, meta2| {
                let mut tags = meta1.tags;
                tags.extend(meta2.tags);
                Metadata {
                    comment: meta1
                        .comment
                        .into_iter()
                        .chain(meta2.comment)
                        .reduce(|mut a, b| {
                            a.push('\n');
                            a.push_str(&b);
                            a
                        }),
                    date: meta2.date.or(meta1.date),
                    effective_date: meta2.effective_date.or(meta1.effective_date),
                    tags,
                }
            },
        ),
        preceded(space0, eol_or_eof),
    )(input)
}

fn parse_include_file(input: &str) -> LedgerParseResult<'_, &str> {
    let (input, _) = delimited(space0, tag("include"), space1)(input)?;
    verify(
        terminated(not_line_ending, eol_or_eof).map(str::trim_end),
        |s: &str| !s.is_empty(),
    )(input)
}

fn take_until_hard_separator(input: &str) -> LedgerParseResult<'_, &str> {
    let mut second_space = false;
    for (pos, c) in input.char_indices() {
        if c == '\t' || c == '\r' || c == '\n' {
            if pos > 0 {
                let (found, rest) = if second_space {
                    input.split_at(pos - 1)
                } else {
                    input.split_at(pos)
                };
                return Ok((rest, found));
            } else {
                return Err(Err::Incomplete(Needed::new(1)));
            }
        }

        if c == ' ' {
            if second_space {
                let (found, rest) = input.split_at(pos - 1);
                return Ok((rest, found));
            } else {
                second_space = true;
            }
        } else {
            second_space = false;

            if pos == input.len() - 1 && pos > 0 {
                return Ok(("", input));
            }
        }
    }

    Err(Err::Incomplete(Needed::new(1)))
}

fn parse_account(input: &str) -> LedgerParseResult<'_, (&str, Reality)> {
    let (input, name) = take_until_hard_separator(input)?;

    if let Some(n1) = name.strip_prefix('[') {
        if let Some(n2) = n1.strip_suffix(']') {
            return Ok((input, (n2, Reality::BalancedVirtual)));
        }
    }

    if let Some(n1) = name.strip_prefix('(') {
        if let Some(n2) = n1.strip_suffix(')') {
            return Ok((input, (n2, Reality::UnbalancedVirtual)));
        }
    }

    Ok((input, (name, Reality::Real)))
}

fn parse_transaction_status(input: &str) -> LedgerParseResult<'_, TransactionStatus> {
    alt((
        value(TransactionStatus::Cleared, char('*')),
        value(TransactionStatus::Pending, char('!')),
    ))(input)
}

fn parse_posting(input: &str) -> LedgerParseResult<'_, Posting> {
    let (input, _) = space1(input)?;
    let (input, status) = opt(parse_transaction_status)(input)?;
    let (input, _) = space0(input)?;
    let (input, (account, reality)) = parse_account(input)?;
    let (input, amount) = opt(preceded(space0, parse_posting_amount))(input)?;
    let (input, balance) = opt(preceded(
        delimited(space0, char('='), space0),
        parse_balance,
    ))(input)?;

    let (
        input,
        Metadata {
            comment,
            date,
            effective_date,
            tags,
        },
    ) = parse_metadata_comments(input)?;

    Ok((
        input,
        Posting {
            account: account.to_owned(),
            reality,
            amount,
            balance,
            status,
            comment,
            metadata: PostingMetadata {
                date,
                effective_date,
                tags,
            },
        },
    ))
}

fn parse_payee(input: &str) -> LedgerParseResult<'_, &str> {
    alt((
        terminated(take_until_hard_separator, peek(pair(space1, char(';')))),
        not_line_ending,
    ))(input)
}

fn parse_transaction(input: &str) -> LedgerParseResult<'_, Transaction> {
    let (input, date) = parse_date(input)?;
    let (input, effective_date) = opt(preceded(char('='), parse_date))(input)?;
    let (input, status) = opt(preceded(space1, parse_transaction_status))(input)?;
    let (input, code) = opt(preceded(
        space1,
        delimited(char('('), is_not(")"), char(')')),
    ))(input)?;
    let (input, description) = opt(preceded(space1, parse_payee))(input)?;

    let (
        input,
        Metadata {
            comment,
            date: posting_date,
            effective_date: posting_effective_date,
            tags,
        },
    ) = parse_metadata_comments(input)?;
    let (input, postings) = many1(parse_posting)(input)?;

    Ok((
        input,
        Transaction {
            comment,
            date,
            effective_date,
            status,
            code: code.map(str::to_owned),
            description: description.map(str::to_owned),
            posting_metadata: PostingMetadata {
                date: posting_date,
                effective_date: posting_effective_date,
                tags,
            },
            postings,
        },
    ))
}

fn parse_ledger_item(input: &str) -> LedgerParseResult<'_, LedgerItem> {
    alt((
        value(LedgerItem::EmptyLine, parse_empty_line),
        parse_global_line_comment
            .map(str::to_owned)
            .map(LedgerItem::LineComment),
        parse_transaction.map(LedgerItem::Transaction),
        parse_commodity_price.map(LedgerItem::CommodityPrice),
        parse_include_file
            .map(str::to_owned)
            .map(LedgerItem::Include),
    ))(input)
}

pub fn parse_ledger(input: &str) -> LedgerParseResult<'_, Ledger> {
    let (input, items) = many0(parse_ledger_item)(input)?;
    let (input, _) = eof(input)?;

    Ok((input, Ledger { items }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nom::{
        error::{ErrorKind, ParseError},
        Err::Error,
    };

    #[test]
    fn parse_date_test() {
        assert_eq!(
            parse_date("2017-03-24"),
            Ok(("", NaiveDate::from_ymd_opt(2017, 3, 24).unwrap()))
        );
        assert_eq!(
            parse_date("2017/03/24"),
            Ok(("", NaiveDate::from_ymd_opt(2017, 3, 24).unwrap()))
        );
        assert_eq!(
            parse_date("2017.03.24"),
            Ok(("", NaiveDate::from_ymd_opt(2017, 3, 24).unwrap()))
        );
        assert_eq!(
            parse_date("2017-13-24"),
            Err(Error(ParseError::from_error_kind(
                "2017-13-24",
                ErrorKind::MapOpt
            )))
        );
    }

    #[test]
    fn parse_datetime_test() {
        assert_eq!(
            parse_datetime("2017-03-24 17:15:23"),
            Ok((
                "",
                NaiveDate::from_ymd_opt(2017, 3, 24)
                    .unwrap()
                    .and_hms_opt(17, 15, 23)
                    .unwrap()
            ))
        );
        assert_eq!(
            parse_datetime("2017-13-24 22:11:22"),
            Err(Error(ParseError::from_error_kind(
                "2017-13-24 22:11:22",
                ErrorKind::MapOpt
            )))
        );
        assert_eq!(
            parse_datetime("2017-03-24 25:11:22"),
            Err(Error(ParseError::from_error_kind(
                "2017-03-24 25:11:22",
                ErrorKind::MapOpt
            )))
        );
    }

    #[test]
    fn parse_quantity_test() {
        assert_eq!(parse_quantity("1000"), Ok(("", Decimal::new(1000, 0))));
        assert_eq!(parse_quantity("2.02"), Ok(("", Decimal::new(202, 2))));
        assert_eq!(parse_quantity("-12.13"), Ok(("", Decimal::new(-1213, 2))));
        assert_eq!(parse_quantity("0.1"), Ok(("", Decimal::new(1, 1))));
        assert_eq!(parse_quantity("3"), Ok(("", Decimal::new(3, 0))));
        assert_eq!(parse_quantity("1"), Ok(("", Decimal::new(1, 0))));
        assert_eq!(parse_quantity("1,000"), Ok(("", Decimal::new(1000, 0))));
        assert_eq!(
            parse_quantity("12,456,132.14"),
            Ok(("", Decimal::new(1245613214, 2)))
        );
    }

    #[test]
    fn parse_commodity_test() {
        assert_eq!(
            parse_commodity("\"ABC 123\""),
            Ok(("", "ABC 123".to_owned()))
        );
        assert_eq!(parse_commodity("ABC "), Ok((" ", "ABC".to_owned())));
        assert_eq!(parse_commodity("$1"), Ok(("1", "$".to_owned())));
        assert_eq!(parse_commodity("€1"), Ok(("1", "€".to_owned())));
        assert_eq!(parse_commodity("€ "), Ok((" ", "€".to_owned())));
        assert_eq!(parse_commodity("€-1"), Ok(("-1", "€".to_owned())));
    }

    #[test]
    fn parse_amount_test() {
        assert_eq!(
            parse_amount("$1.20"),
            Ok((
                "",
                Amount {
                    quantity: Decimal::new(120, 2),
                    commodity: Commodity {
                        name: "$".to_owned(),
                        position: CommodityPosition::Left
                    }
                }
            ))
        );
        assert_eq!(
            parse_amount("$-1.20"),
            Ok((
                "",
                Amount {
                    quantity: Decimal::new(-120, 2),
                    commodity: Commodity {
                        name: "$".to_owned(),
                        position: CommodityPosition::Left
                    }
                }
            ))
        );
        assert_eq!(
            parse_amount("-$1.20 "),
            Ok((
                " ",
                Amount {
                    quantity: Decimal::new(-120, 2),
                    commodity: Commodity {
                        name: "$".to_owned(),
                        position: CommodityPosition::Left
                    }
                }
            ))
        );
        assert_eq!(
            parse_amount("- $ 1.20"),
            Ok((
                "",
                Amount {
                    quantity: Decimal::new(-120, 2),
                    commodity: Commodity {
                        name: "$".to_owned(),
                        position: CommodityPosition::Left
                    }
                }
            ))
        );
        assert_eq!(
            parse_amount("1.20USD"),
            Ok((
                "",
                Amount {
                    quantity: Decimal::new(120, 2),
                    commodity: Commodity {
                        name: "USD".to_owned(),
                        position: CommodityPosition::Right
                    }
                }
            ))
        );
        assert_eq!(
            parse_amount("1.20USD "),
            Ok((
                " ",
                Amount {
                    quantity: Decimal::new(120, 2),
                    commodity: Commodity {
                        name: "USD".to_owned(),
                        position: CommodityPosition::Right
                    }
                }
            ))
        );
        assert_eq!(
            parse_amount("-1.20 USD"),
            Ok((
                "",
                Amount {
                    quantity: Decimal::new(-120, 2),
                    commodity: Commodity {
                        name: "USD".to_owned(),
                        position: CommodityPosition::Right
                    }
                }
            ))
        );
    }

    #[test]
    fn parse_lot_price_test() {
        assert_eq!(
            parse_lot_price("{$1.20}"),
            Ok((
                "",
                Price::Unit(Amount {
                    quantity: Decimal::new(120, 2),
                    commodity: Commodity {
                        name: "$".to_owned(),
                        position: CommodityPosition::Left
                    }
                })
            ))
        );
        assert_eq!(
            parse_lot_price("{ $1.20 }"),
            Ok((
                "",
                Price::Unit(Amount {
                    quantity: Decimal::new(120, 2),
                    commodity: Commodity {
                        name: "$".to_owned(),
                        position: CommodityPosition::Left
                    }
                })
            ))
        );
        assert_eq!(
            parse_lot_price("{1.20PLN}"),
            Ok((
                "",
                Price::Unit(Amount {
                    quantity: Decimal::new(120, 2),
                    commodity: Commodity {
                        name: "PLN".to_owned(),
                        position: CommodityPosition::Right
                    }
                })
            ))
        );
        assert_eq!(
            parse_lot_price("{ 1.20 PLN } "),
            Ok((
                " ",
                Price::Unit(Amount {
                    quantity: Decimal::new(120, 2),
                    commodity: Commodity {
                        name: "PLN".to_owned(),
                        position: CommodityPosition::Right
                    }
                })
            ))
        );
    }

    #[test]
    fn parse_price_test() {
        assert_eq!(
            parse_price("@$1.20"),
            Ok((
                "",
                Price::Unit(Amount {
                    quantity: Decimal::new(120, 2),
                    commodity: Commodity {
                        name: "$".to_owned(),
                        position: CommodityPosition::Left
                    }
                })
            ))
        );
        assert_eq!(
            parse_price("@ $1.20"),
            Ok((
                "",
                Price::Unit(Amount {
                    quantity: Decimal::new(120, 2),
                    commodity: Commodity {
                        name: "$".to_owned(),
                        position: CommodityPosition::Left
                    }
                })
            ))
        );
        assert_eq!(
            parse_price("@@1.20PLN"),
            Ok((
                "",
                Price::Total(Amount {
                    quantity: Decimal::new(120, 2),
                    commodity: Commodity {
                        name: "PLN".to_owned(),
                        position: CommodityPosition::Right
                    }
                })
            ))
        );
        assert_eq!(
            parse_price("@@ 1.20 PLN "),
            Ok((
                " ",
                Price::Total(Amount {
                    quantity: Decimal::new(120, 2),
                    commodity: Commodity {
                        name: "PLN".to_owned(),
                        position: CommodityPosition::Right
                    }
                })
            ))
        );
    }

    #[test]
    fn parse_posting_amount_test() {
        assert_eq!(
            parse_posting_amount("$1.20"),
            Ok((
                "",
                PostingAmount {
                    amount: Amount {
                        quantity: Decimal::new(120, 2),
                        commodity: Commodity {
                            name: "$".to_owned(),
                            position: CommodityPosition::Left
                        }
                    },
                    lot_price: None,
                    price: None
                }
            ))
        );
        assert_eq!(
            parse_posting_amount("$1.20 @ 5.00 PLN"),
            Ok((
                "",
                PostingAmount {
                    amount: Amount {
                        quantity: Decimal::new(120, 2),
                        commodity: Commodity {
                            name: "$".to_owned(),
                            position: CommodityPosition::Left
                        }
                    },
                    lot_price: None,
                    price: Some(Price::Unit(Amount {
                        quantity: Decimal::new(500, 2),
                        commodity: Commodity {
                            name: "PLN".to_owned(),
                            position: CommodityPosition::Right
                        }
                    }))
                }
            ))
        );
        assert_eq!(
            parse_posting_amount("$1.20 {5.00 PLN}"),
            Ok((
                "",
                PostingAmount {
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
                    price: None,
                }
            ))
        );
        assert_eq!(
            parse_posting_amount("$1.20 {{5.00 PLN}} @@6.0PLN "),
            Ok((
                " ",
                PostingAmount {
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
                    })),
                }
            ))
        );
    }

    #[test]
    fn parse_balance_test() {
        assert_eq!(
            parse_balance("$1.20"),
            Ok((
                "",
                Balance::Amount(Amount {
                    quantity: Decimal::new(120, 2),
                    commodity: Commodity {
                        name: "$".to_owned(),
                        position: CommodityPosition::Left
                    }
                })
            ))
        );
        assert_eq!(
            parse_balance("0 PLN"),
            Ok((
                "",
                Balance::Amount(Amount {
                    quantity: Decimal::new(0, 0),
                    commodity: Commodity {
                        name: "PLN".to_owned(),
                        position: CommodityPosition::Right
                    }
                })
            ))
        );
        assert_eq!(parse_balance("0"), Ok(("", Balance::Zero)));
    }

    #[test]
    fn parse_commodity_price_test() {
        assert_eq!(
            parse_commodity_price("P 2017-11-12 12:00:00 mBH 5.00 PLN"),
            Ok((
                "",
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
            ))
        );
    }

    #[test]
    fn parse_account_test() {
        assert_eq!(
            parse_account("TEST:ABC 123  "),
            Ok(("  ", ("TEST:ABC 123", Reality::Real)))
        );
        assert_eq!(
            parse_account("TEST:ABC 123\t"),
            Ok(("\t", ("TEST:ABC 123", Reality::Real)))
        );
        assert_eq!(
            parse_account("TEST:ABC 123"),
            Ok(("", ("TEST:ABC 123", Reality::Real)))
        );
        assert_eq!(
            parse_account("[TEST:ABC 123]"),
            Ok(("", ("TEST:ABC 123", Reality::BalancedVirtual)))
        );
        assert_eq!(
            parse_account("(TEST:ABC 123)"),
            Ok(("", ("TEST:ABC 123", Reality::UnbalancedVirtual)))
        );
    }

    #[test]
    fn parse_transaction_status_test() {
        assert_eq!(
            parse_transaction_status("!"),
            Ok(("", TransactionStatus::Pending))
        );
        assert_eq!(
            parse_transaction_status("*"),
            Ok(("", TransactionStatus::Cleared))
        );
    }

    #[test]
    fn parse_posting_test() {
        assert_eq!(
            parse_posting(" TEST:ABC 123  $1.20"),
            Ok((
                "",
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
            ))
        );
        assert_eq!(
            parse_posting(" ! TEST:ABC 123  $1.20;test\n;comment line 2"),
            Ok((
                "",
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
                    status: Some(TransactionStatus::Pending),
                    comment: Some("test\ncomment line 2".to_owned()),
                    metadata: PostingMetadata {
                        date: None,
                        effective_date: None,
                        tags: vec![],
                    },
                }
            ))
        );
        assert_eq!(
            parse_posting(" ! TEST:ABC 123;test\n;comment"),
            Ok((
                "",
                Posting {
                    account: "TEST:ABC 123;test".to_owned(),
                    reality: Reality::Real,
                    amount: None,
                    balance: None,
                    status: Some(TransactionStatus::Pending),
                    comment: Some("comment".to_owned()),
                    metadata: PostingMetadata {
                        date: None,
                        effective_date: None,
                        tags: vec![],
                    },
                }
            ))
        );
        assert_eq!(
            parse_posting(" ! TEST:ABC 123  ;test\n;comment line 2"),
            Ok((
                "",
                Posting {
                    account: "TEST:ABC 123".to_owned(),
                    reality: Reality::Real,
                    amount: None,
                    balance: None,
                    status: Some(TransactionStatus::Pending),
                    comment: Some("test\ncomment line 2".to_owned()),
                    metadata: PostingMetadata {
                        date: None,
                        effective_date: None,
                        tags: vec![],
                    },
                }
            ))
        );
        assert_eq!(
            parse_posting(" ! TEST:ABC 123   ;  test     \n       ;        comment line 2     "),
            Ok((
                "",
                Posting {
                    account: "TEST:ABC 123".to_owned(),
                    reality: Reality::Real,
                    amount: None,
                    balance: None,
                    status: Some(TransactionStatus::Pending),
                    comment: Some("test\ncomment line 2".to_owned()),
                    metadata: PostingMetadata {
                        date: None,
                        effective_date: None,
                        tags: vec![],
                    },
                }
            ))
        );
        assert_eq!(
            parse_posting(" TEST:ABC 123  $1.20 = $2.40 ;comment"),
            Ok((
                "",
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
                    balance: Some(Balance::Amount(Amount {
                        quantity: Decimal::new(240, 2),
                        commodity: Commodity {
                            name: "$".to_owned(),
                            position: CommodityPosition::Left
                        }
                    })),
                    status: None,
                    comment: Some("comment".to_owned()),
                    metadata: PostingMetadata {
                        date: None,
                        effective_date: None,
                        tags: vec![],
                    },
                }
            ))
        );
        assert_eq!(
            parse_posting(" TEST:ABC 123"),
            Ok((
                "",
                Posting {
                    account: "TEST:ABC 123".to_owned(),
                    reality: Reality::Real,
                    amount: None,
                    balance: None,
                    status: None,
                    comment: None,
                    metadata: PostingMetadata {
                        date: None,
                        effective_date: None,
                        tags: vec![],
                    },
                }
            ))
        );
        assert_eq!(
            parse_posting(" TEST:ABC 123   ; 456"),
            Ok((
                "",
                Posting {
                    account: "TEST:ABC 123".to_owned(),
                    reality: Reality::Real,
                    amount: None,
                    balance: None,
                    status: None,
                    comment: Some("456".to_owned()),
                    metadata: PostingMetadata {
                        date: None,
                        effective_date: None,
                        tags: vec![],
                    },
                }
            ))
        );
        assert_eq!(
            parse_posting(" TEST:ABC 123   ; [2018-10-01]"),
            Ok((
                "",
                Posting {
                    account: "TEST:ABC 123".to_owned(),
                    reality: Reality::Real,
                    amount: None,
                    balance: None,
                    status: None,
                    comment: None,
                    metadata: PostingMetadata {
                        date: Some(NaiveDate::from_ymd_opt(2018, 10, 1).unwrap()),
                        effective_date: None,
                        tags: vec![],
                    },
                }
            ))
        );
        assert_eq!(
            parse_posting(" TEST:ABC 123   ; [=2018-10-01]"),
            Ok((
                "",
                Posting {
                    account: "TEST:ABC 123".to_owned(),
                    reality: Reality::Real,
                    amount: None,
                    balance: None,
                    status: None,
                    comment: None,
                    metadata: PostingMetadata {
                        date: None,
                        effective_date: Some(NaiveDate::from_ymd_opt(2018, 10, 1).unwrap()),
                        tags: vec![],
                    },
                }
            ))
        );
        assert_eq!(
            parse_posting(" TEST:ABC 123   ; [2018-10-01=2018-10-14]"),
            Ok((
                "",
                Posting {
                    account: "TEST:ABC 123".to_owned(),
                    reality: Reality::Real,
                    amount: None,
                    balance: None,
                    status: None,
                    comment: None,
                    metadata: PostingMetadata {
                        date: Some(NaiveDate::from_ymd_opt(2018, 10, 1).unwrap()),
                        effective_date: Some(NaiveDate::from_ymd_opt(2018, 10, 14).unwrap()),
                        tags: vec![],
                    },
                }
            ))
        );
        assert_eq!(
            parse_posting(" TEST:ABC 123   ; Tag: tag value"),
            Ok((
                "",
                Posting {
                    account: "TEST:ABC 123".to_owned(),
                    reality: Reality::Real,
                    amount: None,
                    balance: None,
                    status: None,
                    comment: None,
                    metadata: PostingMetadata {
                        date: None,
                        effective_date: None,
                        tags: vec![Tag {
                            name: "Tag".to_owned(),
                            value: Some(TagValue::String("tag value".to_owned())),
                        }],
                    },
                }
            ))
        );
        assert_eq!(
            parse_posting(" TEST:ABC 123   ; :tag1:tag2:   "),
            Ok((
                "",
                Posting {
                    account: "TEST:ABC 123".to_owned(),
                    reality: Reality::Real,
                    amount: None,
                    balance: None,
                    status: None,
                    comment: None,
                    metadata: PostingMetadata {
                        date: None,
                        effective_date: None,
                        tags: vec![
                            Tag {
                                name: "tag1".to_owned(),
                                value: None,
                            },
                            Tag {
                                name: "tag2".to_owned(),
                                value: None,
                            }
                        ],
                    },
                }
            ))
        );
        assert_eq!(
            parse_posting(" TEST:ABC 123   ; comment :tag1:tag2: and: more comment"),
            Ok((
                "",
                Posting {
                    account: "TEST:ABC 123".to_owned(),
                    reality: Reality::Real,
                    amount: None,
                    balance: None,
                    status: None,
                    comment: Some("comment and: more comment".to_owned()),
                    metadata: PostingMetadata {
                        date: None,
                        effective_date: None,
                        tags: vec![
                            Tag {
                                name: "tag1".to_owned(),
                                value: None,
                            },
                            Tag {
                                name: "tag2".to_owned(),
                                value: None,
                            }
                        ],
                    },
                }
            ))
        );
        assert_eq!(
            parse_posting(" TEST:ABC 123   ; Tag:: [2018-01-05]"),
            Ok((
                "",
                Posting {
                    account: "TEST:ABC 123".to_owned(),
                    reality: Reality::Real,
                    amount: None,
                    balance: None,
                    status: None,
                    comment: None,
                    metadata: PostingMetadata {
                        date: None,
                        effective_date: None,
                        tags: vec![Tag {
                            name: "Tag".to_owned(),
                            value: Some(TagValue::Date(
                                NaiveDate::from_ymd_opt(2018, 1, 5).unwrap()
                            ))
                        }],
                    },
                }
            ))
        );
        assert_eq!(
            parse_posting(" TEST:ABC 123   ; TheAnswer:: 42"),
            Ok((
                "",
                Posting {
                    account: "TEST:ABC 123".to_owned(),
                    reality: Reality::Real,
                    amount: None,
                    balance: None,
                    status: None,
                    comment: None,
                    metadata: PostingMetadata {
                        date: None,
                        effective_date: None,
                        tags: vec![Tag {
                            name: "TheAnswer".to_owned(),
                            value: Some(TagValue::Integer(42))
                        }],
                    },
                }
            ))
        );
        assert_eq!(
            parse_posting(" TEST:ABC 123   ; ISquared:: -1"),
            Ok((
                "",
                Posting {
                    account: "TEST:ABC 123".to_owned(),
                    reality: Reality::Real,
                    amount: None,
                    balance: None,
                    status: None,
                    comment: None,
                    metadata: PostingMetadata {
                        date: None,
                        effective_date: None,
                        tags: vec![Tag {
                            name: "ISquared".to_owned(),
                            value: Some(TagValue::Integer(-1))
                        }],
                    },
                }
            ))
        );
        assert_eq!(
            parse_posting(" TEST:ABC 123   ; Pi:: 3.141592653589793"),
            Ok((
                "",
                Posting {
                    account: "TEST:ABC 123".to_owned(),
                    reality: Reality::Real,
                    amount: None,
                    balance: None,
                    status: None,
                    comment: None,
                    metadata: PostingMetadata {
                        date: None,
                        effective_date: None,
                        tags: vec![Tag {
                            name: "Pi".to_owned(),
                            value: Some(TagValue::Float(
                                NotNan::new(std::f64::consts::PI).unwrap()
                            ))
                        }],
                    },
                }
            ))
        );
    }

    /// Parses a posting line, which is expected to be valid, and returns the posting.
    fn parsed_posting(input: &str) -> Posting {
        let (rest, posting) = parse_posting(input).unwrap();
        assert_eq!(rest, "");
        posting
    }

    /// Parses a posting line, which is expected to be valid, and returns its metadata tags.
    fn parsed_posting_tags(input: &str) -> Vec<Tag> {
        parsed_posting(input).metadata.tags
    }

    fn tag_without_value(name: &str) -> Tag {
        Tag {
            name: name.to_owned(),
            value: None,
        }
    }

    fn string_tag(name: &str, value: &str) -> Tag {
        Tag {
            name: name.to_owned(),
            value: Some(TagValue::String(value.to_owned())),
        }
    }

    #[test]
    fn parse_tags_test() {
        assert_eq!(
            parse_tags(":alphanum1:with-hyphen:with.dot:with_underscore:"),
            Ok((
                "",
                vec![
                    tag_without_value("alphanum1"),
                    tag_without_value("with-hyphen"),
                    tag_without_value("with.dot"),
                    tag_without_value("with_underscore"),
                ]
            ))
        );
        // Spaces and tabs end a tag name, so they cannot be a part of it
        assert!(parse_tags(":with space:").is_err());
        assert!(parse_tags(":with\ttab:").is_err());
        // A series is a whole token, it never ends in the middle of one
        assert!(parse_tags(":tag:extra text").is_err());
        assert!(parse_tags(":tag:extra\ntext").is_err());
    }

    #[test]
    fn parse_metadata_tag_names_test() {
        // Ledger-cli takes everything up to the first space or tab as a tag name, so tag
        // names are not limited to alphanumerics
        assert_eq!(
            parsed_posting_tags(" TEST:ABC 123  ; :alphanum1:with-hyphen:with.dot:"),
            vec![
                tag_without_value("alphanum1"),
                tag_without_value("with-hyphen"),
                tag_without_value("with.dot"),
            ]
        );
        assert_eq!(
            parsed_posting_tags(" TEST:ABC 123  ; keyalphanum1: alphanum1 afterspace"),
            vec![string_tag("keyalphanum1", "alphanum1 afterspace")]
        );
        assert_eq!(
            parsed_posting_tags(" TEST:ABC 123  ; key-with-hyphen: with hyphen"),
            vec![string_tag("key-with-hyphen", "with hyphen")]
        );
        assert_eq!(
            parsed_posting_tags(" TEST:ABC 123  ; key.with.dot: with dot"),
            vec![string_tag("key.with.dot", "with dot")]
        );
        assert_eq!(
            parsed_posting_tags(" TEST:ABC 123  ; key_with_underscore: with underscore"),
            vec![string_tag("key_with_underscore", "with underscore")]
        );
        assert_eq!(
            parsed_posting_tags(" TEST:ABC 123  ; key-with-hyphen:: -1"),
            vec![Tag {
                name: "key-with-hyphen".to_owned(),
                value: Some(TagValue::Integer(-1)),
            }]
        );
        assert_eq!(
            parsed_posting_tags(" TEST:ABC 123  ; key.with.dot:: [2018-01-05]"),
            vec![Tag {
                name: "key.with.dot".to_owned(),
                value: Some(TagValue::Date(NaiveDate::from_ymd_opt(2018, 1, 5).unwrap())),
            }]
        );

        // Additional ';' characters preceding a tag are skipped, they are not a part of the name
        assert_eq!(
            parsed_posting_tags(" TEST:ABC 123  ;;key-with-hyphen: with hyphen"),
            vec![string_tag("key-with-hyphen", "with hyphen")]
        );
        assert_eq!(
            parsed_posting_tags(" TEST:ABC 123  ; ;  ;key.with.dot: with dot"),
            vec![string_tag("key.with.dot", "with dot")]
        );

        // A space or a tab cannot be a part of a tag name, such comments are kept as
        // plain comments. The same applies to text which just looks like a series of tags:
        // ledger-cli splits comments into tokens separated by whitespace and a tag series
        // has to be a whole token.
        for comment in [
            ":with space:",
            "key with space: with space",
            "a:b: value",
            "12:30: meeting",
            ":tag:extra",
            "see http://example.com/tag: value",
        ] {
            let posting = parsed_posting(&format!(" TEST:ABC 123  ; {comment}"));
            assert!(
                posting.metadata.tags.is_empty(),
                "unexpected tags for comment {comment:?}"
            );
            assert_eq!(posting.comment, Some(comment.to_owned()));
        }
    }

    #[test]
    fn parse_transaction_metadata_tags_test() {
        let ledger = parse_ledger(
            r#"2020-01-01 Description
  ; :alphanum1:
  ; :with-hyphen:
  ; :with.dot:
  ; :with_underscore:
  ; :with space:
  ; keyalphanum1: alphanum1 afterspace
  ; key-with-hyphen: with hyphen
  ; key.with.dot: with dot
  ; key_with_underscore: with underscore
  ; key with space: with space
  expenses:dummy  $1
  income:dummy  -$1
"#,
        )
        .unwrap()
        .1;

        let transaction = match &ledger.items[0] {
            LedgerItem::Transaction(transaction) => transaction,
            other => panic!("expected a transaction, got {other:?}"),
        };
        assert_eq!(
            transaction.posting_metadata.tags,
            vec![
                tag_without_value("alphanum1"),
                tag_without_value("with-hyphen"),
                tag_without_value("with.dot"),
                tag_without_value("with_underscore"),
                string_tag("keyalphanum1", "alphanum1 afterspace"),
                string_tag("key-with-hyphen", "with hyphen"),
                string_tag("key.with.dot", "with dot"),
                string_tag("key_with_underscore", "with underscore"),
            ]
        );
        assert_eq!(
            transaction.comment,
            Some(":with space:\nkey with space: with space".to_owned())
        );
    }

    #[test]
    fn parse_transaction_test() {
        assert_eq!(
            parse_transaction(
                r#"2018-10-01=2018-10-14 ! (123) Marek Ogarek  ; Transaction comment
 TEST:ABC 123  $1.20 ; Posting comment
                     ; over two lines
 TEST:ABC 123  $1.20"#
            ),
            Ok((
                "",
                Transaction {
                    comment: Some("Transaction comment".to_owned()),
                    date: NaiveDate::from_ymd_opt(2018, 10, 1).unwrap(),
                    effective_date: Some(NaiveDate::from_ymd_opt(2018, 10, 14).unwrap()),
                    posting_metadata: PostingMetadata {
                        date: None,
                        effective_date: None,
                        tags: vec![],
                    },
                    status: Some(TransactionStatus::Pending),
                    code: Some("123".to_owned()),
                    description: Some("Marek Ogarek".to_owned()),
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
                            comment: Some("Posting comment\nover two lines".to_owned()),
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
                }
            ))
        );
        assert_eq!(
            parse_transaction(
                r#"2018-10-01=2018-10-14 Marek Ogarek ; one space
 TEST:ABC 123  $1.20 ; test
 TEST:DEF 123  EUR-1.20
 TEST:GHI 123
 TEST:JKL 123  EUR-2.00"#
            ),
            Ok((
                "",
                Transaction {
                    comment: None,
                    date: NaiveDate::from_ymd_opt(2018, 10, 1).unwrap(),
                    effective_date: Some(NaiveDate::from_ymd_opt(2018, 10, 14).unwrap()),
                    posting_metadata: PostingMetadata {
                        date: None,
                        effective_date: None,
                        tags: vec![],
                    },
                    status: None,
                    code: None,
                    description: Some("Marek Ogarek ; one space".to_owned()),
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
                            comment: Some("test".to_owned()),
                            metadata: PostingMetadata {
                                date: None,
                                effective_date: None,
                                tags: vec![],
                            },
                        },
                        Posting {
                            balance: None,
                            account: "TEST:DEF 123".to_owned(),
                            reality: Reality::Real,
                            amount: Some(PostingAmount {
                                amount: Amount {
                                    quantity: Decimal::new(-120, 2),
                                    commodity: Commodity {
                                        name: "EUR".to_owned(),
                                        position: CommodityPosition::Left
                                    }
                                },
                                lot_price: None,
                                price: None
                            }),
                            status: None,
                            comment: None,
                            metadata: PostingMetadata {
                                date: None,
                                effective_date: None,
                                tags: vec![],
                            },
                        },
                        Posting {
                            account: "TEST:GHI 123".to_owned(),
                            reality: Reality::Real,
                            amount: None,
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
                            account: "TEST:JKL 123".to_owned(),
                            reality: Reality::Real,
                            amount: Some(PostingAmount {
                                amount: Amount {
                                    quantity: Decimal::new(-200, 2),
                                    commodity: Commodity {
                                        name: "EUR".to_owned(),
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
                        },
                    ]
                }
            ))
        );
        assert_eq!(
            parse_transaction(
                r#"2018-10-01=2018-10-14 ! (123) Marek Ogarek  two spaces
 TEST:ABC 123  $1.20 ; test
 TEST:DEF 123"#
            ),
            Ok((
                "",
                Transaction {
                    comment: None,
                    date: NaiveDate::from_ymd_opt(2018, 10, 1).unwrap(),
                    effective_date: Some(NaiveDate::from_ymd_opt(2018, 10, 14).unwrap()),
                    posting_metadata: PostingMetadata {
                        date: None,
                        effective_date: None,
                        tags: vec![],
                    },
                    status: Some(TransactionStatus::Pending),
                    code: Some("123".to_owned()),
                    description: Some("Marek Ogarek  two spaces".to_owned()),
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
                            comment: Some("test".to_owned()),
                            metadata: PostingMetadata {
                                date: None,
                                effective_date: None,
                                tags: vec![],
                            },
                        },
                        Posting {
                            account: "TEST:DEF 123".to_owned(),
                            reality: Reality::Real,
                            amount: None,
                            balance: None,
                            status: None,
                            comment: None,
                            metadata: PostingMetadata {
                                date: None,
                                effective_date: None,
                                tags: vec![],
                            },
                        },
                    ]
                }
            ))
        );

        // same transaction, but no payee/description (these are optional in ledger)
        assert_eq!(
            parse_transaction(
                r#"2018-10-01=2018-10-14 ! (123)
 TEST:ABC 123  $1.20
 TEST:DEF 123"#
            ),
            Ok((
                "",
                Transaction {
                    comment: None,
                    date: NaiveDate::from_ymd_opt(2018, 10, 1).unwrap(),
                    effective_date: Some(NaiveDate::from_ymd_opt(2018, 10, 14).unwrap()),
                    posting_metadata: PostingMetadata {
                        date: None,
                        effective_date: None,
                        tags: vec![],
                    },
                    status: Some(TransactionStatus::Pending),
                    code: Some("123".to_owned()),
                    description: None,
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
                            comment: None,
                            metadata: PostingMetadata {
                                date: None,
                                effective_date: None,
                                tags: vec![],
                            },
                        },
                        Posting {
                            account: "TEST:DEF 123".to_owned(),
                            reality: Reality::Real,
                            amount: None,
                            balance: None,
                            status: None,
                            comment: None,
                            metadata: PostingMetadata {
                                date: None,
                                effective_date: None,
                                tags: vec![],
                            },
                        },
                    ]
                }
            ))
        );
    }

    #[test]
    fn parse_include_test() {
        assert_eq!(
            parse_include_file(r#"include other_file.ledger"#),
            Ok(("", "other_file.ledger"))
        );
    }

    #[test]
    fn parse_ledger_test() {
        let res = parse_ledger(
            r#"; Example 1

include other_file.ledger

P 2017-11-12 12:00:00 mBH 5.00 PLN  ; comment

; Comment
2018-10-01=2018-10-14 ! (123) Marek Ogarek
 TEST:ABC 123  $1.20
 TEST:ABC 123  $1.20

2018-10-01=2018-10-14 ! (123) Marek Ogarek
 TEST:ABC 123  $1.20
 TEST:ABC 123  $1.20
"#,
        )
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
