pub enum Transaction {
    Deposit { client: u16, tx: u32, amount: u64 },
    Withdrawal { client: u16, tx: u32, amount: u64 },
    Dispute { client: u16, tx: u32 },
    Resolve { client: u16, tx: u32 },
    Chargeback { client: u16, tx: u32 },
}

pub(crate) enum TransactionState {
    Valid,
    Disputed,
    Resolved,
    Chargedback,
}

impl TransactionState {
    pub(crate) fn is_disputable(&self) -> bool {
        matches!(self, TransactionState::Valid)
    }

    pub(crate) fn is_resolvable(&self) -> bool {
        matches!(self, TransactionState::Disputed)
    }

    pub(crate) fn is_chargebackable(&self) -> bool {
        matches!(self, TransactionState::Disputed)
    }
}

pub(crate) struct TransactionRecord {
    pub(crate) client: u16,
    pub(crate) amount: u64,
    pub(crate) state: TransactionState,
}
