use crate::account::Account;
use crate::amount::Amount;
use crate::command::Command;
use crate::event::Event;
use crate::id::ClientId;
use crate::projection::LedgerProjection;

/// Processes a stream of transaction commands and maintains account state.
///
/// `Processor` applies each [`Command`] to the corresponding [`Account`],
/// enforcing all transaction rules:
///
/// - Duplicate transaction IDs are silently ignored.
/// - Only deposits can be disputed; disputes on withdrawals are ignored.
/// - Disputes, resolves, and chargebacks must reference a transaction
///   belonging to the same client.
/// - All operations on locked accounts are silently ignored.
#[derive(Default)]
pub struct Processor {
    projection: LedgerProjection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyResult {
    Applied,
    Ignored,
}

enum Decision {
    Apply(Event),
    Ignore,
}

impl Processor {
    /// Creates a new processor with no accounts or transaction history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Consumes all events from `transactions` and returns the final account state.
    ///
    /// Each client account is created on first deposit or withdrawal.
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

    pub fn apply(&mut self, transaction: Command) -> ApplyResult {
        match self.decide(transaction) {
            Decision::Apply(event) => {
                self.projection.apply(event);
                ApplyResult::Applied
            }
            Decision::Ignore => ApplyResult::Ignored,
        }
    }

    fn decide(&mut self, transaction: Command) -> Decision {
        match transaction {
            Command::Deposit { client, tx, amount } => {
                if self.projection.has_record(tx) {
                    Decision::Ignore
                } else {
                    self.decide_deposit(client, tx, amount)
                }
            }
            Command::Withdrawal { client, tx, amount } => {
                if self.projection.has_record(tx) {
                    Decision::Ignore
                } else {
                    self.decide_withdrawal(client, tx, amount)
                }
            }
            Command::Dispute { client, tx } => self.decide_dispute(client, tx),
            Command::Resolve { client, tx } => self.decide_resolve(client, tx),
            Command::Chargeback { client, tx } => self.decide_chargeback(client, tx),
        }
    }

    fn decide_deposit(&mut self, client: u16, tx: u32, amount: Amount) -> Decision {
        if self
            .projection
            .account(client)
            .is_some_and(|account| account.locked())
        {
            return Decision::Ignore;
        }

        Decision::Apply(Event::DepositAccepted { client, tx, amount })
    }

    fn decide_withdrawal(&mut self, client: u16, tx: u32, amount: Amount) -> Decision {
        let account = self.projection.account_mut_or_create(client);
        if account.locked() || account.available() < amount {
            return Decision::Ignore;
        }

        Decision::Apply(Event::WithdrawalAccepted { client, tx, amount })
    }

    fn decide_dispute(&mut self, client: u16, tx: u32) -> Decision {
        let amount = match self.projection.record(tx) {
            Some(record) if record.client == client && record.is_disputable() => record.amount,
            _ => return Decision::Ignore,
        };

        if self
            .projection
            .account(client)
            .is_some_and(|account| account.locked())
        {
            return Decision::Ignore;
        }

        Decision::Apply(Event::DepositDisputed { client, tx, amount })
    }

    fn decide_resolve(&mut self, client: u16, tx: u32) -> Decision {
        let amount = match self.projection.record(tx) {
            Some(record) if record.client == client && record.state.is_resolvable() => {
                record.amount
            }
            _ => return Decision::Ignore,
        };

        if self
            .projection
            .account(client)
            .is_some_and(|account| account.locked())
        {
            return Decision::Ignore;
        }

        Decision::Apply(Event::DisputeResolved { client, tx, amount })
    }

    fn decide_chargeback(&mut self, client: u16, tx: u32) -> Decision {
        let amount = match self.projection.record(tx) {
            Some(record) if record.client == client && record.state.is_chargebackable() => {
                record.amount
            }
            _ => return Decision::Ignore,
        };

        if self
            .projection
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
    use super::ApplyResult;
    use super::Processor;
    use crate::account::Account;
    use crate::amount::Amount;
    use crate::command::Command;

    fn processor_with(transactions: Vec<Command>) -> Processor {
        let mut processor = Processor::new();
        for transaction in transactions {
            processor.apply(transaction);
        }
        processor
    }

    fn account(processor: &Processor, client: u16) -> &Account {
        processor.account(client).expect("account not found")
    }

    #[test]
    fn test_deposit_is_applied() {
        let mut processor = Processor::new();

        let result = processor.apply(Command::Deposit {
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
        let mut processor = processor_with(vec![Command::Deposit {
            client: 1,
            tx: 1,
            amount: Amount::raw(100),
        }]);

        let result = processor.apply(Command::Withdrawal {
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
        let mut processor = processor_with(vec![Command::Deposit {
            client: 1,
            tx: 1,
            amount: Amount::raw(100),
        }]);

        let result = processor.apply(Command::Deposit {
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

        let result = processor.apply(Command::Withdrawal {
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
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 1, tx: 1 },
            Command::Chargeback { client: 1, tx: 1 },
        ]);

        let result = processor.apply(Command::Deposit {
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
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 1, tx: 1 },
            Command::Chargeback { client: 1, tx: 1 },
        ]);

        let result = processor.apply(Command::Withdrawal {
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

        let result = processor.apply(Command::Dispute { client: 1, tx: 2 });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&processor, 1);
        assert_eq!(account.available(), 60);
        assert_eq!(account.held(), Amount::default());
    }

    #[test]
    fn test_dispute_decreases_available_and_increases_held() {
        let mut processor = processor_with(vec![Command::Deposit {
            client: 1,
            tx: 1,
            amount: Amount::raw(100),
        }]);

        let result = processor.apply(Command::Dispute { client: 1, tx: 1 });

        assert_eq!(result, ApplyResult::Applied);
        let account = account(&processor, 1);
        assert_eq!(account.available(), 0);
        assert_eq!(account.held(), Amount::raw(100));
        assert_eq!(account.total(), 100);
    }

    #[test]
    fn test_dispute_on_wrong_client_is_ignored() {
        let mut processor = processor_with(vec![Command::Deposit {
            client: 1,
            tx: 1,
            amount: Amount::raw(100),
        }]);

        let result = processor.apply(Command::Dispute { client: 2, tx: 1 });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&processor, 1);
        assert_eq!(account.available(), 100);
        assert_eq!(account.held(), Amount::default());
    }

    #[test]
    fn test_dispute_on_invalid_state_is_ignored() {
        let mut processor = processor_with(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 1, tx: 1 },
        ]);

        let result = processor.apply(Command::Dispute { client: 1, tx: 1 });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&processor, 1);
        assert_eq!(account.available(), 0);
        assert_eq!(account.held(), Amount::raw(100));
    }

    #[test]
    fn test_dispute_on_locked_account_is_ignored() {
        let mut processor = processor_with(vec![
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

        let result = processor.apply(Command::Dispute { client: 1, tx: 2 });

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
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 1, tx: 1 },
        ]);

        let result = processor.apply(Command::Resolve { client: 1, tx: 1 });

        assert_eq!(result, ApplyResult::Applied);
        let account = account(&processor, 1);
        assert_eq!(account.available(), 100);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), 100);
    }

    #[test]
    fn test_resolve_without_dispute_is_ignored() {
        let mut processor = processor_with(vec![Command::Deposit {
            client: 1,
            tx: 1,
            amount: Amount::raw(100),
        }]);

        let result = processor.apply(Command::Resolve { client: 1, tx: 1 });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&processor, 1);
        assert_eq!(account.available(), 100);
        assert_eq!(account.held(), Amount::default());
    }

    #[test]
    fn test_resolve_on_wrong_client_is_ignored() {
        let mut processor = processor_with(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 1, tx: 1 },
        ]);

        let result = processor.apply(Command::Resolve { client: 2, tx: 1 });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&processor, 1);
        assert_eq!(account.available(), 0);
        assert_eq!(account.held(), Amount::raw(100));
    }

    #[test]
    fn test_resolve_on_already_resolved_is_ignored() {
        let mut processor = processor_with(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 1, tx: 1 },
            Command::Resolve { client: 1, tx: 1 },
        ]);

        let result = processor.apply(Command::Resolve { client: 1, tx: 1 });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&processor, 1);
        assert_eq!(account.available(), 100);
        assert_eq!(account.held(), Amount::default());
    }

    #[test]
    fn test_chargeback_locks_account_and_decreases_held_and_total() {
        let mut processor = processor_with(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 1, tx: 1 },
        ]);

        let result = processor.apply(Command::Chargeback { client: 1, tx: 1 });

        assert_eq!(result, ApplyResult::Applied);
        let account = account(&processor, 1);
        assert_eq!(account.available(), 0);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), 0);
        assert!(account.locked());
    }

    #[test]
    fn test_chargeback_without_dispute_is_ignored() {
        let mut processor = processor_with(vec![Command::Deposit {
            client: 1,
            tx: 1,
            amount: Amount::raw(100),
        }]);

        let result = processor.apply(Command::Chargeback { client: 1, tx: 1 });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&processor, 1);
        assert!(!account.locked());
    }

    #[test]
    fn test_chargeback_on_wrong_client_is_ignored() {
        let mut processor = processor_with(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 1, tx: 1 },
        ]);

        let result = processor.apply(Command::Chargeback { client: 2, tx: 1 });

        assert_eq!(result, ApplyResult::Ignored);
        let account = account(&processor, 1);
        assert_eq!(account.held(), Amount::raw(100));
        assert!(!account.locked());
    }

    #[test]
    fn test_chargeback_on_already_chargedback_is_ignored() {
        let mut processor = processor_with(vec![
            Command::Deposit {
                client: 1,
                tx: 1,
                amount: Amount::raw(100),
            },
            Command::Dispute { client: 1, tx: 1 },
            Command::Chargeback { client: 1, tx: 1 },
        ]);

        let result = processor.apply(Command::Chargeback { client: 1, tx: 1 });

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
        let account = account(&processor, 1);
        assert_eq!(account.available(), -80);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), -80);
        assert!(account.locked());
    }
}
