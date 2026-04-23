use crate::amount::Amount;
use crate::command::Command;
use crate::event::Event;
use crate::projection::LedgerProjection;

/// The result of evaluating a [`Command`] against the current ledger state.
///
/// A command either becomes one accepted [`Event`] or is ignored without
/// changing ledger state.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Decision {
    /// The command passed business validation and should be recorded.
    Apply(Event),
    /// The command is invalid for the current state and should do nothing.
    Ignore,
}

/// Decides whether transaction commands should become accepted ledger events.
///
/// `Decider` owns business rule checks that require current ledger state, such
/// as duplicate transaction IDs, locked accounts, available funds, transaction
/// ownership, and dispute lifecycle state. It does not change state itself;
/// [`Ledger`](crate::Ledger) coordinates the state change after receiving a
/// [`Decision::Apply`].
pub(crate) struct Decider;

impl Decider {
    /// Evaluates `transaction` against the current ledger state.
    ///
    /// Returns [`Decision::Apply`] with the accepted event when the command is
    /// valid for the current state, otherwise returns [`Decision::Ignore`].
    pub(crate) fn decide(projection: &LedgerProjection, transaction: Command) -> Decision {
        match transaction {
            Command::Deposit { client, tx, amount } => {
                if projection.has_record(tx) {
                    Decision::Ignore
                } else {
                    Self::decide_deposit(projection, client, tx, amount)
                }
            }
            Command::Withdrawal { client, tx, amount } => {
                if projection.has_record(tx) {
                    Decision::Ignore
                } else {
                    Self::decide_withdrawal(projection, client, tx, amount)
                }
            }
            Command::Dispute { client, tx } => Self::decide_dispute(projection, client, tx),
            Command::Resolve { client, tx } => Self::decide_resolve(projection, client, tx),
            Command::Chargeback { client, tx } => Self::decide_chargeback(projection, client, tx),
        }
    }

    fn decide_deposit(
        projection: &LedgerProjection,
        client: u16,
        tx: u32,
        amount: Amount,
    ) -> Decision {
        if projection
            .account(client)
            .is_some_and(|account| account.locked())
        {
            return Decision::Ignore;
        }

        Decision::Apply(Event::DepositAccepted { client, tx, amount })
    }

    fn decide_withdrawal(
        projection: &LedgerProjection,
        client: u16,
        tx: u32,
        amount: Amount,
    ) -> Decision {
        let Some(account) = projection.account(client) else {
            return Decision::Ignore;
        };

        if account.locked() || account.available() < amount {
            return Decision::Ignore;
        }

        Decision::Apply(Event::WithdrawalAccepted { client, tx, amount })
    }

    fn decide_dispute(projection: &LedgerProjection, client: u16, tx: u32) -> Decision {
        let amount = match projection.record(tx) {
            Some(record) if record.client == client && record.is_disputable() => record.amount,
            _ => return Decision::Ignore,
        };

        if projection
            .account(client)
            .is_some_and(|account| account.locked())
        {
            return Decision::Ignore;
        }

        Decision::Apply(Event::DepositDisputed { client, tx, amount })
    }

    fn decide_resolve(projection: &LedgerProjection, client: u16, tx: u32) -> Decision {
        let amount = match projection.record(tx) {
            Some(record) if record.client == client && record.state.is_resolvable() => {
                record.amount
            }
            _ => return Decision::Ignore,
        };

        if projection
            .account(client)
            .is_some_and(|account| account.locked())
        {
            return Decision::Ignore;
        }

        Decision::Apply(Event::DisputeResolved { client, tx, amount })
    }

    fn decide_chargeback(projection: &LedgerProjection, client: u16, tx: u32) -> Decision {
        let amount = match projection.record(tx) {
            Some(record) if record.client == client && record.state.is_chargebackable() => {
                record.amount
            }
            _ => return Decision::Ignore,
        };

        if projection
            .account(client)
            .is_some_and(|account| account.locked())
        {
            return Decision::Ignore;
        }

        Decision::Apply(Event::DepositChargedBack { client, tx, amount })
    }
}

#[cfg(test)]
mod test {
    use super::{Decider, Decision};
    use crate::amount::Amount;
    use crate::command::Command;
    use crate::event::Event;
    use crate::projection::LedgerProjection;

    fn projection_with(events: &[Event]) -> LedgerProjection {
        let mut projection = LedgerProjection::default();
        for event in events {
            projection.apply(*event);
        }
        projection
    }

    fn decide_with_previous_events(events: &[Event], command: Command) -> Decision {
        Decider::decide(&projection_with(events), command)
    }

    fn deposit(client: u16, tx: u32, amount: u64) -> Command {
        Command::Deposit {
            client,
            tx,
            amount: Amount::raw(amount),
        }
    }

    fn withdrawal(client: u16, tx: u32, amount: u64) -> Command {
        Command::Withdrawal {
            client,
            tx,
            amount: Amount::raw(amount),
        }
    }

    fn deposit_accepted(client: u16, tx: u32, amount: u64) -> Event {
        Event::DepositAccepted {
            client,
            tx,
            amount: Amount::raw(amount),
        }
    }

    fn withdrawal_accepted(client: u16, tx: u32, amount: u64) -> Event {
        Event::WithdrawalAccepted {
            client,
            tx,
            amount: Amount::raw(amount),
        }
    }

    fn deposit_disputed(client: u16, tx: u32, amount: u64) -> Event {
        Event::DepositDisputed {
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

    fn deposit_charged_back(client: u16, tx: u32, amount: u64) -> Event {
        Event::DepositChargedBack {
            client,
            tx,
            amount: Amount::raw(amount),
        }
    }

    #[test]
    fn test_deposit_is_accepted_for_new_transaction() {
        assert_eq!(
            decide_with_previous_events(&[], deposit(1, 1, 100)),
            Decision::Apply(deposit_accepted(1, 1, 100))
        );
    }

    #[test]
    fn test_duplicate_deposit_is_ignored() {
        assert_eq!(
            decide_with_previous_events(&[deposit_accepted(1, 1, 100)], deposit(1, 1, 999),),
            Decision::Ignore
        );
    }

    #[test]
    fn test_withdrawal_is_accepted_when_account_has_enough_available_funds() {
        assert_eq!(
            decide_with_previous_events(&[deposit_accepted(1, 1, 100)], withdrawal(1, 2, 60),),
            Decision::Apply(withdrawal_accepted(1, 2, 60))
        );
    }

    #[test]
    fn test_withdrawal_is_ignored_without_existing_account() {
        assert_eq!(
            decide_with_previous_events(&[], withdrawal(1, 1, 60)),
            Decision::Ignore
        );
    }

    #[test]
    fn test_withdrawal_is_ignored_without_enough_available_funds() {
        assert_eq!(
            decide_with_previous_events(&[deposit_accepted(1, 1, 50)], withdrawal(1, 2, 60),),
            Decision::Ignore
        );
    }

    #[test]
    fn test_dispute_is_accepted_for_valid_deposit_owned_by_client() {
        assert_eq!(
            decide_with_previous_events(
                &[deposit_accepted(1, 1, 100)],
                Command::Dispute { client: 1, tx: 1 },
            ),
            Decision::Apply(deposit_disputed(1, 1, 100))
        );
    }

    #[test]
    fn test_dispute_is_ignored_for_withdrawal() {
        assert_eq!(
            decide_with_previous_events(
                &[deposit_accepted(1, 1, 100), withdrawal_accepted(1, 2, 40)],
                Command::Dispute { client: 1, tx: 2 },
            ),
            Decision::Ignore
        );
    }

    #[test]
    fn test_dispute_is_ignored_for_transaction_owned_by_another_client() {
        assert_eq!(
            decide_with_previous_events(
                &[deposit_accepted(1, 1, 100)],
                Command::Dispute { client: 2, tx: 1 },
            ),
            Decision::Ignore
        );
    }

    #[test]
    fn test_resolve_is_accepted_for_disputed_deposit() {
        assert_eq!(
            decide_with_previous_events(
                &[deposit_accepted(1, 1, 100), deposit_disputed(1, 1, 100)],
                Command::Resolve { client: 1, tx: 1 },
            ),
            Decision::Apply(dispute_resolved(1, 1, 100))
        );
    }

    #[test]
    fn test_resolve_is_ignored_for_valid_deposit() {
        assert_eq!(
            decide_with_previous_events(
                &[deposit_accepted(1, 1, 100)],
                Command::Resolve { client: 1, tx: 1 },
            ),
            Decision::Ignore
        );
    }

    #[test]
    fn test_chargeback_is_accepted_for_disputed_deposit() {
        assert_eq!(
            decide_with_previous_events(
                &[deposit_accepted(1, 1, 100), deposit_disputed(1, 1, 100)],
                Command::Chargeback { client: 1, tx: 1 },
            ),
            Decision::Apply(deposit_charged_back(1, 1, 100))
        );
    }

    #[test]
    fn test_commands_for_locked_account_are_ignored() {
        let events = [
            deposit_accepted(1, 1, 100),
            deposit_accepted(1, 2, 50),
            deposit_disputed(1, 1, 100),
            deposit_charged_back(1, 1, 100),
        ];

        assert_eq!(
            decide_with_previous_events(&events, deposit(1, 3, 50)),
            Decision::Ignore
        );
        assert_eq!(
            decide_with_previous_events(&events, withdrawal(1, 3, 10)),
            Decision::Ignore
        );
        assert_eq!(
            decide_with_previous_events(&events, Command::Dispute { client: 1, tx: 2 }),
            Decision::Ignore
        );
    }
}
