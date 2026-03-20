use crate::{
    amount::Amount,
    id::{ClientId, TransactionId},
};

/// A transaction event parsed from the input stream.
///
/// Each variant corresponds to one row in the CSV input. All variants carry
/// `client` (the account owner) and `tx` (the transaction ID). Deposit and
/// withdrawal variants also carry `amount`.
pub enum Event {
    /// Credits `amount` to the client's account.
    Deposit {
        /// The account owner.
        client: ClientId,
        /// The transaction ID.
        tx: TransactionId,
        /// The amount to credit.
        amount: Amount,
    },
    /// Debits `amount` from the client's account.
    Withdrawal {
        /// The account owner.
        client: ClientId,
        /// The transaction ID.
        tx: TransactionId,
        /// The amount to debit.
        amount: Amount,
    },
    /// Opens a dispute on transaction `tx`, freezing the associated funds.
    Dispute {
        /// The account owner.
        client: ClientId,
        /// The transaction being disputed.
        tx: TransactionId,
    },
    /// Resolves the dispute on transaction `tx`, releasing the frozen funds.
    Resolve {
        /// The account owner.
        client: ClientId,
        /// The transaction being resolved.
        tx: TransactionId,
    },
    /// Finalises the dispute on transaction `tx` in the client's favour,
    /// permanently deducting the frozen funds and locking the account.
    Chargeback {
        /// The account owner.
        client: ClientId,
        /// The transaction being charged back.
        tx: TransactionId,
    },
}
