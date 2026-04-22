use crate::account::Account;
use crate::command::Command;
use crate::decider::{Decider, Decision};
use crate::event::Event;
use crate::event_log::EventLog;
use crate::id::ClientId;
use crate::projection::LedgerProjection;

/// Coordinates command handling for the transaction ledger.
///
/// `Ledger` decides whether each [`Command`] should produce an accepted
/// [`Event`], records accepted events, and applies them to the current
/// [`LedgerProjection`].
///
/// - Duplicate transaction IDs are silently ignored.
/// - Only deposits can be disputed; disputes on withdrawals are ignored.
/// - Disputes, resolves, and chargebacks must reference a transaction
///   belonging to the same client.
/// - All operations on locked accounts are silently ignored.
#[derive(Default)]
pub struct Ledger {
    events: EventLog,
    projection: LedgerProjection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyResult {
    Applied,
    Ignored,
}

impl Ledger {
    /// Creates a new ledger with no accounts or transaction history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Consumes all commands from `transactions` and returns the final account state.
    ///
    /// Accounts are created by accepted deposit events.
    pub fn process(
        mut self,
        transactions: impl Iterator<Item = Command>,
    ) -> std::collections::HashMap<u16, Account> {
        for transaction in transactions {
            self.apply(transaction);
        }
        self.projection.into_accounts()
    }

    /// Returns the current account state for `client`, if it exists.
    pub fn account(&self, client: ClientId) -> Option<&Account> {
        self.projection.account(client)
    }

    /// Returns accepted ledger events in the order they were applied.
    pub fn events(&self) -> &[Event] {
        self.events.events()
    }

    pub fn apply(&mut self, transaction: Command) -> ApplyResult {
        match Decider::decide(&self.projection, transaction) {
            Decision::Apply(event) => {
                self.events.append(event);
                self.projection.apply(event);
                ApplyResult::Applied
            }
            Decision::Ignore => ApplyResult::Ignored,
        }
    }
}

#[cfg(test)]
mod test {
    use super::ApplyResult;
    use super::Ledger;
    use crate::account::Account;
    use crate::amount::Amount;
    use crate::command::Command;
    use crate::event::Event;

    fn account(ledger: &Ledger, client: u16) -> &Account {
        ledger.account(client).expect("account not found")
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

    #[test]
    fn test_accepted_commands_are_recorded_and_projected() {
        let mut ledger = Ledger::new();

        assert_eq!(ledger.apply(deposit(1, 1, 100)), ApplyResult::Applied);
        assert_eq!(ledger.apply(withdrawal(1, 2, 40)), ApplyResult::Applied);

        assert_eq!(
            ledger.events(),
            &[deposit_accepted(1, 1, 100), withdrawal_accepted(1, 2, 40)]
        );

        let account = account(&ledger, 1);
        assert_eq!(account.available(), 60);
        assert_eq!(account.total(), 60);
    }

    #[test]
    fn test_ignored_command_is_not_recorded_or_projected() {
        let mut ledger = Ledger::new();

        assert_eq!(ledger.apply(withdrawal(1, 1, 100)), ApplyResult::Ignored);

        assert!(ledger.events().is_empty());
        assert!(ledger.account(1).is_none());
    }

    #[test]
    fn test_process_consumes_commands_and_returns_final_accounts() {
        let accounts =
            Ledger::new().process(vec![deposit(1, 1, 100), withdrawal(1, 2, 40)].into_iter());

        assert_eq!(accounts.len(), 1);
        let account = accounts.get(&1).expect("account not found");
        assert_eq!(account.available(), 60);
        assert_eq!(account.total(), 60);
    }
}
