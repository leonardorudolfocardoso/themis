use std::collections::HashMap;

use crate::amount::Amount;
use crate::event::Event;
use crate::id::{ClientId, TransactionId};

/// Lifecycle state of a transaction.
///
/// Transitions flow in one direction: `Valid` → `Disputed` → `Resolved` or `Chargedback`.
/// A resolved or charged-back transaction cannot be disputed again.
enum State {
    /// The transaction has not been disputed.
    Valid,
    /// A dispute is open on this transaction.
    Disputed,
    /// The dispute was resolved; funds were returned to the client.
    Resolved,
    /// The dispute was finalised as a chargeback; funds were permanently deducted.
    Chargedback,
}

/// The kind of transaction, used to determine eligibility for disputes.
///
/// Only deposits can be disputed — withdrawal disputes are silently ignored.
enum Kind {
    /// A credit to the client's account.
    Deposit,
    /// A debit from the client's account.
    Withdrawal,
}

/// A processed transaction, tracking its dispute lifecycle.
struct Transaction {
    /// The client this transaction belongs to.
    client: ClientId,
    /// The transaction amount.
    amount: Amount,
    /// Whether this was a deposit or withdrawal.
    kind: Kind,
    /// Current dispute state of the transaction.
    state: State,
}

impl Transaction {
    /// Returns `true` if the transaction can be disputed.
    ///
    /// Only `Valid` deposits are disputable; withdrawals are never disputable.
    fn is_disputable(&self) -> bool {
        matches!(self.kind, Kind::Deposit) && matches!(self.state, State::Valid)
    }

    /// Returns the amount if the transaction belongs to `client` and is disputable.
    fn dispute_amount(&self, client: ClientId) -> Option<Amount> {
        (self.client == client && self.is_disputable()).then_some(self.amount)
    }

    /// Returns the amount if the transaction belongs to `client` and is resolvable.
    fn resolve_amount(&self, client: ClientId) -> Option<Amount> {
        (self.client == client && matches!(self.state, State::Disputed)).then_some(self.amount)
    }

    /// Returns the amount if the transaction belongs to `client` and is chargebackable.
    fn chargeback_amount(&self, client: ClientId) -> Option<Amount> {
        (self.client == client && matches!(self.state, State::Disputed)).then_some(self.amount)
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

    /// Returns the disputed amount if the transaction is eligible for a dispute.
    ///
    /// Checks that the transaction exists, belongs to `client`, and is in a disputable state.
    pub(crate) fn dispute_amount(&self, client: ClientId, tx: &TransactionId) -> Option<Amount> {
        self.0.get(tx)?.dispute_amount(client)
    }

    /// Returns the held amount if the transaction is eligible for resolution.
    ///
    /// Checks that the transaction exists, belongs to `client`, and is currently disputed.
    pub(crate) fn resolve_amount(&self, client: ClientId, tx: &TransactionId) -> Option<Amount> {
        self.0.get(tx)?.resolve_amount(client)
    }

    /// Returns the held amount if the transaction is eligible for chargeback.
    ///
    /// Checks that the transaction exists, belongs to `client`, and is currently disputed.
    pub(crate) fn chargeback_amount(
        &self,
        client: ClientId,
        tx: &TransactionId,
    ) -> Option<Amount> {
        self.0.get(tx)?.chargeback_amount(client)
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

#[cfg(test)]
mod test {
    use super::Transactions;
    use crate::amount::Amount;
    use crate::event::Event;

    fn deposited(client: u16, tx: u32, amount: u64) -> Event {
        Event::Deposited {
            client,
            tx,
            amount: Amount::raw(amount),
        }
    }

    fn withdrawn(client: u16, tx: u32, amount: u64) -> Event {
        Event::Withdrawn {
            client,
            tx,
            amount: Amount::raw(amount),
        }
    }

    fn dispute_opened(client: u16, tx: u32, amount: u64) -> Event {
        Event::DisputeOpened {
            client,
            tx,
            amount: Amount::raw(amount),
        }
    }

    fn dispute_resolved(client: u16, tx: u32, amount: u64) -> Event {
        Event::DisputeResolved {
            client,
            tx,
            amount: Amount::raw(amount),
        }
    }

    fn charged_back(client: u16, tx: u32, amount: u64) -> Event {
        Event::ChargedBack {
            client,
            tx,
            amount: Amount::raw(amount),
        }
    }

    #[test]
    fn test_contains_after_deposit() {
        let mut txns = Transactions::default();
        assert!(!txns.contains(&1));
        txns.apply(deposited(1, 1, 100));
        assert!(txns.contains(&1));
    }

    #[test]
    fn test_contains_after_withdrawal() {
        let mut txns = Transactions::default();
        txns.apply(withdrawn(1, 1, 50));
        assert!(txns.contains(&1));
    }

    #[test]
    fn test_dispute_amount_on_valid_deposit() {
        let mut txns = Transactions::default();
        txns.apply(deposited(1, 1, 100));
        assert_eq!(txns.dispute_amount(1, &1), Some(Amount::raw(100)));
    }

    #[test]
    fn test_dispute_amount_on_withdrawal_is_none() {
        let mut txns = Transactions::default();
        txns.apply(withdrawn(1, 1, 50));
        assert_eq!(txns.dispute_amount(1, &1), None);
    }

    #[test]
    fn test_dispute_amount_wrong_client_is_none() {
        let mut txns = Transactions::default();
        txns.apply(deposited(1, 1, 100));
        assert_eq!(txns.dispute_amount(2, &1), None);
    }

    #[test]
    fn test_dispute_amount_unknown_tx_is_none() {
        let txns = Transactions::default();
        assert_eq!(txns.dispute_amount(1, &99), None);
    }

    #[test]
    fn test_dispute_amount_already_disputed_is_none() {
        let mut txns = Transactions::default();
        txns.apply(deposited(1, 1, 100));
        txns.apply(dispute_opened(1, 1, 100));
        assert_eq!(txns.dispute_amount(1, &1), None);
    }

    #[test]
    fn test_resolve_amount_on_disputed() {
        let mut txns = Transactions::default();
        txns.apply(deposited(1, 1, 100));
        txns.apply(dispute_opened(1, 1, 100));
        assert_eq!(txns.resolve_amount(1, &1), Some(Amount::raw(100)));
    }

    #[test]
    fn test_resolve_amount_on_valid_is_none() {
        let mut txns = Transactions::default();
        txns.apply(deposited(1, 1, 100));
        assert_eq!(txns.resolve_amount(1, &1), None);
    }

    #[test]
    fn test_resolve_amount_wrong_client_is_none() {
        let mut txns = Transactions::default();
        txns.apply(deposited(1, 1, 100));
        txns.apply(dispute_opened(1, 1, 100));
        assert_eq!(txns.resolve_amount(2, &1), None);
    }

    #[test]
    fn test_resolve_amount_already_resolved_is_none() {
        let mut txns = Transactions::default();
        txns.apply(deposited(1, 1, 100));
        txns.apply(dispute_opened(1, 1, 100));
        txns.apply(dispute_resolved(1, 1, 100));
        assert_eq!(txns.resolve_amount(1, &1), None);
    }

    #[test]
    fn test_chargeback_amount_on_disputed() {
        let mut txns = Transactions::default();
        txns.apply(deposited(1, 1, 100));
        txns.apply(dispute_opened(1, 1, 100));
        assert_eq!(txns.chargeback_amount(1, &1), Some(Amount::raw(100)));
    }

    #[test]
    fn test_chargeback_amount_on_valid_is_none() {
        let mut txns = Transactions::default();
        txns.apply(deposited(1, 1, 100));
        assert_eq!(txns.chargeback_amount(1, &1), None);
    }

    #[test]
    fn test_chargeback_amount_wrong_client_is_none() {
        let mut txns = Transactions::default();
        txns.apply(deposited(1, 1, 100));
        txns.apply(dispute_opened(1, 1, 100));
        assert_eq!(txns.chargeback_amount(2, &1), None);
    }

    #[test]
    fn test_chargeback_amount_already_chargedback_is_none() {
        let mut txns = Transactions::default();
        txns.apply(deposited(1, 1, 100));
        txns.apply(dispute_opened(1, 1, 100));
        txns.apply(charged_back(1, 1, 100));
        assert_eq!(txns.chargeback_amount(1, &1), None);
    }

    #[test]
    fn test_dispute_after_resolve_is_none() {
        let mut txns = Transactions::default();
        txns.apply(deposited(1, 1, 100));
        txns.apply(dispute_opened(1, 1, 100));
        txns.apply(dispute_resolved(1, 1, 100));
        assert_eq!(txns.dispute_amount(1, &1), None);
    }

    #[test]
    fn test_dispute_after_chargeback_is_none() {
        let mut txns = Transactions::default();
        txns.apply(deposited(1, 1, 100));
        txns.apply(dispute_opened(1, 1, 100));
        txns.apply(charged_back(1, 1, 100));
        assert_eq!(txns.dispute_amount(1, &1), None);
    }
}
