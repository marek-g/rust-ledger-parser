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

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TransactionStatus {
    Pending,
    Cleared,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Posting {
    pub account: String,
    pub amount: Amount,
    pub status: Option<TransactionStatus>,
    pub comment: Option<String>,
}

#[derive(PartialEq, Eq, Clone)]
pub struct Amount {
    pub quantity: Decimal,
    pub commodity: Commodity,
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
