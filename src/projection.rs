use std::collections::HashMap;

use crate::Event;
use crate::account::Account;
use crate::id::{ClientId, TransactionId};
use crate::transaction::{Kind, Record, State};

#[derive(Default)]
pub(crate) struct LedgerProjection {
    accounts: HashMap<ClientId, Account>,
    records: HashMap<TransactionId, Record>,
}

impl LedgerProjection {
    pub(crate) fn apply(&mut self, event: Event) {
        match event {
            Event::DepositAccepted { client, tx, amount } => {
                let account = self.account_mut_or_create(client);
                account
                    .deposit(amount)
                    .expect("DepositAccepted must target an unlocked account");

                self.insert_record(
                    tx,
                    Record {
                        client,
                        amount,
                        kind: Kind::Deposit,
                        state: State::Valid,
                    },
                );
            }
            Event::WithdrawalAccepted { client, tx, amount } => {
                let account = self
                    .account_mut(client)
                    .expect("WithdrawalAccepted must target an existing account");

                account
                    .withdraw(amount)
                    .expect("WithdrawalAccepted must target an account with enough funds");

                self.insert_record(
                    tx,
                    Record {
                        client,
                        amount,
                        kind: Kind::Withdrawal,
                        state: State::Valid,
                    },
                );
            }
            Event::DepositDisputed { client, tx, amount } => {
                let account = self
                    .account_mut(client)
                    .expect("DepositDisputed must target an existing account");

                account
                    .hold(amount)
                    .expect("DepositDisputed must target an unlocked account");

                let record = self
                    .record_mut(tx)
                    .expect("DepositDisputed must target an existing transaction");

                record.state = State::Disputed;
            }
            Event::DisputeResolved { client, tx, amount } => {
                let account = self
                    .account_mut(client)
                    .expect("DisputeResolved must target an existing account");

                account
                    .release(amount)
                    .expect("DisputeResolved must target an unlocked account");

                let record = self
                    .record_mut(tx)
                    .expect("DisputeResolved must target an existing transaction");

                record.state = State::Resolved;
            }
            Event::DepositChargedBack { client, tx, amount } => {
                let account = self
                    .account_mut(client)
                    .expect("DepositChargedBack must target an existing account");

                account
                    .chargeback(amount)
                    .expect("DepositChargedBack must target an unlocked account");

                let record = self
                    .record_mut(tx)
                    .expect("DepositChargedBack must target an existing transaction");

                record.state = State::Chargedback;
            }
        }
    }
    pub(crate) fn account(&self, client: ClientId) -> Option<&Account> {
        self.accounts.get(&client)
    }

    pub(crate) fn account_mut(&mut self, client: ClientId) -> Option<&mut Account> {
        self.accounts.get_mut(&client)
    }

    pub(crate) fn account_mut_or_create(&mut self, client: ClientId) -> &mut Account {
        self.accounts
            .entry(client)
            .or_insert_with(|| Account::new(client))
    }

    pub(crate) fn has_record(&self, tx: TransactionId) -> bool {
        self.records.contains_key(&tx)
    }

    pub(crate) fn record(&self, tx: TransactionId) -> Option<&Record> {
        self.records.get(&tx)
    }

    pub(crate) fn record_mut(&mut self, tx: TransactionId) -> Option<&mut Record> {
        self.records.get_mut(&tx)
    }

    pub(crate) fn insert_record(&mut self, tx: TransactionId, record: Record) {
        self.records.insert(tx, record);
    }

    pub(crate) fn into_accounts(self) -> HashMap<ClientId, Account> {
        self.accounts
    }
}

#[cfg(test)]
mod test {
    use super::LedgerProjection;
    use crate::amount::Amount;
    use crate::event::Event;
    use crate::transaction::{Kind, State};

    fn deposit_accepted(client: u16, tx: u32, amount: u64) -> Event {
        Event::DepositAccepted {
            client,
            tx,
            amount: Amount::raw(amount),
        }
    }

    fn withdrawal_accepted(client: u16, tx: u32, amount: u64) -> Event {
        Event::WithdrawalAccepted {
            client,
            tx,
            amount: Amount::raw(amount),
        }
    }

    fn deposit_disputed(client: u16, tx: u32, amount: u64) -> Event {
        Event::DepositDisputed {
            client,
            tx,
            amount: Amount::raw(amount),
        }
    }

    fn dispute_resolved(client: u16, tx: u32, amount: u64) -> Event {
        Event::DisputeResolved {
            client,
            tx,
            amount: Amount::raw(amount),
        }
    }

    fn deposit_charged_back(client: u16, tx: u32, amount: u64) -> Event {
        Event::DepositChargedBack {
            client,
            tx,
            amount: Amount::raw(amount),
        }
    }

    fn projection_with(events: &[Event]) -> LedgerProjection {
        let mut projection = LedgerProjection::default();
        for event in events {
            projection.apply(*event);
        }
        projection
    }

    #[test]
    fn test_deposit_accepted_creates_account_and_deposit_record() {
        let projection = projection_with(&[deposit_accepted(1, 1, 100)]);

        let account = projection.account(1).expect("account not found");
        assert_eq!(account.available(), 100);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), 100);

        let record = projection.record(1).expect("record not found");
        assert_eq!(record.client, 1);
        assert_eq!(record.amount, Amount::raw(100));
        assert!(matches!(record.kind, Kind::Deposit));
        assert!(matches!(record.state, State::Valid));
    }

    #[test]
    fn test_withdrawal_accepted_decreases_available_and_creates_withdrawal_record() {
        let projection =
            projection_with(&[deposit_accepted(1, 1, 100), withdrawal_accepted(1, 2, 40)]);

        let account = projection.account(1).expect("account not found");
        assert_eq!(account.available(), 60);
        assert_eq!(account.total(), 60);

        let record = projection.record(2).expect("record not found");
        assert_eq!(record.client, 1);
        assert_eq!(record.amount, Amount::raw(40));
        assert!(matches!(record.kind, Kind::Withdrawal));
        assert!(matches!(record.state, State::Valid));
    }

    #[test]
    fn test_deposit_disputed_moves_available_to_held_and_marks_record_disputed() {
        let projection =
            projection_with(&[deposit_accepted(1, 1, 100), deposit_disputed(1, 1, 100)]);

        let account = projection.account(1).expect("account not found");
        assert_eq!(account.available(), 0);
        assert_eq!(account.held(), Amount::raw(100));
        assert_eq!(account.total(), 100);

        let record = projection.record(1).expect("record not found");
        assert!(matches!(record.state, State::Disputed));
    }

    #[test]
    fn test_dispute_resolved_moves_held_to_available_and_marks_record_resolved() {
        let projection = projection_with(&[
            deposit_accepted(1, 1, 100),
            deposit_disputed(1, 1, 100),
            dispute_resolved(1, 1, 100),
        ]);

        let account = projection.account(1).expect("account not found");
        assert_eq!(account.available(), 100);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), 100);

        let record = projection.record(1).expect("record not found");
        assert!(matches!(record.state, State::Resolved));
    }

    #[test]
    fn test_deposit_charged_back_removes_held_locks_account_and_marks_record_chargedback() {
        let projection = projection_with(&[
            deposit_accepted(1, 1, 100),
            deposit_disputed(1, 1, 100),
            deposit_charged_back(1, 1, 100),
        ]);

        let account = projection.account(1).expect("account not found");
        assert_eq!(account.available(), 0);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), 0);
        assert!(account.locked());

        let record = projection.record(1).expect("record not found");
        assert!(matches!(record.state, State::Chargedback));
    }

    #[test]
    fn test_chargeback_after_withdrawal_can_make_total_negative() {
        let projection = projection_with(&[
            deposit_accepted(1, 1, 100),
            withdrawal_accepted(1, 2, 80),
            deposit_disputed(1, 1, 100),
            deposit_charged_back(1, 1, 100),
        ]);

        let account = projection.account(1).expect("account not found");
        assert_eq!(account.available(), -80);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), -80);
        assert!(account.locked());
    }
}
