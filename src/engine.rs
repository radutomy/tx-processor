use crate::account::{Account, AccountOutput};
use crate::transaction::{StoredTransaction, TransactionRecord, TransactionType};
use anyhow::{Context, Result};
use std::collections::HashMap;

/// The core payment processing engine that manages account states and transaction history.
/// In a real world application, this would likely be backed by a persistent data store,
/// but for demo purposes we use in-memory storage. With more time, I would implement
/// this with an RDBMS backend...
pub struct PaymentEngine {
    /// A HashMap is probably the best structure for in-memory calculation
    /// because we need to frequently look for accounts using the ID.
    /// This will yield a constant time lookup, which is probably the best we can do.
    accounts: HashMap<u16, Account>,
    transactions: HashMap<u32, StoredTransaction>,
}

impl PaymentEngine {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            transactions: HashMap::new(),
        }
    }

    /// We want to decouple the file reading/parsing from the actual processing logic,
    /// this accepts a parsed transaction record and applies it to the appropriate account.
    pub fn process_transaction(&mut self, record: TransactionRecord) -> Result<()> {
        record.validate().context("Invalid transaction")?;

        let account = self
            .accounts
            .entry(record.client)
            .or_insert_with(Account::new);

        match record.tx_type {
            TransactionType::Deposit => {
                let amount = record.amount.context("Deposit missing amount")?;
                account.deposit(amount);

                // Store transaction for potential disputes
                self.transactions.insert(
                    record.tx,
                    StoredTransaction {
                        client: record.client,
                        amount,
                        tx_type: TransactionType::Deposit,
                        disputed: false,
                    },
                );
            }

            TransactionType::Withdrawal => {
                let amount = record.amount.context("Withdrawal missing amount")?;
                let success = account.withdraw(amount);

                // Only store successful withdrawals
                if success {
                    self.transactions.insert(
                        record.tx,
                        StoredTransaction {
                            client: record.client,
                            amount,
                            tx_type: TransactionType::Withdrawal,
                            disputed: false,
                        },
                    );
                }
            }

            TransactionType::Dispute => {
                if let Some(tx) = self.transactions.get_mut(&record.tx) {
                    // Only dispute if client matches and not already disputed
                    if tx.client == record.client && !tx.disputed {
                        tx.disputed = true;
                        account.hold_funds(tx.amount);
                    }
                }
            }

            TransactionType::Resolve => {
                if let Some(tx) = self.transactions.get_mut(&record.tx) {
                    // Only resolve if client matches and is disputed
                    if tx.client == record.client && tx.disputed {
                        tx.disputed = false;
                        account.release_funds(tx.amount);
                    }
                }
            }

            TransactionType::Chargeback => {
                if let Some(tx) = self.transactions.get_mut(&record.tx) {
                    // Only chargeback if client matches and is disputed
                    if tx.client == record.client && tx.disputed {
                        account.chargeback(tx.amount);
                        tx.disputed = false; // Transaction is finalized
                    }
                }
            }
        }

        Ok(())
    }

    pub fn get_accounts(&self) -> Vec<AccountOutput> {
        self.accounts
            .iter()
            .map(|(&client, account)| AccountOutput::from_account(client, account))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::TransactionRecord;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn create_deposit(client: u16, tx: u32, amount: &str) -> TransactionRecord {
        TransactionRecord {
            tx_type: TransactionType::Deposit,
            client,
            tx,
            amount: Some(Decimal::from_str(amount).unwrap()),
        }
    }

    fn create_withdrawal(client: u16, tx: u32, amount: &str) -> TransactionRecord {
        TransactionRecord {
            tx_type: TransactionType::Withdrawal,
            client,
            tx,
            amount: Some(Decimal::from_str(amount).unwrap()),
        }
    }

    fn create_dispute(client: u16, tx: u32) -> TransactionRecord {
        TransactionRecord {
            tx_type: TransactionType::Dispute,
            client,
            tx,
            amount: None,
        }
    }

    fn create_resolve(client: u16, tx: u32) -> TransactionRecord {
        TransactionRecord {
            tx_type: TransactionType::Resolve,
            client,
            tx,
            amount: None,
        }
    }

    fn create_chargeback(client: u16, tx: u32) -> TransactionRecord {
        TransactionRecord {
            tx_type: TransactionType::Chargeback,
            client,
            tx,
            amount: None,
        }
    }

    #[test]
    fn deposit_withdraw_test() {
        let mut engine = PaymentEngine::new();

        // Deposit 10.0 to client 1
        engine
            .process_transaction(create_deposit(1, 1, "10.0"))
            .unwrap();

        // Withdraw 5.0 from client 1
        engine
            .process_transaction(create_withdrawal(1, 2, "5.0"))
            .unwrap();

        let accounts = engine.get_accounts();
        assert_eq!(accounts.len(), 1);

        let account = &accounts[0];
        assert_eq!(account.client, 1);
        assert_eq!(account.available, Decimal::from_str("5.0").unwrap());
        assert_eq!(account.held, Decimal::from_str("0.0").unwrap());
        assert_eq!(account.total, Decimal::from_str("5.0").unwrap());
        assert!(!account.locked);
    }

    #[test]
    fn fail_withdraw_no_funds() {
        let mut engine = PaymentEngine::new();

        // Deposit 5.0 to client 1
        engine
            .process_transaction(create_deposit(1, 1, "5.0"))
            .unwrap();

        // Try to withdraw 10.0 from client 1 (should fail)
        engine
            .process_transaction(create_withdrawal(1, 2, "10.0"))
            .unwrap();

        let accounts = engine.get_accounts();
        assert_eq!(accounts.len(), 1);

        let account = &accounts[0];
        assert_eq!(account.client, 1);
        assert_eq!(account.available, Decimal::from_str("5.0").unwrap());
        assert_eq!(account.held, Decimal::from_str("0.0").unwrap());
        assert_eq!(account.total, Decimal::from_str("5.0").unwrap());
        assert!(!account.locked);
    }

    #[test]
    fn success_successive_no_transactions_after_failure() {
        let mut engine = PaymentEngine::new();

        // Deposit 5.0 to client 1
        engine
            .process_transaction(create_deposit(1, 1, "5.0"))
            .unwrap();

        // Try to withdraw 10.0 from client 1 (should fail)
        engine
            .process_transaction(create_withdrawal(1, 2, "10.0"))
            .unwrap();

        // Withdraw 3.0 from client 1 (should succeed)
        engine
            .process_transaction(create_withdrawal(1, 3, "3.0"))
            .unwrap();

        let accounts = engine.get_accounts();
        assert_eq!(accounts.len(), 1);

        let account = &accounts[0];
        assert_eq!(account.client, 1);
        assert_eq!(account.available, Decimal::from_str("2.0").unwrap());
        assert_eq!(account.held, Decimal::from_str("0.0").unwrap());
        assert_eq!(account.total, Decimal::from_str("2.0").unwrap());
        assert!(!account.locked);
    }

    #[test]
    fn manage_disputes() {
        let mut engine = PaymentEngine::new();

        // Deposit 10.0 to client 1
        engine
            .process_transaction(create_deposit(1, 1, "10.0"))
            .unwrap();

        // Dispute the deposit
        engine.process_transaction(create_dispute(1, 1)).unwrap();

        let accounts = engine.get_accounts();
        assert_eq!(accounts.len(), 1);

        let account = &accounts[0];
        assert_eq!(account.client, 1);
        assert_eq!(account.available, Decimal::from_str("0.0").unwrap());
        assert_eq!(account.held, Decimal::from_str("10.0").unwrap());
        assert_eq!(account.total, Decimal::from_str("10.0").unwrap());
        assert!(!account.locked);

        // Resolve the dispute
        engine.process_transaction(create_resolve(1, 1)).unwrap();

        let accounts = engine.get_accounts();
        let account = &accounts[0];
        assert_eq!(account.available, Decimal::from_str("10.0").unwrap());
        assert_eq!(account.held, Decimal::from_str("0.0").unwrap());
        assert_eq!(account.total, Decimal::from_str("10.0").unwrap());
        assert!(!account.locked);
    }

    #[test]
    fn no_dispute_bad_tx() {
        let mut engine = PaymentEngine::new();

        // Deposit 10.0 to client 1
        engine
            .process_transaction(create_deposit(1, 1, "10.0"))
            .unwrap();

        // Try to dispute a non-existent transaction
        engine.process_transaction(create_dispute(1, 999)).unwrap();

        let accounts = engine.get_accounts();
        assert_eq!(accounts.len(), 1);

        let account = &accounts[0];
        assert_eq!(account.client, 1);
        assert_eq!(account.available, Decimal::from_str("10.0").unwrap());
        assert_eq!(account.held, Decimal::from_str("0.0").unwrap());
        assert_eq!(account.total, Decimal::from_str("10.0").unwrap());
        assert!(!account.locked);
    }

    #[test]
    fn manage_multiple_disputes() {
        let mut engine = PaymentEngine::new();

        // Deposit 10.0 to client 1
        engine
            .process_transaction(create_deposit(1, 1, "10.0"))
            .unwrap();

        // Deposit 5.0 to client 1
        engine
            .process_transaction(create_deposit(1, 2, "5.0"))
            .unwrap();

        // Dispute both deposits
        engine.process_transaction(create_dispute(1, 1)).unwrap();
        engine.process_transaction(create_dispute(1, 2)).unwrap();

        let accounts = engine.get_accounts();
        assert_eq!(accounts.len(), 1);

        let account = &accounts[0];
        assert_eq!(account.client, 1);
        assert_eq!(account.available, Decimal::from_str("0.0").unwrap());
        assert_eq!(account.held, Decimal::from_str("15.0").unwrap());
        assert_eq!(account.total, Decimal::from_str("15.0").unwrap());
        assert!(!account.locked);

        // Resolve one dispute
        engine.process_transaction(create_resolve(1, 1)).unwrap();

        let accounts = engine.get_accounts();
        let account = &accounts[0];
        assert_eq!(account.available, Decimal::from_str("10.0").unwrap());
        assert_eq!(account.held, Decimal::from_str("5.0").unwrap());
        assert_eq!(account.total, Decimal::from_str("15.0").unwrap());
        assert!(!account.locked);
    }

    #[test]
    fn no_dispute_no_chargeback() {
        let mut engine = PaymentEngine::new();

        // Deposit 10.0 to client 1
        engine
            .process_transaction(create_deposit(1, 1, "10.0"))
            .unwrap();

        // Try to chargeback without dispute
        engine.process_transaction(create_chargeback(1, 1)).unwrap();

        let accounts = engine.get_accounts();
        assert_eq!(accounts.len(), 1);

        let account = &accounts[0];
        assert_eq!(account.client, 1);
        assert_eq!(account.available, Decimal::from_str("10.0").unwrap());
        assert_eq!(account.held, Decimal::from_str("0.0").unwrap());
        assert_eq!(account.total, Decimal::from_str("10.0").unwrap());
        assert!(!account.locked);
    }

    #[test]
    fn chargeback() {
        let mut engine = PaymentEngine::new();

        // Deposit 10.0 to client 1
        engine
            .process_transaction(create_deposit(1, 1, "10.0"))
            .unwrap();

        // Dispute the deposit
        engine.process_transaction(create_dispute(1, 1)).unwrap();

        // Chargeback the disputed transaction
        engine.process_transaction(create_chargeback(1, 1)).unwrap();

        let accounts = engine.get_accounts();
        assert_eq!(accounts.len(), 1);

        let account = &accounts[0];
        assert_eq!(account.client, 1);
        assert_eq!(account.available, Decimal::from_str("0.0").unwrap());
        assert_eq!(account.held, Decimal::from_str("0.0").unwrap());
        assert_eq!(account.total, Decimal::from_str("0.0").unwrap());
        assert!(account.locked);
    }

    #[test]
    fn no_further_withdraw_after_chargeback() {
        let mut engine = PaymentEngine::new();

        // Deposit 15.0 to client 1
        engine
            .process_transaction(create_deposit(1, 1, "15.0"))
            .unwrap();

        // Deposit 5.0 to client 1
        engine
            .process_transaction(create_deposit(1, 2, "5.0"))
            .unwrap();

        // Dispute the first deposit
        engine.process_transaction(create_dispute(1, 1)).unwrap();

        // Chargeback the disputed transaction (locks account)
        engine.process_transaction(create_chargeback(1, 1)).unwrap();

        // Try to withdraw from the locked account (should fail)
        engine
            .process_transaction(create_withdrawal(1, 3, "2.0"))
            .unwrap();

        let accounts = engine.get_accounts();
        assert_eq!(accounts.len(), 1);

        let account = &accounts[0];
        assert_eq!(account.client, 1);
        assert_eq!(account.available, Decimal::from_str("5.0").unwrap());
        assert_eq!(account.held, Decimal::from_str("0.0").unwrap());
        assert_eq!(account.total, Decimal::from_str("5.0").unwrap());
        assert!(account.locked);
    }

    #[test]
    fn disputes_possible_after_chargeback() {
        let mut engine = PaymentEngine::new();

        // Deposit 10.0 to client 1
        engine
            .process_transaction(create_deposit(1, 1, "10.0"))
            .unwrap();

        // Deposit 5.0 to client 1
        engine
            .process_transaction(create_deposit(1, 2, "5.0"))
            .unwrap();

        // Dispute the first deposit
        engine.process_transaction(create_dispute(1, 1)).unwrap();

        // Dispute the second deposit
        engine.process_transaction(create_dispute(1, 2)).unwrap();

        // Chargeback the first disputed transaction (locks account)
        engine.process_transaction(create_chargeback(1, 1)).unwrap();

        // Chargeback the second disputed transaction (should still work)
        engine.process_transaction(create_chargeback(1, 2)).unwrap();

        let accounts = engine.get_accounts();
        assert_eq!(accounts.len(), 1);

        let account = &accounts[0];
        assert_eq!(account.client, 1);
        assert_eq!(account.available, Decimal::from_str("0.0").unwrap());
        assert_eq!(account.held, Decimal::from_str("0.0").unwrap());
        assert_eq!(account.total, Decimal::from_str("0.0").unwrap());
        assert!(account.locked);
    }
}
