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
            Event::DisputeResolved { .. } | Event::DepositChargedBack { .. } => {
                unreachable!(
                    "only deposits, withdrawals, and disputes are projected through events yet"
                )
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
