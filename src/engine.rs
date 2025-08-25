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
