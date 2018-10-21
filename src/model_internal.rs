use model::*;

pub enum LedgerItem {
    EmptyLine,
    LineComment(String),
    Transaction(Transaction),
    CommodityPrice(CommodityPrice),
}

pub fn convert_items_to_ledger(items: Vec<LedgerItem>) -> Ledger {
    let mut transactions = Vec::<Transaction>::new();
    let mut commodity_prices = Vec::<CommodityPrice>::new();

    let mut current_comment: Option<String> = None;

    for item in items {
        match item {
            LedgerItem::EmptyLine => {
                current_comment = None;
            }
            LedgerItem::LineComment(comment) => {
                if let Some(ref mut c) = current_comment {
                    c.push_str("\n");
                    c.push_str(&comment);
                } else {
                    current_comment = Some(comment);
                }
            }
            LedgerItem::Transaction(mut transaction) => {
                transaction.comment = current_comment.take();
                transactions.push(transaction);
            }
            LedgerItem::CommodityPrice(commodity_price) => {
                current_comment = None;
                commodity_prices.push(commodity_price);
            }
        }
    }

    Ledger {
        transactions: transactions,
        commodity_prices: commodity_prices,
    }
}
