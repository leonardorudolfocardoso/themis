use crate::{
    amount::Amount,
    id::{ClientId, TransactionId},
};

pub enum Event {
    Deposit {
        client: ClientId,
        tx: TransactionId,
        amount: Amount,
    },
    Withdrawal {
        client: ClientId,
        tx: TransactionId,
        amount: Amount,
    },
    Dispute {
        client: ClientId,
        tx: TransactionId,
    },
    Resolve {
        client: ClientId,
        tx: TransactionId,
    },
    Chargeback {
        client: ClientId,
        tx: TransactionId,
    },
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
    pub(crate) client: ClientId,
    pub(crate) amount: Amount,
    pub(crate) kind: TransactionKind,
    pub(crate) state: TransactionState,
}

impl TransactionRecord {
    pub(crate) fn is_disputable(&self) -> bool {
        matches!(self.kind, TransactionKind::Deposit) && self.state.is_disputable()
    }
}
