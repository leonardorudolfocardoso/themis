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
pub enum Event {
    DepositAccepted {
        client: ClientId,
        tx: TransactionId,
        amount: Amount,
    },
    WithdrawalAccepted {
        client: ClientId,
        tx: TransactionId,
        amount: Amount,
    },
    DepositDisputed {
        client: ClientId,
        tx: TransactionId,
        amount: Amount,
    },
    DisputeResolved {
        client: ClientId,
        tx: TransactionId,
        amount: Amount,
    },
    DepositChargedBack {
        client: ClientId,
        tx: TransactionId,
        amount: Amount,
    },
}
