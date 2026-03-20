use crate::{
    amount::Amount,
    id::{ClientId, TransactionId},
};

/// A transaction event parsed from the input stream.
///
/// Each variant corresponds to one row in the CSV input. Deposit and
/// withdrawal events carry an amount; dispute, resolve, and chargeback
/// events reference an existing transaction by ID.
pub enum Event {
    /// Credits `amount` to the client's account.
    Deposit {
        client: ClientId,
        tx: TransactionId,
        amount: Amount,
    },
    /// Debits `amount` from the client's account.
    Withdrawal {
        client: ClientId,
        tx: TransactionId,
        amount: Amount,
    },
    /// Opens a dispute on transaction `tx`, freezing the associated funds.
    Dispute {
        client: ClientId,
        tx: TransactionId,
    },
    /// Resolves the dispute on transaction `tx`, releasing the frozen funds.
    Resolve {
        client: ClientId,
        tx: TransactionId,
    },
    /// Finalises the dispute on transaction `tx` in the client's favour,
    /// permanently deducting the frozen funds and locking the account.
    Chargeback {
        client: ClientId,
        tx: TransactionId,
    },
}
