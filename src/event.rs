use crate::amount::Amount;

pub enum Event {
    Deposit { client: u16, tx: u32, amount: Amount },
    Withdrawal { client: u16, tx: u32, amount: Amount },
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

pub(crate) enum TransactionKind {
    Deposit,
    Withdrawal,
}

pub(crate) struct TransactionRecord {
    pub(crate) client: u16,
    pub(crate) amount: Amount,
    pub(crate) kind: TransactionKind,
    pub(crate) state: TransactionState,
}

impl TransactionRecord {
    pub(crate) fn is_disputable(&self) -> bool {
        matches!(self.kind, TransactionKind::Deposit) && self.state.is_disputable()
    }
}
