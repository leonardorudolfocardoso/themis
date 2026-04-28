use crate::amount::Amount;
use crate::id::ClientId;

/// Lifecycle state of a transaction record.
///
/// Transitions flow in one direction: `Valid` → `Disputed` → `Resolved` or `Chargedback`.
/// A resolved or charged-back transaction cannot be disputed again.
pub(crate) enum State {
    /// The transaction has not been disputed.
    Valid,
    /// A dispute is open on this transaction.
    Disputed,
    /// The dispute was resolved; funds were returned to the client.
    Resolved,
    /// The dispute was finalised as a chargeback; funds were permanently deducted.
    Chargedback,
}

impl State {
    /// Returns `true` if the transaction can be disputed (i.e. it is `Valid`).
    pub(crate) fn is_disputable(&self) -> bool {
        matches!(self, State::Valid)
    }

    /// Returns `true` if the transaction can be resolved (i.e. it is `Disputed`).
    pub(crate) fn is_resolvable(&self) -> bool {
        matches!(self, State::Disputed)
    }

    /// Returns `true` if the transaction can be charged back (i.e. it is `Disputed`).
    pub(crate) fn is_chargebackable(&self) -> bool {
        matches!(self, State::Disputed)
    }
}

/// The kind of transaction, used to determine eligibility for disputes.
///
/// Only deposits can be disputed — withdrawal disputes are silently ignored.
pub(crate) enum Kind {
    /// A credit to the client's account.
    Deposit,
    /// A debit from the client's account.
    Withdrawal,
}

/// Internal record of a processed transaction, tracking its dispute lifecycle.
pub(crate) struct Record {
    /// The client this transaction belongs to.
    pub(crate) client: ClientId,
    /// The transaction amount.
    pub(crate) amount: Amount,
    /// Whether this was a deposit or withdrawal.
    pub(crate) kind: Kind,
    /// Current dispute state of the transaction.
    pub(crate) state: State,
}

impl Record {
    /// Returns `true` if the transaction can be disputed.
    ///
    /// Only `Valid` deposits are disputable; withdrawals are never disputable.
    pub(crate) fn is_disputable(&self) -> bool {
        matches!(self.kind, Kind::Deposit) && self.state.is_disputable()
    }

    /// Opens a dispute and returns the amount that must be held.
    pub(crate) fn open_dispute(&mut self, client: ClientId) -> Option<Amount> {
        if self.client != client || !self.is_disputable() {
            return None;
        }
        self.state = State::Disputed;
        Some(self.amount)
    }

    /// Resolves an open dispute and returns the amount that must be released.
    pub(crate) fn resolve_dispute(&mut self, client: ClientId) -> Option<Amount> {
        if self.client != client || !self.state.is_resolvable() {
            return None;
        }
        self.state = State::Resolved;
        Some(self.amount)
    }

    /// Finalises an open dispute and returns the amount to charge back.
    pub(crate) fn chargeback(&mut self, client: ClientId) -> Option<Amount> {
        if self.client != client || !self.state.is_chargebackable() {
            return None;
        }
        self.state = State::Chargedback;
        Some(self.amount)
    }
}
