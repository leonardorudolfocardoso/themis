use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::amount::Amount;
use crate::id::{ClientId, TransactionId};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Recorded {
    pub(crate) position: u64,
    pub(crate) recorded_at: SystemTime,
    pub(crate) event: Event,
}

/// A domain event representing a validated state change.
///
/// Events are the output of the decision phase — each variant
/// carries all data needed to apply the mutation without further lookups.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Event {
    /// Funds were credited to the client's account.
    Deposited {
        /// The account owner.
        client: ClientId,
        /// The transaction ID.
        tx: TransactionId,
        /// The credited amount.
        amount: Amount,
    },
    /// Funds were debited from the client's account.
    Withdrawn {
        /// The account owner.
        client: ClientId,
        /// The transaction ID.
        tx: TransactionId,
        /// The debited amount.
        amount: Amount,
    },
    /// A dispute was opened, moving funds to held.
    DisputeOpened {
        /// The account owner.
        client: ClientId,
        /// The disputed transaction ID.
        tx: TransactionId,
        /// The held amount.
        amount: Amount,
    },
    /// A dispute was resolved, releasing held funds.
    DisputeResolved {
        /// The account owner.
        client: ClientId,
        /// The resolved transaction ID.
        tx: TransactionId,
        /// The released amount.
        amount: Amount,
    },
    /// A chargeback was finalised, removing held funds and locking the account.
    ChargedBack {
        /// The account owner.
        client: ClientId,
        /// The charged-back transaction ID.
        tx: TransactionId,
        /// The charged-back amount.
        amount: Amount,
    },
}

/// The outcome of validating a command against current state.
pub(crate) enum Decision {
    /// The command passed validation and produced an event.
    Approved(Event),
    /// The command was rejected (duplicate, locked, insufficient funds, etc.).
    Denied,
}
