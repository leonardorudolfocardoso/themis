use crate::account::Accounts;
use crate::command::Command;
use crate::event::{Decision, Event, Log};
use crate::transaction::Transactions;

/// Processes a stream of transaction commands and maintains account state.
///
/// `Ledger` applies each [`Command`] to the corresponding [`Account`],
/// enforcing all transaction rules:
///
/// - Duplicate transaction IDs are silently ignored.
/// - Only deposits can be disputed; disputes on withdrawals are ignored.
/// - Disputes, resolves, and chargebacks must reference a transaction
///   belonging to the same client.
/// - All operations on locked accounts are silently ignored.
#[derive(Default)]
pub struct Ledger {
    log: Log,
    transactions: Transactions,
    accounts: Accounts,
}

impl Ledger {
    /// Creates a new ledger with no accounts or transaction history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Rebuilds a ledger by replaying a sequence of previously validated events.
    ///
    /// No validation is performed — events are recorded unconditionally.
    pub fn replay(events: impl Iterator<Item = Event>) -> Self {
        let mut ledger = Self::new();
        for event in events {
            ledger.record(event);
        }
        ledger
    }

    /// Ingests a stream of commands, validating each and recording approved events.
    pub fn ingest(&mut self, commands: impl Iterator<Item = Command>) {
        for command in commands {
            if let Decision::Approved(event) = self.decide(&command) {
                self.record(event);
            }
        }
    }

    /// Consumes the ledger and returns the final account state.
    pub fn into_accounts(self) -> Accounts {
        self.accounts
    }

    /// Consumes the ledger and returns the event log.
    pub fn into_log(self) -> Log {
        self.log
    }

    /// Validates a command against current state and returns a decision.
    ///
    /// This is a pure validation phase — it borrows `self` immutably.
    fn decide(&self, command: &Command) -> Decision {
        match command {
            Command::Deposit { client, tx, amount } => {
                if self.transactions.contains(tx) || self.accounts.is_locked(client) {
                    return Decision::Denied;
                }
                Decision::Approved(Event::Deposited {
                    client: *client,
                    tx: *tx,
                    amount: *amount,
                })
            }
            Command::Withdrawal { client, tx, amount } => {
                if self.transactions.contains(tx) || !self.accounts.can_withdraw(client, *amount) {
                    return Decision::Denied;
                }
                Decision::Approved(Event::Withdrawn {
                    client: *client,
                    tx: *tx,
                    amount: *amount,
                })
            }
            Command::Dispute { client, tx } => {
                let Some(amount) = self.transactions.dispute_amount(*client, tx) else {
                    return Decision::Denied;
                };
                if self.accounts.is_locked(client) {
                    return Decision::Denied;
                }
                Decision::Approved(Event::DisputeOpened {
                    client: *client,
                    tx: *tx,
                    amount,
                })
            }
            Command::Resolve { client, tx } => {
                let Some(amount) = self.transactions.resolve_amount(*client, tx) else {
                    return Decision::Denied;
                };
                if self.accounts.is_locked(client) {
                    return Decision::Denied;
                }
                Decision::Approved(Event::DisputeResolved {
                    client: *client,
                    tx: *tx,
                    amount,
                })
            }
            Command::Chargeback { client, tx } => {
                let Some(amount) = self.transactions.chargeback_amount(*client, tx) else {
                    return Decision::Denied;
                };
                if self.accounts.is_locked(client) {
                    return Decision::Denied;
                }
                Decision::Approved(Event::ChargedBack {
                    client: *client,
                    tx: *tx,
                    amount,
                })
            }
        }
    }

    /// Records an event: appends to the log and updates both aggregates.
    fn record(&mut self, event: Event) {
        self.log.push(event);
        self.transactions.apply(event);
        self.accounts.apply(event);
    }
}

#[cfg(test)]
mod test {
    use super::Ledger;
    use crate::account::Accounts;
    use crate::amount::Amount;
    use crate::command::Command;

    fn ingest(commands: Vec<Command>) -> Accounts {
        let mut ledger = Ledger::new();
        ledger.ingest(commands.into_iter());
        ledger.into_accounts()
    }

    #[test]
    fn test_duplicate_tx_id_is_ignored() {
        let accounts = ingest(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(999),
            },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), 100);
        assert_eq!(account.total(), 100);
    }

    #[test]
    fn test_duplicate_withdrawal_tx_id_is_ignored() {
        let accounts = ingest(vec![
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
            Command::Withdrawal {
                client: 1,
                tx: 2,
                amount: Amount::raw(20),
            },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), 80);
        assert_eq!(account.total(), 80);
    }

    #[test]
    fn test_deposit_on_locked_account_is_ignored() {
        let accounts = ingest(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 1, tx: 1 },
            Command::Chargeback { client: 1, tx: 1 },
            Command::Deposit {
                client: 1,
                tx: 2,
                amount: Amount::raw(50),
            },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), 0);
        assert_eq!(account.total(), 0);
        assert!(account.locked());
    }

    #[test]
    fn test_withdraw_locked_is_silently_ignored() {
        let accounts = ingest(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 1, tx: 1 },
            Command::Chargeback { client: 1, tx: 1 },
            Command::Withdrawal {
                client: 1,
                tx: 2,
                amount: Amount::raw(50),
            },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.total(), 0);
        assert!(account.locked());
    }

    #[test]
    fn test_dispute_on_withdrawal_is_ignored() {
        let accounts = ingest(vec![
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
            Command::Dispute { client: 1, tx: 2 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), 60);
        assert_eq!(account.held(), Amount::default());
    }

    #[test]
    fn test_dispute_decreases_available_and_increases_held() {
        let accounts = ingest(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 1, tx: 1 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), 0);
        assert_eq!(account.held(), Amount::raw(100));
        assert_eq!(account.total(), 100);
    }

    #[test]
    fn test_dispute_on_wrong_client_is_ignored() {
        let accounts = ingest(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 2, tx: 1 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), 100);
        assert_eq!(account.held(), Amount::default());
    }

    #[test]
    fn test_dispute_on_invalid_state_is_ignored() {
        let accounts = ingest(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 1, tx: 1 },
            Command::Dispute { client: 1, tx: 1 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), 0);
        assert_eq!(account.held(), Amount::raw(100));
    }

    #[test]
    fn test_dispute_on_locked_account_is_ignored() {
        let accounts = ingest(vec![
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
            Command::Dispute { client: 1, tx: 2 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), 50);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), 50);
        assert!(account.locked());
    }

    #[test]
    fn test_resolve_decreases_held_and_increases_available() {
        let accounts = ingest(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 1, tx: 1 },
            Command::Resolve { client: 1, tx: 1 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), 100);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), 100);
    }

    #[test]
    fn test_resolve_without_dispute_is_ignored() {
        let accounts = ingest(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Resolve { client: 1, tx: 1 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), 100);
        assert_eq!(account.held(), Amount::default());
    }

    #[test]
    fn test_resolve_on_wrong_client_is_ignored() {
        let accounts = ingest(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 1, tx: 1 },
            Command::Resolve { client: 2, tx: 1 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), 0);
        assert_eq!(account.held(), Amount::raw(100));
    }

    #[test]
    fn test_resolve_on_already_resolved_is_ignored() {
        let accounts = ingest(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 1, tx: 1 },
            Command::Resolve { client: 1, tx: 1 },
            Command::Resolve { client: 1, tx: 1 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), 100);
        assert_eq!(account.held(), Amount::default());
    }

    #[test]
    fn test_chargeback_locks_account_and_decreases_held_and_total() {
        let accounts = ingest(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 1, tx: 1 },
            Command::Chargeback { client: 1, tx: 1 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), 0);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), 0);
        assert!(account.locked());
    }

    #[test]
    fn test_chargeback_without_dispute_is_ignored() {
        let accounts = ingest(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Chargeback { client: 1, tx: 1 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert!(!account.locked());
    }

    #[test]
    fn test_chargeback_on_wrong_client_is_ignored() {
        let accounts = ingest(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 1, tx: 1 },
            Command::Chargeback { client: 2, tx: 1 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.held(), Amount::raw(100));
        assert!(!account.locked());
    }

    #[test]
    fn test_chargeback_on_already_chargedback_is_ignored() {
        let accounts = ingest(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 1, tx: 1 },
            Command::Chargeback { client: 1, tx: 1 },
            Command::Chargeback { client: 1, tx: 1 },
        ]);
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), 0);
        assert!(account.locked());
    }

    #[test]
    fn test_resolve_after_withdrawal_is_correct() {
        // Deposit 100, withdraw 80 (total=20), dispute the deposit, resolve.
        let accounts = ingest(vec![
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
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), 20);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), 20);
    }

    #[test]
    fn test_chargeback_of_deposit_after_withdrawal_total_is_negative() {
        // Deposit 100, withdraw 80 (total=20), dispute the deposit, chargeback.
        // total = 20 - 100 = -80: the account owes the bank the withdrawn funds.
        let accounts = ingest(vec![
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
        let account = accounts.get(&1).unwrap();
        assert_eq!(account.available(), -80);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), -80);
        assert!(account.locked());
    }

    #[test]
    fn test_replay_produces_same_state_as_process() {
        let commands = vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Withdrawal {
                client: 1,
                tx: 2,
                amount: Amount::raw(30),
            },
            Command::Deposit {
                client: 2,
                tx: 3,
                amount: Amount::raw(200),
            },
            Command::Dispute { client: 1, tx: 1 },
            Command::Resolve { client: 1, tx: 1 },
        ];

        // Process commands and capture the log.
        let mut ledger = Ledger::new();
        ledger.ingest(commands.into_iter());
        let log = ledger.into_log();

        // Replay the log into a fresh ledger.
        let replayed = Ledger::replay(log.into_iter()).into_accounts();

        let a1 = replayed.get(&1).unwrap();
        assert_eq!(a1.available(), 70);
        assert_eq!(a1.held(), Amount::default());
        assert_eq!(a1.total(), 70);

        let a2 = replayed.get(&2).unwrap();
        assert_eq!(a2.available(), 200);
        assert_eq!(a2.total(), 200);
    }
}
