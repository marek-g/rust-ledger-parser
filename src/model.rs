use chrono::{NaiveDate, NaiveDateTime};
use rust_decimal::Decimal;

///
/// Ledger contains transactions and/or commodity prices.
///
#[derive(Debug, PartialEq, Eq)]
pub struct Ledger {
    pub transactions: Vec<Transaction>,
    pub commodity_prices: Vec<CommodityPrice>,
}

///
/// Transaction.
///
#[derive(Debug, PartialEq, Eq)]
pub struct Transaction {
    pub comment: Option<String>,
    pub date: NaiveDate,
    pub effective_date: Option<NaiveDate>,
    pub status: Option<TransactionStatus>,
    pub code: Option<String>,
    pub description: String,
    pub postings: Vec<Posting>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TransactionStatus {
    Pending,
    Cleared,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Posting {
    pub account: String,
    pub amount: Amount,
    pub status: Option<TransactionStatus>,
    pub comment: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Amount {
    pub quantity: Decimal,
    pub commodity: Commodity,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Commodity {
    pub name: String,
    pub position: CommodityPosition,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CommodityPosition {
    Left,
    Right,
}

///
/// Commodity price.
///
#[derive(Debug, PartialEq, Eq)]
pub struct CommodityPrice {
    pub datetime: NaiveDateTime,
    pub commodity_name: String,
    pub amount: Amount,
}
