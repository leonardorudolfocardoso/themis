use std::collections::HashMap;

use crate::account::Account;
use crate::command::Command;
use crate::event::{Decision, Event};
use crate::id::{ClientId, TransactionId};
use crate::transaction::{Kind, Record, State};

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
    log: Vec<Event>,
    accounts: HashMap<ClientId, Account>,
    records: HashMap<TransactionId, Record>,
}

impl Ledger {
    /// Creates a new ledger with no accounts or transaction history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Consumes all commands from `transactions` and returns the final account state.
    ///
    /// Each client account is created on first deposit or withdrawal.
    pub fn process(mut self, transactions: impl Iterator<Item = Command>) -> HashMap<u16, Account> {
        for command in transactions {
            if let Decision::Approved(event) = self.decide(&command) {
                self.log.push(event);
                self.apply(event);
            }
        }
        self.accounts
    }

    /// Validates a command against current state and returns a decision.
    ///
    /// This is a pure validation phase — it borrows `self` immutably.
    fn decide(&self, command: &Command) -> Decision {
        match command {
            Command::Deposit { client, tx, amount } => {
                if self.records.contains_key(tx) {
                    return Decision::Denied;
                }
                if self
                    .accounts
                    .get(client)
                    .is_some_and(|account| account.locked())
                {
                    return Decision::Denied;
                }
                Decision::Approved(Event::Deposited {
                    client: *client,
                    tx: *tx,
                    amount: *amount,
                })
            }
            Command::Withdrawal { client, tx, amount } => {
                if self.records.contains_key(tx) {
                    return Decision::Denied;
                }
                let account = self.accounts.get(client);
                if account.is_some_and(|a| a.locked()) {
                    return Decision::Denied;
                }
                if account
                    .map(|a| a.available() < *amount)
                    .unwrap_or(true)
                {
                    return Decision::Denied;
                }
                Decision::Approved(Event::Withdrawn {
                    client: *client,
                    tx: *tx,
                    amount: *amount,
                })
            }
            Command::Dispute { client, tx } => {
                let Some(record) = self.records.get(tx) else {
                    return Decision::Denied;
                };
                if self
                    .accounts
                    .get(client)
                    .is_some_and(|a| a.locked())
                {
                    return Decision::Denied;
                }
                if record.client != *client || !record.is_disputable() {
                    return Decision::Denied;
                }
                Decision::Approved(Event::DisputeOpened {
                    client: *client,
                    tx: *tx,
                    amount: record.amount,
                })
            }
            Command::Resolve { client, tx } => {
                let Some(record) = self.records.get(tx) else {
                    return Decision::Denied;
                };
                if self
                    .accounts
                    .get(client)
                    .is_some_and(|a| a.locked())
                {
                    return Decision::Denied;
                }
                if record.client != *client || !record.state.is_resolvable() {
                    return Decision::Denied;
                }
                Decision::Approved(Event::DisputeResolved {
                    client: *client,
                    tx: *tx,
                    amount: record.amount,
                })
            }
            Command::Chargeback { client, tx } => {
                let Some(record) = self.records.get(tx) else {
                    return Decision::Denied;
                };
                if self
                    .accounts
                    .get(client)
                    .is_some_and(|a| a.locked())
                {
                    return Decision::Denied;
                }
                if record.client != *client || !record.state.is_chargebackable() {
                    return Decision::Denied;
                }
                Decision::Approved(Event::ChargedBack {
                    client: *client,
                    tx: *tx,
                    amount: record.amount,
                })
            }
        }
    }

    /// Applies a validated event to accounts and records unconditionally.
    fn apply(&mut self, event: Event) {
        match event {
            Event::Deposited {
                client,
                tx,
                amount,
            } => {
                let account = self
                    .accounts
                    .entry(client)
                    .or_insert_with(|| Account::new(client));
                let _ = account.deposit(amount);
                self.records.insert(
                    tx,
                    Record {
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
                let account = self
                    .accounts
                    .get_mut(&client)
                    .expect("account must exist");
                let _ = account.withdraw(amount);
                self.records.insert(
                    tx,
                    Record {
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
                amount,
            } => {
                let record = self.records.get_mut(&tx).expect("record must exist");
                let account = self
                    .accounts
                    .get_mut(&record.client)
                    .expect("account must exist");
                record.state = State::Disputed;
                let _ = account.hold(amount);
            }
            Event::DisputeResolved {
                client: _,
                tx,
                amount,
            } => {
                let record = self.records.get_mut(&tx).expect("record must exist");
                let account = self
                    .accounts
                    .get_mut(&record.client)
                    .expect("account must exist");
                record.state = State::Resolved;
                let _ = account.release(amount);
            }
            Event::ChargedBack {
                client: _,
                tx,
                amount,
            } => {
                let record = self.records.get_mut(&tx).expect("record must exist");
                let account = self
                    .accounts
                    .get_mut(&record.client)
                    .expect("account must exist");
                record.state = State::Chargedback;
                let _ = account.chargeback(amount);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::Ledger;
    use crate::account::Account;
    use crate::amount::Amount;
    use crate::command::Command;

    fn process(transactions: Vec<Command>) -> HashMap<u16, Account> {
        Ledger::new().process(transactions.into_iter())
    }

    use std::collections::HashMap;

    #[test]
    fn test_duplicate_tx_id_is_ignored() {
        let accounts = process(vec![
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
        let accounts = process(vec![
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
        let accounts = process(vec![
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
        let accounts = process(vec![
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
        let accounts = process(vec![
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
        let accounts = process(vec![
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
        let accounts = process(vec![
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
        let accounts = process(vec![
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
        let accounts = process(vec![
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
        let accounts = process(vec![
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
        let accounts = process(vec![
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
        let accounts = process(vec![
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
        let accounts = process(vec![
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
        let accounts = process(vec![
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
        let accounts = process(vec![
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
        let accounts = process(vec![
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
        let accounts = process(vec![
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
        let accounts = process(vec![
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
        let accounts = process(vec![
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
}
