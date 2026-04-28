use std::collections::HashMap;

use crate::amount::Amount;
use crate::event::Event;
use crate::id::{ClientId, TransactionId};

/// Lifecycle state of a transaction.
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

/// A processed transaction, tracking its dispute lifecycle.
pub(crate) struct Transaction {
    /// The client this transaction belongs to.
    pub(crate) client: ClientId,
    /// The transaction amount.
    pub(crate) amount: Amount,
    /// Whether this was a deposit or withdrawal.
    pub(crate) kind: Kind,
    /// Current dispute state of the transaction.
    pub(crate) state: State,
}

impl Transaction {
    /// Returns `true` if the transaction can be disputed.
    ///
    /// Only `Valid` deposits are disputable; withdrawals are never disputable.
    pub(crate) fn is_disputable(&self) -> bool {
        matches!(self.kind, Kind::Deposit) && self.state.is_disputable()
    }
}

/// All processed transactions, indexed by ID — the transaction aggregate.
///
/// Owns the dispute lifecycle and transaction identity. Used by the ledger
/// to validate commands against existing transaction state.
#[derive(Default)]
pub(crate) struct Transactions(HashMap<TransactionId, Transaction>);

impl Transactions {
    /// Returns `true` if a transaction with the given ID has already been processed.
    pub(crate) fn contains(&self, tx: &TransactionId) -> bool {
        self.0.contains_key(tx)
    }

    /// Returns a reference to the transaction with the given ID, if it exists.
    pub(crate) fn get(&self, tx: &TransactionId) -> Option<&Transaction> {
        self.0.get(tx)
    }

    /// Applies a validated event, updating transaction records.
    pub(crate) fn apply(&mut self, event: Event) {
        match event {
            Event::Deposited {
                client,
                tx,
                amount,
            } => {
                self.0.insert(
                    tx,
                    Transaction {
                        client,
                        amount,
                        kind: Kind::Deposit,
                        state: State::Valid,
                    },
                );
            }
            Event::Withdrawn {
                client,
                tx,
                amount,
            } => {
                self.0.insert(
                    tx,
                    Transaction {
                        client,
                        amount,
                        kind: Kind::Withdrawal,
                        state: State::Valid,
                    },
                );
            }
            Event::DisputeOpened {
                client: _,
                tx,
                amount: _,
            } => {
                self.0.get_mut(&tx).expect("transaction must exist").state = State::Disputed;
            }
            Event::DisputeResolved {
                client: _,
                tx,
                amount: _,
            } => {
                self.0.get_mut(&tx).expect("transaction must exist").state = State::Resolved;
            }
            Event::ChargedBack {
                client: _,
                tx,
                amount: _,
            } => {
                self.0.get_mut(&tx).expect("transaction must exist").state = State::Chargedback;
            }
        }
    }
}
