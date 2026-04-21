use std::collections::HashMap;

use crate::account::Account;
use crate::amount::Amount;
use crate::event::Event;
use crate::id::{ClientId, TransactionId};
use crate::transaction::{Kind, Record, State};

/// Processes a stream of transaction events and maintains account state.
///
/// `Processor` applies each [`Event`] to the corresponding [`Account`],
/// enforcing all transaction rules:
///
/// - Duplicate transaction IDs are silently ignored.
/// - Only deposits can be disputed; disputes on withdrawals are ignored.
/// - Disputes, resolves, and chargebacks must reference a transaction
///   belonging to the same client.
/// - All operations on locked accounts are silently ignored.
#[derive(Default)]
pub struct Processor {
    accounts: HashMap<ClientId, Account>,
    records: HashMap<TransactionId, Record>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyResult {
    Applied,
    Ignored,
}

impl Processor {
    /// Creates a new processor with no accounts or transaction history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Consumes all events from `transactions` and returns the final account state.
    ///
    /// Each client account is created on first deposit or withdrawal.
    pub fn process(mut self, transactions: impl Iterator<Item = Event>) -> HashMap<u16, Account> {
        for transaction in transactions {
            self.apply(transaction);
        }
        self.accounts
    }

    pub fn apply(&mut self, transaction: Event) -> ApplyResult {
        match transaction {
            Event::Deposit { client, tx, amount } => {
                if self.records.contains_key(&tx) {
                    ApplyResult::Ignored
                } else {
                    self.deposit(client, tx, amount)
                }
            }
            Event::Withdrawal { client, tx, amount } => {
                if self.records.contains_key(&tx) {
                    ApplyResult::Ignored
                } else {
                    self.withdraw(client, tx, amount)
                }
            }
            Event::Dispute { client, tx } => self.dispute(client, tx),
            Event::Resolve { client, tx } => self.resolve(client, tx),
            Event::Chargeback { client, tx } => self.chargeback(client, tx),
        }
    }

    fn deposit(&mut self, client: u16, tx: u32, amount: Amount) -> ApplyResult {
        let account = self
            .accounts
            .entry(client)
            .or_insert_with(|| Account::new(client));

        if account.deposit(amount).is_ok() {
            self.records.insert(
                tx,
                Record {
                    client,
                    amount,
                    kind: Kind::Deposit,
                    state: State::Valid,
                },
            );

            ApplyResult::Applied
        } else {
            ApplyResult::Ignored
        }
    }

    fn withdraw(&mut self, client: u16, tx: u32, amount: Amount) -> ApplyResult {
        let account = self
            .accounts
            .entry(client)
            .or_insert_with(|| Account::new(client));
        if account.withdraw(amount).is_ok() {
            self.records.insert(
                tx,
                Record {
                    client,
                    amount,
                    kind: Kind::Withdrawal,
                    state: State::Valid,
                },
            );

            ApplyResult::Applied
        } else {
            ApplyResult::Ignored
        }
    }

    fn dispute(&mut self, client: u16, tx: u32) -> ApplyResult {
        if let Some(record) = self.records.get_mut(&tx)
            && record.client == client
            && record.is_disputable()
            && let Some(account) = self.accounts.get_mut(&client)
            && account.hold(record.amount).is_ok()
        {
            record.state = State::Disputed;
            ApplyResult::Applied
        } else {
            ApplyResult::Ignored
        }
    }

    fn resolve(&mut self, client: u16, tx: u32) -> ApplyResult {
        if let Some(record) = self.records.get_mut(&tx)
            && record.client == client
            && record.state.is_resolvable()
            && let Some(account) = self.accounts.get_mut(&client)
            && account.release(record.amount).is_ok()
        {
            record.state = State::Resolved;
            ApplyResult::Applied
        } else {
            ApplyResult::Ignored
        }
    }

    fn chargeback(&mut self, client: u16, tx: u32) -> ApplyResult {
        if let Some(record) = self.records.get_mut(&tx)
            && record.client == client
            && record.state.is_chargebackable()
            && let Some(account) = self.accounts.get_mut(&client)
            && account.chargeback(record.amount).is_ok()
        {
            record.state = State::Chargedback;
            ApplyResult::Applied
        } else {
            ApplyResult::Ignored
        }
    }
}

#[cfg(test)]
mod test {
    use super::ApplyResult;
    use super::Processor;
    use crate::account::Account;
    use crate::amount::Amount;
    use crate::event::Event;

    fn processor_with(transactions: Vec<Event>) -> Processor {
        let mut processor = Processor::new();
        for transaction in transactions {
            processor.apply(transaction);
        }
        processor
    }

    fn account(processor: &Processor, client: u16) -> &Account {
        processor.accounts.get(&client).expect("account not found")
    }

    #[test]
    fn test_deposit_is_applied() {
        let mut processor = Processor::new();

        let result = processor.apply(Event::Deposit {
            client: 1,
            tx: 1,
            amount: Amount::raw(100),
        });

        assert_eq!(result, ApplyResult::Applied);
        let account = account(&processor, 1);
        assert_eq!(account.available(), 100);
        assert_eq!(account.total(), 100);
    }

    #[test]
    fn test_withdrawal_is_applied() {
        let mut processor = processor_with(vec![Event::Deposit {
            client: 1,
            tx: 1,
            amount: Amount::raw(100),
        }]);

        let result = processor.apply(Event::Withdrawal {
            client: 1,
            tx: 2,
            amount: Amount::raw(20),
        });

        assert_eq!(result, ApplyResult::Applied);
        let account = account(&processor, 1);
        assert_eq!(account.available(), 80);
        assert_eq!(account.total(), 80);
    }

    #[test]
    fn test_duplicate_tx_id_is_ignored() {
        let mut processor = processor_with(vec![Event::Deposit {
            client: 1,
            tx: 1,
            amount: Amount::raw(100),
        }]);

        let result = processor.apply(Event::Deposit {
            client: 1,
            tx: 1,
            amount: Amount::raw(999),
        });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&processor, 1);
        assert_eq!(account.available(), 100);
        assert_eq!(account.total(), 100);
    }

    #[test]
    fn test_duplicate_withdrawal_tx_id_is_ignored() {
        let mut processor = processor_with(vec![
            Event::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Event::Withdrawal {
                client: 1,
                tx: 2,
                amount: Amount::raw(20),
            },
        ]);

        let result = processor.apply(Event::Withdrawal {
            client: 1,
            tx: 2,
            amount: Amount::raw(20),
        });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&processor, 1);
        assert_eq!(account.available(), 80);
        assert_eq!(account.total(), 80);
    }

    #[test]
    fn test_deposit_on_locked_account_is_ignored() {
        let mut processor = processor_with(vec![
            Event::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Event::Dispute { client: 1, tx: 1 },
            Event::Chargeback { client: 1, tx: 1 },
        ]);

        let result = processor.apply(Event::Deposit {
            client: 1,
            tx: 2,
            amount: Amount::raw(50),
        });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&processor, 1);
        assert_eq!(account.available(), 0);
        assert_eq!(account.total(), 0);
        assert!(account.locked());
    }

    #[test]
    fn test_withdraw_locked_is_silently_ignored() {
        let mut processor = processor_with(vec![
            Event::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Event::Dispute { client: 1, tx: 1 },
            Event::Chargeback { client: 1, tx: 1 },
        ]);

        let result = processor.apply(Event::Withdrawal {
            client: 1,
            tx: 2,
            amount: Amount::raw(50),
        });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&processor, 1);
        assert_eq!(account.total(), 0);
        assert!(account.locked());
    }

    #[test]
    fn test_dispute_on_withdrawal_is_ignored() {
        let mut processor = processor_with(vec![
            Event::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Event::Withdrawal {
                client: 1,
                tx: 2,
                amount: Amount::raw(40),
            },
        ]);

        let result = processor.apply(Event::Dispute { client: 1, tx: 2 });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&processor, 1);
        assert_eq!(account.available(), 60);
        assert_eq!(account.held(), Amount::default());
    }

    #[test]
    fn test_dispute_decreases_available_and_increases_held() {
        let mut processor = processor_with(vec![Event::Deposit {
            client: 1,
            tx: 1,
            amount: Amount::raw(100),
        }]);

        let result = processor.apply(Event::Dispute { client: 1, tx: 1 });

        assert_eq!(result, ApplyResult::Applied);
        let account = account(&processor, 1);
        assert_eq!(account.available(), 0);
        assert_eq!(account.held(), Amount::raw(100));
        assert_eq!(account.total(), 100);
    }

    #[test]
    fn test_dispute_on_wrong_client_is_ignored() {
        let mut processor = processor_with(vec![Event::Deposit {
            client: 1,
            tx: 1,
            amount: Amount::raw(100),
        }]);

        let result = processor.apply(Event::Dispute { client: 2, tx: 1 });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&processor, 1);
        assert_eq!(account.available(), 100);
        assert_eq!(account.held(), Amount::default());
    }

    #[test]
    fn test_dispute_on_invalid_state_is_ignored() {
        let mut processor = processor_with(vec![
            Event::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Event::Dispute { client: 1, tx: 1 },
        ]);

        let result = processor.apply(Event::Dispute { client: 1, tx: 1 });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&processor, 1);
        assert_eq!(account.available(), 0);
        assert_eq!(account.held(), Amount::raw(100));
    }

    #[test]
    fn test_dispute_on_locked_account_is_ignored() {
        let mut processor = processor_with(vec![
            Event::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Event::Deposit {
                client: 1,
                tx: 2,
                amount: Amount::raw(50),
            },
            Event::Dispute { client: 1, tx: 1 },
            Event::Chargeback { client: 1, tx: 1 },
        ]);

        let result = processor.apply(Event::Dispute { client: 1, tx: 2 });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&processor, 1);
        assert_eq!(account.available(), 50);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), 50);
        assert!(account.locked());
    }

    #[test]
    fn test_resolve_decreases_held_and_increases_available() {
        let mut processor = processor_with(vec![
            Event::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Event::Dispute { client: 1, tx: 1 },
        ]);

        let result = processor.apply(Event::Resolve { client: 1, tx: 1 });

        assert_eq!(result, ApplyResult::Applied);
        let account = account(&processor, 1);
        assert_eq!(account.available(), 100);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), 100);
    }

    #[test]
    fn test_resolve_without_dispute_is_ignored() {
        let mut processor = processor_with(vec![Event::Deposit {
            client: 1,
            tx: 1,
            amount: Amount::raw(100),
        }]);

        let result = processor.apply(Event::Resolve { client: 1, tx: 1 });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&processor, 1);
        assert_eq!(account.available(), 100);
        assert_eq!(account.held(), Amount::default());
    }

    #[test]
    fn test_resolve_on_wrong_client_is_ignored() {
        let mut processor = processor_with(vec![
            Event::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Event::Dispute { client: 1, tx: 1 },
        ]);

        let result = processor.apply(Event::Resolve { client: 2, tx: 1 });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&processor, 1);
        assert_eq!(account.available(), 0);
        assert_eq!(account.held(), Amount::raw(100));
    }

    #[test]
    fn test_resolve_on_already_resolved_is_ignored() {
        let mut processor = processor_with(vec![
            Event::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Event::Dispute { client: 1, tx: 1 },
            Event::Resolve { client: 1, tx: 1 },
        ]);

        let result = processor.apply(Event::Resolve { client: 1, tx: 1 });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&processor, 1);
        assert_eq!(account.available(), 100);
        assert_eq!(account.held(), Amount::default());
    }

    #[test]
    fn test_chargeback_locks_account_and_decreases_held_and_total() {
        let mut processor = processor_with(vec![
            Event::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Event::Dispute { client: 1, tx: 1 },
        ]);

        let result = processor.apply(Event::Chargeback { client: 1, tx: 1 });

        assert_eq!(result, ApplyResult::Applied);
        let account = account(&processor, 1);
        assert_eq!(account.available(), 0);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), 0);
        assert!(account.locked());
    }

    #[test]
    fn test_chargeback_without_dispute_is_ignored() {
        let mut processor = processor_with(vec![Event::Deposit {
            client: 1,
            tx: 1,
            amount: Amount::raw(100),
        }]);

        let result = processor.apply(Event::Chargeback { client: 1, tx: 1 });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&processor, 1);
        assert!(!account.locked());
    }

    #[test]
    fn test_chargeback_on_wrong_client_is_ignored() {
        let mut processor = processor_with(vec![
            Event::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Event::Dispute { client: 1, tx: 1 },
        ]);

        let result = processor.apply(Event::Chargeback { client: 2, tx: 1 });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&processor, 1);
        assert_eq!(account.held(), Amount::raw(100));
        assert!(!account.locked());
    }

    #[test]
    fn test_chargeback_on_already_chargedback_is_ignored() {
        let mut processor = processor_with(vec![
            Event::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Event::Dispute { client: 1, tx: 1 },
            Event::Chargeback { client: 1, tx: 1 },
        ]);

        let result = processor.apply(Event::Chargeback { client: 1, tx: 1 });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&processor, 1);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), 0);
        assert!(account.locked());
    }

    #[test]
    fn test_resolve_after_withdrawal_is_correct() {
        // Deposit 100, withdraw 80 (total=20), dispute the deposit, resolve.
        let processor = processor_with(vec![
            Event::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Event::Withdrawal {
                client: 1,
                tx: 2,
                amount: Amount::raw(80),
            },
            Event::Dispute { client: 1, tx: 1 },
            Event::Resolve { client: 1, tx: 1 },
        ]);
        let account = account(&processor, 1);
        assert_eq!(account.available(), 20);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), 20);
    }

    #[test]
    fn test_chargeback_of_deposit_after_withdrawal_total_is_negative() {
        // Deposit 100, withdraw 80 (total=20), dispute the deposit, chargeback.
        // total = 20 - 100 = -80: the account owes the bank the withdrawn funds.
        let processor = processor_with(vec![
            Event::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Event::Withdrawal {
                client: 1,
                tx: 2,
                amount: Amount::raw(80),
            },
            Event::Dispute { client: 1, tx: 1 },
            Event::Chargeback { client: 1, tx: 1 },
        ]);
        let account = account(&processor, 1);
        assert_eq!(account.available(), -80);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), -80);
        assert!(account.locked());
    }
}
