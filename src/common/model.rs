use chrono::NaiveDateTime;
use rust_decimal::Decimal;
use std::fmt;

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

#[derive(PartialEq, Eq, Clone)]
pub struct Amount {
    pub quantity: Decimal,
    pub commodity: Commodity,
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
}
