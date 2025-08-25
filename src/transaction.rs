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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_type_parsing() {
        assert!(matches!(
            TransactionType::from_str("deposit").unwrap(),
            TransactionType::Deposit
        ));
        assert!(matches!(
            TransactionType::from_str("withdrawal").unwrap(),
            TransactionType::Withdrawal
        ));
        assert!(matches!(
            TransactionType::from_str("dispute").unwrap(),
            TransactionType::Dispute
        ));

        // Test case insensitive
        assert!(matches!(
            TransactionType::from_str("DEPOSIT").unwrap(),
            TransactionType::Deposit
        ));

        // Test invalid type
        assert!(TransactionType::from_str("invalid").is_err());
    }

    #[test]
    fn test_transaction_record_validate_with_amount() {
        let valid_deposit = TransactionRecord {
            tx_type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: Some(Decimal::from_str("10.0").unwrap()),
        };
        assert!(valid_deposit.validate().is_ok());

        let invalid_deposit = TransactionRecord {
            tx_type: TransactionType::Deposit,
            client: 1,
            tx: 1,
            amount: None,
        };
        assert!(invalid_deposit.validate().is_err());
    }

    #[test]
    fn test_transaction_record_validate_without_amount() {
        let dispute = TransactionRecord {
            tx_type: TransactionType::Dispute,
            client: 1,
            tx: 1,
            amount: None,
        };
        assert!(dispute.validate().is_ok());

        let resolve = TransactionRecord {
            tx_type: TransactionType::Resolve,
            client: 1,
            tx: 1,
            amount: None,
        };
        assert!(resolve.validate().is_ok());
    }

    #[test]
    fn test_withdrawal_validation() {
        let valid_withdrawal = TransactionRecord {
            tx_type: TransactionType::Withdrawal,
            client: 1,
            tx: 1,
            amount: Some(Decimal::from_str("5.0").unwrap()),
        };
        assert!(valid_withdrawal.validate().is_ok());

        let invalid_withdrawal = TransactionRecord {
            tx_type: TransactionType::Withdrawal,
            client: 1,
            tx: 1,
            amount: None,
        };
        assert!(invalid_withdrawal.validate().is_err());
    }

    #[test]
    fn test_stored_transaction_creation() {
        let stored_tx = StoredTransaction {
            client: 123,
            amount: Decimal::from_str("15.5").unwrap(),
            tx_type: TransactionType::Deposit,
            disputed: false,
        };

        assert_eq!(stored_tx.client, 123);
        assert_eq!(stored_tx.amount, Decimal::from_str("15.5").unwrap());
        assert!(matches!(stored_tx.tx_type, TransactionType::Deposit));
        assert!(!stored_tx.disputed);
    }
}
