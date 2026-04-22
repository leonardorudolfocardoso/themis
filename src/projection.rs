use std::collections::HashMap;

use crate::account::Account;
use crate::id::{ClientId, TransactionId};
use crate::transaction::Record;

#[derive(Default)]
pub(crate) struct LedgerProjection {
    accounts: HashMap<ClientId, Account>,
    records: HashMap<TransactionId, Record>,
}

impl LedgerProjection {
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
