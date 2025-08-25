use anyhow::Result;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::str::FromStr;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TransactionRecord {
    #[serde(rename = "type")]
    pub tx_type: TransactionType,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<Decimal>,
}

#[derive(Debug, Clone)]
pub struct StoredTransaction {
    pub client: u16,
    pub amount: Decimal,
    pub tx_type: TransactionType,
    pub disputed: bool,
}

impl FromStr for TransactionType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_lowercase().as_str() {
            "deposit" => Ok(TransactionType::Deposit),
            "withdrawal" => Ok(TransactionType::Withdrawal),
            "dispute" => Ok(TransactionType::Dispute),
            "resolve" => Ok(TransactionType::Resolve),
            "chargeback" => Ok(TransactionType::Chargeback),
            _ => Err(anyhow::anyhow!("Unknown transaction type: {}", s)),
        }
    }
}

impl TransactionRecord {
    pub fn validate(&self) -> Result<()> {
        match self.tx_type {
            TransactionType::Deposit | TransactionType::Withdrawal => {
                if self.amount.is_none() {
                    anyhow::bail!("Deposit/Withdrawal requires amount");
                }
            }
            _ => {
                // Dispute, Resolve, Chargeback don't have amounts
            }
        }
        Ok(())
    }
}
