use std::collections::HashMap;

use crate::{account::Account, transactions::StoredTransaction};

pub struct PaymentEngine {
    accounts: HashMap<u16, Account>,
    transactions: HashMap<u32, StoredTransaction>,
}
