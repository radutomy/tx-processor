use rust_decimal::Decimal;
use serde::Serialize;

#[derive(Debug, Clone, Default)]
pub struct Account {
    pub available: Decimal,
    pub held: Decimal,
    pub locked: bool,
}

/// Notes on chargebacks and locking:
/// - Multiple transactions can be disputed and later charged back. On the first chargeback
///   we lock the account (per spec), but still allow chargebacks to complete for transactions
///   that were already under dispute before the lock.
/// - We don’t track a separate list or count of chargebacks. Locking is a boolean that becomes
///   true after the first chargeback. Per-transaction state is tracked via the disputed flag,
///   and a chargeback clears that flag to prevent double-chargeback of the same tx.
impl Account {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn total(&self) -> Decimal {
        self.available + self.held
    }

    pub fn deposit(&mut self, amount: Decimal) {
        if !self.locked {
            self.available += amount;
        }
    }

    pub fn withdraw(&mut self, amount: Decimal) -> bool {
        if !self.locked && self.available >= amount {
            self.available -= amount;
            true
        } else {
            false
        }
    }

    pub fn hold_funds(&mut self, amount: Decimal) {
        if !self.locked && self.available >= amount {
            self.available -= amount;
            self.held += amount;
        }
    }

    pub fn release_funds(&mut self, amount: Decimal) {
        if !self.locked && self.held >= amount {
            self.held -= amount;
            self.available += amount;
        }
    }

    pub fn chargeback(&mut self, amount: Decimal) {
        if self.held >= amount {
            self.held -= amount;
            self.locked = true;
        }
    }
}

// Output format for CSV
#[derive(Debug, Serialize)]
pub struct AccountOutput {
    pub client: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
}

impl AccountOutput {
    pub fn from_account(client: u16, account: &Account) -> Self {
        Self {
            client,
            available: account.available.round_dp(4),
            held: account.held.round_dp(4),
            total: account.total().round_dp(4),
            locked: account.locked,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_deposit_and_withdraw() {
        let mut account = Account::new();
        account.deposit(Decimal::from_str("10.0").unwrap());

        assert_eq!(account.available, Decimal::from_str("10.0").unwrap());
        assert_eq!(account.total(), Decimal::from_str("10.0").unwrap());

        let result = account.withdraw(Decimal::from_str("5.0").unwrap());
        assert!(result);
        assert_eq!(account.available, Decimal::from_str("5.0").unwrap());
    }

    #[test]
    fn test_withdraw_insufficient_funds() {
        let mut account = Account::new();
        account.deposit(Decimal::from_str("5.0").unwrap());

        let result = account.withdraw(Decimal::from_str("10.0").unwrap());
        assert!(!result);
        assert_eq!(account.available, Decimal::from_str("5.0").unwrap());
    }

    #[test]
    fn test_hold_and_release_funds() {
        let mut account = Account::new();
        account.deposit(Decimal::from_str("10.0").unwrap());

        account.hold_funds(Decimal::from_str("3.0").unwrap());
        assert_eq!(account.available, Decimal::from_str("7.0").unwrap());
        assert_eq!(account.held, Decimal::from_str("3.0").unwrap());
        assert_eq!(account.total(), Decimal::from_str("10.0").unwrap());

        account.release_funds(Decimal::from_str("2.0").unwrap());
        assert_eq!(account.available, Decimal::from_str("9.0").unwrap());
        assert_eq!(account.held, Decimal::from_str("1.0").unwrap());
    }

    #[test]
    fn test_chargeback_locks_account() {
        let mut account = Account::new();
        account.deposit(Decimal::from_str("10.0").unwrap());
        account.hold_funds(Decimal::from_str("5.0").unwrap());

        account.chargeback(Decimal::from_str("5.0").unwrap());

        assert_eq!(account.available, Decimal::from_str("5.0").unwrap());
        assert_eq!(account.held, Decimal::from_str("0").unwrap());
        assert_eq!(account.total(), Decimal::from_str("5.0").unwrap());
        assert!(account.locked);
    }

    #[test]
    fn test_locked_account_blocks_operations() {
        let mut account = Account::new();
        account.deposit(Decimal::from_str("10.0").unwrap());
        account.hold_funds(Decimal::from_str("5.0").unwrap());
        account.chargeback(Decimal::from_str("5.0").unwrap());

        // Operations should be blocked on locked account
        account.deposit(Decimal::from_str("1.0").unwrap());
        assert_eq!(account.available, Decimal::from_str("5.0").unwrap()); // No change

        let withdraw_result = account.withdraw(Decimal::from_str("1.0").unwrap());
        assert!(!withdraw_result);
    }

    #[test]
    fn test_account_output_formatting() {
        let mut account = Account::new();
        account.deposit(Decimal::from_str("10.123456").unwrap());
        account.hold_funds(Decimal::from_str("2.5678").unwrap());

        let output = AccountOutput::from_account(123, &account);

        assert_eq!(output.client, 123);
        assert_eq!(output.available, Decimal::from_str("7.5557").unwrap());
        assert_eq!(output.held, Decimal::from_str("2.5678").unwrap());
        assert_eq!(output.total, Decimal::from_str("10.1235").unwrap());
        assert!(!output.locked);
    }
}
