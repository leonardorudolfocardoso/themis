use crate::Amount;
use crate::id::ClientId;
use crate::id::TransactionId;

/// A ledger event accepted by Themis.
///
/// Unlike [`Command`](crate::Command), which represents a request from outside
/// the system, an `Event` represents a fact that passed business validation and
/// should be applied to the ledger projection.
///
/// Events are the source for rebuilding account balances and transaction
/// lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// A deposit command accepted into the ledger.
    Deposit {
        /// The account owner.
        client: ClientId,
        /// The accepted transaction ID.
        tx: TransactionId,
        /// The accepted deposit amount.
        amount: Amount,
    },
    /// A withdrawal command accepted into the ledger.
    Withdrawal {
        /// The account owner.
        client: ClientId,
        /// The accepted transaction ID.
        tx: TransactionId,
        /// The accepted withdrawal amount.
        amount: Amount,
    },
    /// A dispute accepted for an existing deposit.
    DepositDisputed {
        /// The account owner.
        client: ClientId,
        /// The disputed deposit transaction ID.
        tx: TransactionId,
        /// The disputed deposit amount.
        amount: Amount,
    },
    /// A dispute resolution accepted for an existing disputed deposit.
    DisputeResolved {
        /// The account owner.
        client: ClientId,
        /// The resolved deposit transaction ID.
        tx: TransactionId,
        /// The resolved deposit amount.
        amount: Amount,
    },
    /// A chargeback accepted for an existing disputed deposit.
    DepositChargedBack {
        /// The account owner.
        client: ClientId,
        /// The charged-back deposit transaction ID.
        tx: TransactionId,
        /// The charged-back deposit amount.
        amount: Amount,
    },
}
