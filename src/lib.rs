#[macro_use]
extern crate nom;
extern crate chrono;
extern crate rust_decimal;

mod parser;
mod model;
mod model_internal;

pub use model::*;

pub fn parse(input: &str) -> Result<Ledger, String> {
    use nom::types::CompleteStr;

    let result = parser::parse_ledger_items(CompleteStr(input));
    match result {
        Ok((_, result)) => {
            Ok(model_internal::convert_items_to_ledger(result))
        },
        Err(error) => {
            Err(format!("{:?}", error))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ledger_test() {
        let res = parse(r#"; Example 1

P 2017-11-12 12:00:00 mBH 5.00 PLN
fff
; Comment Line 1
; Comment Line 2
2018-10-01=2018-10-14 ! (123) Marek Ogarek
 TEST:ABC 123  $1.20
 TEST:ABC 123  $1.20

2018-10-01=2018-10-14 ! (123) Marek Ogarek
 TEST:ABC 123  $1.20
 TEST:ABC 123  $1.20
"#);

        print!("{:?}", res);


        let i = 0;
        assert_eq!(i, 0);
    }
}
