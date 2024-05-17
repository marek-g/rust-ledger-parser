use crate::model::*;
use std::io;

#[non_exhaustive]
pub struct SerializerSettings {
    pub indent: String,
    pub eol: String,
    pub transaction_date_format: String,
    pub commodity_date_format: String,
}

impl SerializerSettings {
    pub fn with_indent(mut self, indent: &str) -> Self {
        self.indent = indent.to_owned();
        self
    }

    pub fn with_eol(mut self, eol: &str) -> Self {
        self.eol = eol.to_owned();
        self
    }
}

impl Default for SerializerSettings {
    fn default() -> Self {
        Self {
            indent: "  ".to_owned(),
            eol: "\n".to_owned(),
            transaction_date_format: "%Y-%m-%d".to_owned(),
            commodity_date_format: "%Y-%m-%d %H:%M:%S".to_owned(),
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
        return std::str::from_utf8(&res).unwrap().to_owned();
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
            LedgerItem::EmptyLine => write!(writer, "{}", settings.eol)?,
            LedgerItem::LineComment(comment) => write!(writer, "; {}{}", comment, settings.eol)?,
            LedgerItem::Transaction(transaction) => {
                transaction.write(writer, settings)?;
                write!(writer, "{}", settings.eol)?;
            }
            LedgerItem::CommodityPrice(commodity_price) => {
                commodity_price.write(writer, settings)?;
                write!(writer, "{}", settings.eol)?;
            }
            LedgerItem::Include(file) => write!(writer, "include {}{}", file, settings.eol)?,
        }
        Ok(())
    }
}

impl Serializer for Transaction {
    fn write<W>(&self, writer: &mut W, settings: &SerializerSettings) -> Result<(), io::Error>
    where
        W: io::Write,
    {
        write!(
            writer,
            "{}",
            self.date.format(&settings.transaction_date_format)
        )?;

        if let Some(effective_date) = self.effective_date {
            write!(
                writer,
                "={}",
                effective_date.format(&settings.transaction_date_format)
            )?;
        }

        if let Some(ref status) = self.status {
            write!(writer, " ")?;
            status.write(writer, settings)?;
        }

        if let Some(ref code) = self.code {
            write!(writer, " ({})", code)?;
        }

        // for the None case, ledger would print "<Unspecified payee>"
        if let Some(ref description) = self.description {
            if !description.is_empty() {
                write!(writer, " {}", description)?;
            }
        }

        if let Some(ref comment) = self.comment {
            for comment in comment.split('\n') {
                write!(writer, "{}{}; {}", settings.eol, settings.indent, comment)?;
            }
        }

        for posting in &self.postings {
            write!(writer, "{}{}", settings.eol, settings.indent)?;
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

        if self.amount.is_some() || self.balance.is_some() {
            write!(writer, "{}", settings.indent)?;
        }

        if let Some(ref amount) = self.amount {
            amount.write(writer, settings)?;
        }

        if let Some(ref balance) = self.balance {
            write!(writer, " = ")?;
            balance.write(writer, settings)?;
        }

        if let Some(ref comment) = self.comment {
            for comment in comment.split('\n') {
                write!(writer, "{}{}; {}", settings.eol, settings.indent, comment)?;
            }
        }

        Ok(())
    }
}

impl Serializer for PostingAmount {
    fn write<W>(&self, writer: &mut W, settings: &SerializerSettings) -> Result<(), io::Error>
    where
        W: io::Write,
    {
        self.amount.write(writer, settings)?;

        if let Some(ref lot_price) = self.lot_price {
            match lot_price {
                Price::Unit(amount) => {
                    write!(writer, " {{")?;
                    amount.write(writer, settings)?;
                    write!(writer, "}}")?;
                }
                Price::Total(amount) => {
                    write!(writer, " {{{{")?;
                    amount.write(writer, settings)?;
                    write!(writer, "}}}}")?;
                }
            }
        }

        if let Some(ref lot_price) = self.price {
            match lot_price {
                Price::Unit(amount) => {
                    write!(writer, " @ ")?;
                    amount.write(writer, settings)?;
                }
                Price::Total(amount) => {
                    write!(writer, " @@ ")?;
                    amount.write(writer, settings)?;
                }
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
            self.datetime.format(&settings.commodity_date_format),
            self.commodity_name
        )?;
        self.amount.write(writer, settings)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_transaction() {
        let ledger = crate::parse(
            r#"2018/10/01    (123)     Payee 123
  TEST:ABC 123        $1.20
    TEST:DEF 123"#,
        )
        .expect("parsing test transaction");

        let mut buf = Vec::new();
        ledger
            .write(&mut buf, &SerializerSettings::default())
            .expect("serializing test transaction");

        assert_eq!(
            String::from_utf8(buf).unwrap(),
            r#"2018-10-01 (123) Payee 123
  TEST:ABC 123  $1.20
  TEST:DEF 123
"#
        );
    }

    #[test]
    fn serialize_with_custom_date_format() {
        let ledger = crate::parse(
            r#"2018-10-01    (123)     Payee 123
  TEST:ABC 123        $1.20
    TEST:DEF 123"#,
        )
        .expect("parsing test transaction");

        let mut buf = Vec::new();
        ledger
            .write(
                &mut buf,
                &SerializerSettings {
                    transaction_date_format: "%Y/%m/%d".to_owned(),
                    ..SerializerSettings::default()
                },
            )
            .expect("serializing test transaction");

        assert_eq!(
            String::from_utf8(buf).unwrap(),
            r#"2018/10/01 (123) Payee 123
  TEST:ABC 123  $1.20
  TEST:DEF 123
"#
        );
    }
}
