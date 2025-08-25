use rust_decimal::Decimal;
use serde::Serialize;

#[derive(Debug, Clone, Default)]
pub struct Account {
    pub available: Decimal,
    pub held: Decimal,
    pub locked: bool,
}

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
