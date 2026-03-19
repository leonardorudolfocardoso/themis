use std::collections::HashMap;

use crate::transaction::{Transaction, TransactionRecord, TransactionState};

/// Monetary amounts are stored as integer units of 0.0001 (4 decimal places).
/// e.g. 1.2345 is represented as 12345.
pub struct Account {
    client: u16,
    available: i64,
    held: u64,
    total: u64,
    locked: bool,
}

impl Account {
    fn new(client: u16) -> Self {
        Self {
            client,
            available: 0,
            held: 0,
            total: 0,
            locked: false,
        }
    }

    pub fn client(&self) -> u16 {
        self.client
    }

    pub fn available(&self) -> i64 {
        self.available
    }

    pub fn held(&self) -> u64 {
        self.held
    }

    pub fn total(&self) -> u64 {
        self.total
    }

    pub fn locked(&self) -> bool {
        self.locked
    }
}

pub struct Processor {
    accounts: HashMap<u16, Account>,
    records: HashMap<u32, TransactionRecord>,
}

impl Processor {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            records: HashMap::new(),
        }
    }

    pub fn process(
        mut self,
        transactions: impl Iterator<Item = Transaction>,
    ) -> HashMap<u16, Account> {
        for transaction in transactions {
            match transaction {
                Transaction::Deposit { client, tx, amount } => {
                    if !self.records.contains_key(&tx) {
                        self.deposit(client, tx, amount)
                    }
                }
                Transaction::Withdrawal { client, tx, amount } => {
                    if !self.records.contains_key(&tx) {
                        self.withdraw(client, tx, amount)
                    }
                }
                Transaction::Dispute { client, tx } => self.dispute(client, tx),
                Transaction::Resolve { client, tx } => self.resolve(client, tx),
                Transaction::Chargeback { client, tx } => self.chargeback(client, tx),
            }
        }
        self.accounts
    }

    fn deposit(&mut self, client: u16, tx: u32, amount: u64) {
        let account = self
            .accounts
            .entry(client)
            .or_insert_with(|| Account::new(client));
        if !account.locked {
            account.available += amount as i64;
            account.total += amount;
            self.records.insert(
                tx,
                TransactionRecord {
                    client,
                    amount,
                    state: TransactionState::Valid,
                },
            );
        }
    }

    fn withdraw(&mut self, client: u16, tx: u32, amount: u64) {
        let account = self
            .accounts
            .entry(client)
            .or_insert_with(|| Account::new(client));
        if !account.locked && account.available >= amount as i64 {
            account.available -= amount as i64;
            account.total -= amount;
            self.records.insert(
                tx,
                TransactionRecord {
                    client,
                    amount,
                    state: TransactionState::Valid,
                },
            );
        }
    }

    fn dispute(&mut self, client: u16, tx: u32) {
        if let Some(record) = self.records.get_mut(&tx)
            && record.client == client
            && record.state.is_disputable()
            && let Some(account) = self.accounts.get_mut(&client)
        {
            account.available -= record.amount as i64;
            account.held += record.amount;
            record.state = TransactionState::Disputed;
        }
    }

    fn resolve(&mut self, client: u16, tx: u32) {
        if let Some(record) = self.records.get_mut(&tx)
            && record.client == client
            && record.state.is_resolvable()
            && let Some(account) = self.accounts.get_mut(&client)
        {
            account.available += record.amount as i64;
            account.held -= record.amount;
            record.state = TransactionState::Resolved;
        }
    }

    fn chargeback(&mut self, client: u16, tx: u32) {
        if let Some(record) = self.records.get_mut(&tx)
            && record.client == client
            && record.state.is_chargebackable()
            && let Some(account) = self.accounts.get_mut(&client)
        {
            account.held -= record.amount;
            account.total -= record.amount;
            account.locked = true;
            record.state = TransactionState::Chargedback;
        }
    }
}

#[cfg(test)]
mod test {
    use super::Processor;
    use super::Account;
    use crate::transaction::Transaction;

    fn process(transactions: Vec<Transaction>) -> HashMap<u16, Account> {
        Processor::new().process(transactions.into_iter())
    }

    use std::collections::HashMap;

    #[test]
    fn test_deposit_increases_available_and_total() {
        let accounts = process(vec![Transaction::Deposit {
            client: 1,
            tx: 1,
            amount: 100,
        }]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), 100);
        assert_eq!(account.total(), 100);
        assert_eq!(account.held(), 0);
        assert!(!account.locked());
    }

    #[test]
    fn test_duplicate_tx_id_is_ignored() {
        let accounts = process(vec![
            Transaction::Deposit { client: 1, tx: 1, amount: 100 },
            Transaction::Deposit { client: 1, tx: 1, amount: 999 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), 100);
        assert_eq!(account.total(), 100);
    }

    #[test]
    fn test_duplicate_withdrawal_tx_id_is_ignored() {
        let accounts = process(vec![
            Transaction::Deposit { client: 1, tx: 1, amount: 100 },
            Transaction::Withdrawal { client: 1, tx: 2, amount: 20 },
            Transaction::Withdrawal { client: 1, tx: 2, amount: 20 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), 80);
        assert_eq!(account.total(), 80);
    }

    #[test]
    fn test_deposit_on_locked_account_is_ignored() {
        let accounts = process(vec![
            Transaction::Deposit { client: 1, tx: 1, amount: 100 },
            Transaction::Dispute { client: 1, tx: 1 },
            Transaction::Chargeback { client: 1, tx: 1 },
            Transaction::Deposit { client: 1, tx: 2, amount: 50 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), 0);
        assert_eq!(account.total(), 0);
        assert!(account.locked());
    }

    #[test]
    fn test_withdraw_decreases_available_and_total() {
        let accounts = process(vec![
            Transaction::Deposit { client: 1, tx: 1, amount: 100 },
            Transaction::Withdrawal { client: 1, tx: 2, amount: 20 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), 80);
        assert_eq!(account.total(), 80);
        assert_eq!(account.held(), 0);
    }

    #[test]
    fn test_withdraw_insufficient_funds_is_silently_ignored() {
        let accounts = process(vec![
            Transaction::Deposit { client: 1, tx: 1, amount: 100 },
            Transaction::Withdrawal { client: 1, tx: 2, amount: 200 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), 100);
        assert_eq!(account.total(), 100);
    }

    #[test]
    fn test_withdraw_locked_is_silently_ignored() {
        let accounts = process(vec![
            Transaction::Deposit { client: 1, tx: 1, amount: 100 },
            Transaction::Dispute { client: 1, tx: 1 },
            Transaction::Chargeback { client: 1, tx: 1 },
            Transaction::Withdrawal { client: 1, tx: 2, amount: 50 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.total(), 0);
        assert!(account.locked());
    }

    #[test]
    fn test_dispute_decreases_available_and_increases_held() {
        let accounts = process(vec![
            Transaction::Deposit { client: 1, tx: 1, amount: 100 },
            Transaction::Dispute { client: 1, tx: 1 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), 0);
        assert_eq!(account.held(), 100);
        assert_eq!(account.total(), 100);
    }

    #[test]
    fn test_dispute_on_wrong_client_is_ignored() {
        let accounts = process(vec![
            Transaction::Deposit { client: 1, tx: 1, amount: 100 },
            Transaction::Dispute { client: 2, tx: 1 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), 100);
        assert_eq!(account.held(), 0);
    }

    #[test]
    fn test_dispute_on_invalid_state_is_ignored() {
        let accounts = process(vec![
            Transaction::Deposit { client: 1, tx: 1, amount: 100 },
            Transaction::Dispute { client: 1, tx: 1 },
            Transaction::Dispute { client: 1, tx: 1 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), 0);
        assert_eq!(account.held(), 100);
    }

    #[test]
    fn test_resolve_decreases_held_and_increases_available() {
        let accounts = process(vec![
            Transaction::Deposit { client: 1, tx: 1, amount: 100 },
            Transaction::Dispute { client: 1, tx: 1 },
            Transaction::Resolve { client: 1, tx: 1 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), 100);
        assert_eq!(account.held(), 0);
        assert_eq!(account.total(), 100);
    }

    #[test]
    fn test_resolve_without_dispute_is_ignored() {
        let accounts = process(vec![
            Transaction::Deposit { client: 1, tx: 1, amount: 100 },
            Transaction::Resolve { client: 1, tx: 1 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), 100);
        assert_eq!(account.held(), 0);
    }

    #[test]
    fn test_resolve_on_wrong_client_is_ignored() {
        let accounts = process(vec![
            Transaction::Deposit { client: 1, tx: 1, amount: 100 },
            Transaction::Dispute { client: 1, tx: 1 },
            Transaction::Resolve { client: 2, tx: 1 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), 0);
        assert_eq!(account.held(), 100);
    }

    #[test]
    fn test_resolve_on_already_resolved_is_ignored() {
        let accounts = process(vec![
            Transaction::Deposit { client: 1, tx: 1, amount: 100 },
            Transaction::Dispute { client: 1, tx: 1 },
            Transaction::Resolve { client: 1, tx: 1 },
            Transaction::Resolve { client: 1, tx: 1 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), 100);
        assert_eq!(account.held(), 0);
    }

    #[test]
    fn test_chargeback_locks_account_and_decreases_held_and_total() {
        let accounts = process(vec![
            Transaction::Deposit { client: 1, tx: 1, amount: 100 },
            Transaction::Dispute { client: 1, tx: 1 },
            Transaction::Chargeback { client: 1, tx: 1 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), 0);
        assert_eq!(account.held(), 0);
        assert_eq!(account.total(), 0);
        assert!(account.locked());
    }

    #[test]
    fn test_chargeback_without_dispute_is_ignored() {
        let accounts = process(vec![
            Transaction::Deposit { client: 1, tx: 1, amount: 100 },
            Transaction::Chargeback { client: 1, tx: 1 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert!(!account.locked());
    }

    #[test]
    fn test_chargeback_on_wrong_client_is_ignored() {
        let accounts = process(vec![
            Transaction::Deposit { client: 1, tx: 1, amount: 100 },
            Transaction::Dispute { client: 1, tx: 1 },
            Transaction::Chargeback { client: 2, tx: 1 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.held(), 100);
        assert!(!account.locked());
    }

    #[test]
    fn test_chargeback_on_already_chargedback_is_ignored() {
        let accounts = process(vec![
            Transaction::Deposit { client: 1, tx: 1, amount: 100 },
            Transaction::Dispute { client: 1, tx: 1 },
            Transaction::Chargeback { client: 1, tx: 1 },
            Transaction::Chargeback { client: 1, tx: 1 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.held(), 0);
        assert_eq!(account.total(), 0);
        assert!(account.locked());
    }
}
