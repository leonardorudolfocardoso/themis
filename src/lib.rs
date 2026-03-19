use std::collections::HashMap;

/// Monetary amounts are stored as integer units of 0.0001 (4 decimal places).
/// e.g. 1.2345 is represented as 12345.
pub struct Account {
    pub client: u16,
    pub available: i64,
    pub held: u64,
    pub total: u64,
    pub locked: bool,
}

impl Account {
    pub fn new(client: u16) -> Self {
        Self {
            client,
            available: 0,
            held: 0,
            total: 0,
            locked: false,
        }
    }
}

pub enum Transaction {
    Deposit { client: u16, tx: u32, amount: u64 },
    Withdrawal { client: u16, tx: u32, amount: u64 },
    Dispute { client: u16, tx: u32 },
    Resolve { client: u16, tx: u32 },
    Chargeback { client: u16, tx: u32 },
}

enum TransactionState {
    Valid,
    Disputed,
    Resolved,
    Chargedback,
}

impl TransactionState {
    fn is_disputable(&self) -> bool {
        matches!(self, TransactionState::Valid)
    }

    fn is_resolvable(&self) -> bool {
        matches!(self, TransactionState::Disputed)
    }

    fn is_chargebackable(&self) -> bool {
        matches!(self, TransactionState::Disputed)
    }
}

struct TransactionRecord {
    client: u16,
    amount: u64,
    state: TransactionState,
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

    pub fn process(&mut self, transactions: impl Iterator<Item = Transaction>) {
        for transaction in transactions {
            match transaction {
                Transaction::Deposit { client, tx, amount } => self.deposit(client, tx, amount),
                Transaction::Withdrawal { client, tx, amount } => self.withdraw(client, tx, amount),
                Transaction::Dispute { client, tx } => self.dispute(client, tx),
                Transaction::Resolve { client, tx } => self.resolve(client, tx),
                Transaction::Chargeback { client, tx } => self.chargeback(client, tx),
            }
        }
    }

    pub fn accounts(&self) -> &HashMap<u16, Account> {
        &self.accounts
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
        todo!()
    }

    fn chargeback(&mut self, client: u16, tx: u32) {
        todo!()
    }
}

#[cfg(test)]
mod test {
    use super::{Processor, Transaction};

    fn setup() -> Processor {
        let mut processor = Processor::new();
        processor.process(
            vec![Transaction::Deposit {
                client: 1,
                tx: 1,
                amount: 100,
            }]
            .into_iter(),
        );
        processor
    }

    #[test]
    fn test_deposit_increases_available_and_total() {
        let processor = setup();
        let account = processor.accounts().get(&1).unwrap();
        assert_eq!(account.available, 100);
        assert_eq!(account.total, 100);
        assert_eq!(account.held, 0);
        assert!(!account.locked);
    }

    #[test]
    fn test_withdraw_decreases_available_and_total() {
        let mut processor = setup();
        processor.withdraw(1, 2, 20);
        let account = processor.accounts().get(&1).unwrap();
        assert_eq!(account.available, 80);
        assert_eq!(account.total, 80);
        assert_eq!(account.held, 0);
    }

    #[test]
    fn test_withdraw_insufficient_funds_is_silently_ignored() {
        let mut processor = setup();
        processor.withdraw(1, 2, 200);
        let account = processor.accounts().get(&1).unwrap();
        assert_eq!(account.available, 100);
        assert_eq!(account.total, 100);
    }

    #[test]
    fn test_withdraw_locked_is_silently_ignored() {
        let mut processor = setup();
        processor.dispute(1, 1);
        processor.chargeback(1, 1);
        processor.withdraw(1, 2, 50);
        let account = processor.accounts().get(&1).unwrap();
        assert_eq!(account.total, 0);
        assert!(account.locked);
    }

    #[test]
    fn test_dispute_decreases_available_and_increases_held() {
        let mut processor = setup();
        processor.dispute(1, 1);
        let account = processor.accounts().get(&1).unwrap();
        assert_eq!(account.available, 0);
        assert_eq!(account.held, 100);
        assert_eq!(account.total, 100);
    }

    #[test]
    fn test_dispute_on_wrong_client_is_ignored() {
        let mut processor = setup();
        processor.dispute(2, 1);
        let account = processor.accounts().get(&1).unwrap();
        assert_eq!(account.available, 100);
        assert_eq!(account.held, 0);
    }

    #[test]
    fn test_dispute_on_invalid_state_is_ignored() {
        let mut processor = setup();
        processor.dispute(1, 1);
        processor.dispute(1, 1);
        let account = processor.accounts().get(&1).unwrap();
        assert_eq!(account.available, 0);
        assert_eq!(account.held, 100);
    }

    #[test]
    fn test_deposit_on_locked_account_is_ignored() {
        let mut processor = setup();
        processor.dispute(1, 1);
        processor.chargeback(1, 1);
        processor.deposit(1, 2, 50);
        let account = processor.accounts().get(&1).unwrap();
        assert_eq!(account.available, 0);
        assert_eq!(account.total, 0);
        assert!(account.locked);
    }
}
