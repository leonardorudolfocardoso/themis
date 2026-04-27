use std::collections::HashMap;

use crate::Event;
use crate::account::Account;
use crate::amount::Amount;
use crate::id::{ClientId, TransactionId};
use crate::transaction::{Kind, Record, State};

#[derive(Default)]
struct AccountProjection {
    accounts: HashMap<ClientId, Account>,
}

impl AccountProjection {
    fn account(&self, client: ClientId) -> Option<&Account> {
        self.accounts.get(&client)
    }

    fn deposit(&mut self, client: ClientId, amount: Amount) {
        let account = self
            .accounts
            .entry(client)
            .or_insert_with(|| Account::new(client));

        account
            .deposit(amount)
            .expect("Event::Deposit must target an unlocked account");
    }

    fn withdrawal(&mut self, client: ClientId, amount: Amount) {
        let account = self
            .accounts
            .get_mut(&client)
            .expect("Event::Withdrawal must target an existing account");

        account
            .withdraw(amount)
            .expect("Event::Withdrawal must target an account with enough funds");
    }

    fn dispute(&mut self, client: ClientId, amount: Amount) {
        let account = self
            .accounts
            .get_mut(&client)
            .expect("DepositDisputed must target an existing account");

        account
            .hold(amount)
            .expect("DepositDisputed must target an unlocked account");
    }

    fn resolve(&mut self, client: ClientId, amount: Amount) {
        let account = self
            .accounts
            .get_mut(&client)
            .expect("DisputeResolved must target an existing account");

        account
            .release(amount)
            .expect("DisputeResolved must target an unlocked account");
    }

    fn chargeback(&mut self, client: ClientId, amount: Amount) {
        let account = self
            .accounts
            .get_mut(&client)
            .expect("DepositChargedBack must target an existing account");

        account
            .chargeback(amount)
            .expect("DepositChargedBack must target an unlocked account");
    }

    fn into_accounts(self) -> HashMap<ClientId, Account> {
        self.accounts
    }
}

#[derive(Default)]
struct TransactionProjection {
    records: HashMap<TransactionId, Record>,
}

impl TransactionProjection {
    fn has_record(&self, tx: TransactionId) -> bool {
        self.records.contains_key(&tx)
    }

    fn record(&self, tx: TransactionId) -> Option<&Record> {
        self.records.get(&tx)
    }

    fn deposit(&mut self, client: ClientId, tx: TransactionId, amount: Amount) {
        self.records.insert(
            tx,
            Record {
                client,
                amount,
                kind: Kind::Deposit,
                state: State::Valid,
            },
        );
    }

    fn withdrawal(&mut self, client: ClientId, tx: TransactionId, amount: Amount) {
        self.records.insert(
            tx,
            Record {
                client,
                amount,
                kind: Kind::Withdrawal,
                state: State::Valid,
            },
        );
    }

    fn dispute(&mut self, tx: TransactionId) {
        let record = self
            .records
            .get_mut(&tx)
            .expect("DepositDisputed must target an existing transaction");

        record.state = State::Disputed;
    }

    fn resolve(&mut self, tx: TransactionId) {
        let record = self
            .records
            .get_mut(&tx)
            .expect("DisputeResolved must target an existing transaction");

        record.state = State::Resolved;
    }

    fn chargeback(&mut self, tx: TransactionId) {
        let record = self
            .records
            .get_mut(&tx)
            .expect("DepositChargedBack must target an existing transaction");

        record.state = State::Chargedback;
    }
}

#[derive(Default)]
pub(crate) struct LedgerProjection {
    accounts: AccountProjection,
    transactions: TransactionProjection,
}

impl LedgerProjection {
    pub(crate) fn apply(&mut self, event: Event) {
        match event {
            Event::Deposit { client, tx, amount } => {
                self.accounts.deposit(client, amount);
                self.transactions.deposit(client, tx, amount);
            }
            Event::Withdrawal { client, tx, amount } => {
                self.accounts.withdrawal(client, amount);
                self.transactions.withdrawal(client, tx, amount);
            }
            Event::DepositDisputed { client, tx, amount } => {
                self.accounts.dispute(client, amount);
                self.transactions.dispute(tx);
            }
            Event::DisputeResolved { client, tx, amount } => {
                self.accounts.resolve(client, amount);
                self.transactions.resolve(tx);
            }
            Event::DepositChargedBack { client, tx, amount } => {
                self.accounts.chargeback(client, amount);
                self.transactions.chargeback(tx);
            }
        }
    }

    pub(crate) fn account(&self, client: ClientId) -> Option<&Account> {
        self.accounts.account(client)
    }

    pub(crate) fn has_record(&self, tx: TransactionId) -> bool {
        self.transactions.has_record(tx)
    }

    pub(crate) fn record(&self, tx: TransactionId) -> Option<&Record> {
        self.transactions.record(tx)
    }

    pub(crate) fn into_accounts(self) -> HashMap<ClientId, Account> {
        self.accounts.into_accounts()
    }
}

#[cfg(test)]
mod test {
    use super::{AccountProjection, LedgerProjection, TransactionProjection};
    use crate::amount::Amount;
    use crate::event::Event;
    use crate::transaction::{Kind, State};

    fn deposit_event(client: u16, tx: u32, amount: u64) -> Event {
        Event::Deposit {
            client,
            tx,
            amount: Amount::raw(amount),
        }
    }

    fn withdrawal_event(client: u16, tx: u32, amount: u64) -> Event {
        Event::Withdrawal {
            client,
            tx,
            amount: Amount::raw(amount),
        }
    }

    fn account_projection_with(actions: impl FnOnce(&mut AccountProjection)) -> AccountProjection {
        let mut projection = AccountProjection::default();
        actions(&mut projection);
        projection
    }

    fn transaction_projection_with(
        actions: impl FnOnce(&mut TransactionProjection),
    ) -> TransactionProjection {
        let mut projection = TransactionProjection::default();
        actions(&mut projection);
        projection
    }

    fn ledger_projection_with(events: &[Event]) -> LedgerProjection {
        let mut projection = LedgerProjection::default();
        for event in events {
            projection.apply(*event);
        }
        projection
    }

    #[test]
    fn test_account_projection_deposit_creates_account_and_increases_available() {
        let projection = account_projection_with(|projection| {
            projection.deposit(1, Amount::raw(100));
        });

        let account = projection.account(1).expect("account not found");
        assert_eq!(account.available(), 100);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), 100);
    }

    #[test]
    fn test_account_projection_withdrawal_decreases_available() {
        let projection = account_projection_with(|projection| {
            projection.deposit(1, Amount::raw(100));
            projection.withdrawal(1, Amount::raw(40));
        });

        let account = projection.account(1).expect("account not found");
        assert_eq!(account.available(), 60);
        assert_eq!(account.total(), 60);
    }

    #[test]
    fn test_account_projection_dispute_and_resolve_move_funds_between_available_and_held() {
        let projection = account_projection_with(|projection| {
            projection.deposit(1, Amount::raw(100));
            projection.dispute(1, Amount::raw(100));
            projection.resolve(1, Amount::raw(100));
        });

        let account = projection.account(1).expect("account not found");
        assert_eq!(account.available(), 100);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), 100);
    }

    #[test]
    fn test_account_projection_chargeback_after_withdrawal_can_make_total_negative() {
        let projection = account_projection_with(|projection| {
            projection.deposit(1, Amount::raw(100));
            projection.withdrawal(1, Amount::raw(80));
            projection.dispute(1, Amount::raw(100));
            projection.chargeback(1, Amount::raw(100));
        });

        let account = projection.account(1).expect("account not found");
        assert_eq!(account.available(), -80);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), -80);
        assert!(account.locked());
    }

    #[test]
    fn test_transaction_projection_records_deposit_and_withdrawal_kinds() {
        let projection = transaction_projection_with(|projection| {
            projection.deposit(1, 1, Amount::raw(100));
            projection.withdrawal(1, 2, Amount::raw(40));
        });

        let deposit = projection.record(1).expect("deposit record not found");
        assert_eq!(deposit.client, 1);
        assert_eq!(deposit.amount, Amount::raw(100));
        assert!(matches!(deposit.kind, Kind::Deposit));
        assert!(matches!(deposit.state, State::Valid));

        let withdrawal = projection.record(2).expect("withdrawal record not found");
        assert_eq!(withdrawal.client, 1);
        assert_eq!(withdrawal.amount, Amount::raw(40));
        assert!(matches!(withdrawal.kind, Kind::Withdrawal));
        assert!(matches!(withdrawal.state, State::Valid));
    }

    #[test]
    fn test_transaction_projection_tracks_dispute_lifecycle() {
        let projection = transaction_projection_with(|projection| {
            projection.deposit(1, 1, Amount::raw(100));
            projection.dispute(1);
            projection.resolve(1);
        });

        let record = projection.record(1).expect("record not found");
        assert!(matches!(record.state, State::Resolved));
    }

    #[test]
    fn test_transaction_projection_tracks_chargeback_lifecycle() {
        let projection = transaction_projection_with(|projection| {
            projection.deposit(1, 1, Amount::raw(100));
            projection.dispute(1);
            projection.chargeback(1);
        });

        let record = projection.record(1).expect("record not found");
        assert!(matches!(record.state, State::Chargedback));
    }

    #[test]
    fn test_ledger_projection_applies_events_to_both_subprojections() {
        let projection =
            ledger_projection_with(&[deposit_event(1, 1, 100), withdrawal_event(1, 2, 40)]);

        let account = projection.account(1).expect("account not found");
        assert_eq!(account.available(), 60);
        assert_eq!(account.total(), 60);

        let record = projection.record(2).expect("record not found");
        assert!(matches!(record.kind, Kind::Withdrawal));
        assert!(matches!(record.state, State::Valid));
    }
}
