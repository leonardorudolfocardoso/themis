use crate::amount::Amount;
use crate::id::ClientId;

pub(crate) enum State {
    Valid,
    Disputed,
    Resolved,
    Chargedback,
}

impl State {
    pub(crate) fn is_disputable(&self) -> bool {
        matches!(self, State::Valid)
    }

    pub(crate) fn is_resolvable(&self) -> bool {
        matches!(self, State::Disputed)
    }

    pub(crate) fn is_chargebackable(&self) -> bool {
        matches!(self, State::Disputed)
    }
}

pub(crate) enum Kind {
    Deposit,
    Withdrawal,
}

pub(crate) struct Record {
    pub(crate) client: ClientId,
    pub(crate) amount: Amount,
    pub(crate) kind: Kind,
    pub(crate) state: State,
}

impl Record {
    pub(crate) fn is_disputable(&self) -> bool {
        matches!(self.kind, Kind::Deposit) && self.state.is_disputable()
    }
}
