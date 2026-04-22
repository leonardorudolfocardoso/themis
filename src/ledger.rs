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

    fn ledger_with(transactions: Vec<Command>) -> Ledger {
        let mut ledger = Ledger::new();
        for transaction in transactions {
            ledger.apply(transaction);
        }
        ledger
    }

    fn account(ledger: &Ledger, client: u16) -> &Account {
        ledger.account(client).expect("account not found")
    }

    #[test]
    fn test_deposit_is_applied() {
        let mut ledger = Ledger::new();

        let result = ledger.apply(Command::Deposit {
            client: 1,
            tx: 1,
            amount: Amount::raw(100),
        });

        assert_eq!(result, ApplyResult::Applied);
        let account = account(&ledger, 1);
        assert_eq!(account.available(), 100);
        assert_eq!(account.total(), 100);
    }

    #[test]
    fn test_accepted_events_are_recorded() {
        let mut ledger = Ledger::new();

        ledger.apply(Command::Deposit {
            client: 1,
            tx: 1,
            amount: Amount::raw(100),
        });
        ledger.apply(Command::Deposit {
            client: 1,
            tx: 1,
            amount: Amount::raw(999),
        });

        assert_eq!(
            ledger.events(),
            &[Event::DepositAccepted {
                client: 1,
                tx: 1,
                amount: Amount::raw(100)
            }]
        );
    }

    #[test]
    fn test_withdrawal_is_applied() {
        let mut ledger = ledger_with(vec![Command::Deposit {
            client: 1,
            tx: 1,
            amount: Amount::raw(100),
        }]);

        let result = ledger.apply(Command::Withdrawal {
            client: 1,
            tx: 2,
            amount: Amount::raw(20),
        });

        assert_eq!(result, ApplyResult::Applied);
        let account = account(&ledger, 1);
        assert_eq!(account.available(), 80);
        assert_eq!(account.total(), 80);
    }

    #[test]
    fn test_ignored_withdrawal_does_not_create_account() {
        let mut ledger = Ledger::new();

        let result = ledger.apply(Command::Withdrawal {
            client: 1,
            tx: 1,
            amount: Amount::raw(100),
        });

        assert_eq!(result, ApplyResult::Ignored);
        assert!(ledger.account(1).is_none());
    }

    #[test]
    fn test_duplicate_tx_id_is_ignored() {
        let mut ledger = ledger_with(vec![Command::Deposit {
            client: 1,
            tx: 1,
            amount: Amount::raw(100),
        }]);

        let result = ledger.apply(Command::Deposit {
            client: 1,
            tx: 1,
            amount: Amount::raw(999),
        });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&ledger, 1);
        assert_eq!(account.available(), 100);
        assert_eq!(account.total(), 100);
    }

    #[test]
    fn test_duplicate_withdrawal_tx_id_is_ignored() {
        let mut ledger = ledger_with(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Withdrawal {
                client: 1,
                tx: 2,
                amount: Amount::raw(20),
            },
        ]);

        let result = ledger.apply(Command::Withdrawal {
            client: 1,
            tx: 2,
            amount: Amount::raw(20),
        });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&ledger, 1);
        assert_eq!(account.available(), 80);
        assert_eq!(account.total(), 80);
    }

    #[test]
    fn test_deposit_on_locked_account_is_ignored() {
        let mut ledger = ledger_with(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 1, tx: 1 },
            Command::Chargeback { client: 1, tx: 1 },
        ]);

        let result = ledger.apply(Command::Deposit {
            client: 1,
            tx: 2,
            amount: Amount::raw(50),
        });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&ledger, 1);
        assert_eq!(account.available(), 0);
        assert_eq!(account.total(), 0);
        assert!(account.locked());
    }

    #[test]
    fn test_withdraw_locked_is_silently_ignored() {
        let mut ledger = ledger_with(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 1, tx: 1 },
            Command::Chargeback { client: 1, tx: 1 },
        ]);

        let result = ledger.apply(Command::Withdrawal {
            client: 1,
            tx: 2,
            amount: Amount::raw(50),
        });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&ledger, 1);
        assert_eq!(account.total(), 0);
        assert!(account.locked());
    }

    #[test]
    fn test_dispute_on_withdrawal_is_ignored() {
        let mut ledger = ledger_with(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Withdrawal {
                client: 1,
                tx: 2,
                amount: Amount::raw(40),
            },
        ]);

        let result = ledger.apply(Command::Dispute { client: 1, tx: 2 });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&ledger, 1);
        assert_eq!(account.available(), 60);
        assert_eq!(account.held(), Amount::default());
    }

    #[test]
    fn test_dispute_decreases_available_and_increases_held() {
        let mut ledger = ledger_with(vec![Command::Deposit {
            client: 1,
            tx: 1,
            amount: Amount::raw(100),
        }]);

        let result = ledger.apply(Command::Dispute { client: 1, tx: 1 });

        assert_eq!(result, ApplyResult::Applied);
        let account = account(&ledger, 1);
        assert_eq!(account.available(), 0);
        assert_eq!(account.held(), Amount::raw(100));
        assert_eq!(account.total(), 100);
    }

    #[test]
    fn test_dispute_on_wrong_client_is_ignored() {
        let mut ledger = ledger_with(vec![Command::Deposit {
            client: 1,
            tx: 1,
            amount: Amount::raw(100),
        }]);

        let result = ledger.apply(Command::Dispute { client: 2, tx: 1 });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&ledger, 1);
        assert_eq!(account.available(), 100);
        assert_eq!(account.held(), Amount::default());
    }

    #[test]
    fn test_dispute_on_invalid_state_is_ignored() {
        let mut ledger = ledger_with(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 1, tx: 1 },
        ]);

        let result = ledger.apply(Command::Dispute { client: 1, tx: 1 });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&ledger, 1);
        assert_eq!(account.available(), 0);
        assert_eq!(account.held(), Amount::raw(100));
    }

    #[test]
    fn test_dispute_on_locked_account_is_ignored() {
        let mut ledger = ledger_with(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Deposit {
                client: 1,
                tx: 2,
                amount: Amount::raw(50),
            },
            Command::Dispute { client: 1, tx: 1 },
            Command::Chargeback { client: 1, tx: 1 },
        ]);

        let result = ledger.apply(Command::Dispute { client: 1, tx: 2 });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&ledger, 1);
        assert_eq!(account.available(), 50);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), 50);
        assert!(account.locked());
    }

    #[test]
    fn test_resolve_decreases_held_and_increases_available() {
        let mut ledger = ledger_with(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 1, tx: 1 },
        ]);

        let result = ledger.apply(Command::Resolve { client: 1, tx: 1 });

        assert_eq!(result, ApplyResult::Applied);
        let account = account(&ledger, 1);
        assert_eq!(account.available(), 100);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), 100);
    }

    #[test]
    fn test_resolve_without_dispute_is_ignored() {
        let mut ledger = ledger_with(vec![Command::Deposit {
            client: 1,
            tx: 1,
            amount: Amount::raw(100),
        }]);

        let result = ledger.apply(Command::Resolve { client: 1, tx: 1 });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&ledger, 1);
        assert_eq!(account.available(), 100);
        assert_eq!(account.held(), Amount::default());
    }

    #[test]
    fn test_resolve_on_wrong_client_is_ignored() {
        let mut ledger = ledger_with(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 1, tx: 1 },
        ]);

        let result = ledger.apply(Command::Resolve { client: 2, tx: 1 });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&ledger, 1);
        assert_eq!(account.available(), 0);
        assert_eq!(account.held(), Amount::raw(100));
    }

    #[test]
    fn test_resolve_on_already_resolved_is_ignored() {
        let mut ledger = ledger_with(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 1, tx: 1 },
            Command::Resolve { client: 1, tx: 1 },
        ]);

        let result = ledger.apply(Command::Resolve { client: 1, tx: 1 });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&ledger, 1);
        assert_eq!(account.available(), 100);
        assert_eq!(account.held(), Amount::default());
    }

    #[test]
    fn test_chargeback_locks_account_and_decreases_held_and_total() {
        let mut ledger = ledger_with(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 1, tx: 1 },
        ]);

        let result = ledger.apply(Command::Chargeback { client: 1, tx: 1 });

        assert_eq!(result, ApplyResult::Applied);
        let account = account(&ledger, 1);
        assert_eq!(account.available(), 0);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), 0);
        assert!(account.locked());
    }

    #[test]
    fn test_chargeback_without_dispute_is_ignored() {
        let mut ledger = ledger_with(vec![Command::Deposit {
            client: 1,
            tx: 1,
            amount: Amount::raw(100),
        }]);

        let result = ledger.apply(Command::Chargeback { client: 1, tx: 1 });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&ledger, 1);
        assert!(!account.locked());
    }

    #[test]
    fn test_chargeback_on_wrong_client_is_ignored() {
        let mut ledger = ledger_with(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 1, tx: 1 },
        ]);

        let result = ledger.apply(Command::Chargeback { client: 2, tx: 1 });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&ledger, 1);
        assert_eq!(account.held(), Amount::raw(100));
        assert!(!account.locked());
    }

    #[test]
    fn test_chargeback_on_already_chargedback_is_ignored() {
        let mut ledger = ledger_with(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 1, tx: 1 },
            Command::Chargeback { client: 1, tx: 1 },
        ]);

        let result = ledger.apply(Command::Chargeback { client: 1, tx: 1 });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&ledger, 1);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), 0);
        assert!(account.locked());
    }

    #[test]
    fn test_resolve_after_withdrawal_is_correct() {
        // Deposit 100, withdraw 80 (total=20), dispute the deposit, resolve.
        let ledger = ledger_with(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Withdrawal {
                client: 1,
                tx: 2,
                amount: Amount::raw(80),
            },
            Command::Dispute { client: 1, tx: 1 },
            Command::Resolve { client: 1, tx: 1 },
        ]);
        let account = account(&ledger, 1);
        assert_eq!(account.available(), 20);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), 20);
    }

    #[test]
    fn test_chargeback_of_deposit_after_withdrawal_total_is_negative() {
        // Deposit 100, withdraw 80 (total=20), dispute the deposit, chargeback.
        // total = 20 - 100 = -80: the account owes the bank the withdrawn funds.
        let ledger = ledger_with(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Withdrawal {
                client: 1,
                tx: 2,
                amount: Amount::raw(80),
            },
            Command::Dispute { client: 1, tx: 1 },
            Command::Chargeback { client: 1, tx: 1 },
        ]);
        let account = account(&ledger, 1);
        assert_eq!(account.available(), -80);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), -80);
        assert!(account.locked());
    }
}
