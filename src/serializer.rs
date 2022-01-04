use crate::model::*;
use std::io;

#[non_exhaustive]
pub struct SerializerSettings {
    indent: String,
}

impl SerializerSettings {
    pub fn with_indent(mut self, indent: &str) -> Self {
        self.indent = indent.to_string();
        self
    }
}

impl Default for SerializerSettings {
    fn default() -> Self {
        Self {
            indent: "  ".to_string(),
        }
    }
}

pub trait Serializer {
    fn write<W>(&self, writer: &mut W, settings: &SerializerSettings) -> Result<(), io::Error>
    where
        W: io::Write;

    fn to_string_pretty(&self, settings: &SerializerSettings) -> String {
        let mut res = Vec::new();
        self.write(&mut res, settings).unwrap();
        return std::str::from_utf8(&res).unwrap().to_string();
    }
}

impl Serializer for Ledger {
    fn write<W>(&self, writer: &mut W, settings: &SerializerSettings) -> Result<(), io::Error>
    where
        W: io::Write,
    {
        for item in &self.items {
            item.write(writer, settings)?;
        }
        Ok(())
    }
}

impl Serializer for LedgerItem {
    fn write<W>(&self, writer: &mut W, settings: &SerializerSettings) -> Result<(), io::Error>
    where
        W: io::Write,
    {
        match self {
            LedgerItem::EmptyLine => write!(writer, "\n")?,
            LedgerItem::LineComment(comment) => write!(writer, "; {}\n", comment)?,
            LedgerItem::Transaction(transaction) => {
                transaction.write(writer, settings)?;
                write!(writer, "\n")?;
            }
            LedgerItem::CommodityPrice(commodity_price) => {
                commodity_price.write(writer, settings)?;
                write!(writer, "\n")?;
            }
            LedgerItem::Include(file) => write!(writer, "include {}\n", file)?,
        }
        Ok(())
    }
}

impl Serializer for Transaction {
    fn write<W>(&self, writer: &mut W, settings: &SerializerSettings) -> Result<(), io::Error>
    where
        W: io::Write,
    {
        write!(writer, "{}", self.date.format("%Y-%m-%d"))?;

        if let Some(effective_date) = self.effective_date {
            write!(writer, "={}", effective_date.format("%Y-%m-%d"))?;
        }

        if let Some(ref status) = self.status {
            write!(writer, " ")?;
            status.write(writer, settings)?;
        }

        if let Some(ref code) = self.code {
            write!(writer, " ({})", code)?;
        }

        if !self.description.is_empty() {
            write!(writer, " {}", self.description)?;
        }

        if let Some(ref comment) = self.comment {
            for comment in comment.split('\n') {
                write!(writer, "\n{}; {}", settings.indent, comment)?;
            }
        }

        for posting in &self.postings {
            write!(writer, "\n{}", settings.indent)?;
            posting.write(writer, settings)?;
        }

        Ok(())
    }
}

impl Serializer for TransactionStatus {
    fn write<W>(&self, writer: &mut W, _settings: &SerializerSettings) -> Result<(), io::Error>
    where
        W: io::Write,
    {
        match self {
            TransactionStatus::Pending => write!(writer, "!"),
            TransactionStatus::Cleared => write!(writer, "*"),
        }
    }
}

impl Serializer for Posting {
    fn write<W>(&self, writer: &mut W, settings: &SerializerSettings) -> Result<(), io::Error>
    where
        W: io::Write,
    {
        if let Some(ref status) = self.status {
            status.write(writer, settings)?;
            write!(writer, " ")?;
        }

        match self.reality {
            Reality::Real => write!(writer, "{}", self.account)?,
            Reality::BalancedVirtual => write!(writer, "[{}]", self.account)?,
            Reality::UnbalancedVirtual => write!(writer, "({})", self.account)?,
        }

        if let Some(ref amount) = self.amount {
            write!(writer, "{}", settings.indent)?;
            amount.write(writer, settings)?;
        }

        if let Some(ref balance) = self.balance {
            write!(writer, " = ")?;
            balance.write(writer, settings)?;
        }

        if let Some(ref comment) = self.comment {
            for comment in comment.split('\n') {
                write!(writer, "\n{}; {}", settings.indent, comment)?;
            }
        }

        Ok(())
    }
}

impl Serializer for Amount {
    fn write<W>(&self, writer: &mut W, _settings: &SerializerSettings) -> Result<(), io::Error>
    where
        W: io::Write,
    {
        match self.commodity.position {
            CommodityPosition::Left => write!(writer, "{}{}", self.commodity.name, self.quantity),
            CommodityPosition::Right => write!(writer, "{} {}", self.quantity, self.commodity.name),
        }
    }
}

impl Serializer for Balance {
    fn write<W>(&self, writer: &mut W, settings: &SerializerSettings) -> Result<(), io::Error>
    where
        W: io::Write,
    {
        match self {
            Balance::Zero => write!(writer, "0"),
            Balance::Amount(ref balance) => balance.write(writer, settings),
        }
    }
}

impl Serializer for CommodityPrice {
    fn write<W>(&self, writer: &mut W, settings: &SerializerSettings) -> Result<(), io::Error>
    where
        W: io::Write,
    {
        write!(
            writer,
            "P {} {} ",
            self.datetime.format("%Y-%m-%d %H:%M:%S"),
            self.commodity_name
        )?;
        self.amount.write(writer, settings)?;
        Ok(())
    }
}
