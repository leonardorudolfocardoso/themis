use std::collections::HashMap;

use crate::amount::Amount;
use crate::balance::{Balance, BalanceError};
use crate::event::Event;
use crate::funds::Funds;
use crate::id::ClientId;

/// Error type for account operations.
#[derive(Debug)]
pub(crate) enum AccountError {
    /// The account is locked and rejects all mutations.
    Locked,
    /// The requested withdrawal exceeds the available funds.
    InsufficientFunds,
}

/// Represents a client account with a balance and a locked state.
///
/// An `Account` wraps a `Balance` and enforces one additional invariant:
/// a locked account rejects all mutations. Locking happens
/// permanently after a chargeback and cannot be undone.
///
/// Two distinct states govern fund availability:
/// - **Frozen funds** — temporarily unavailable for withdrawal; reversible via `release`.
/// - **Locked account** — permanently rejects all account mutations; set by `chargeback`.
pub struct Account {
    /// The client this account belongs to.
    client: ClientId,
    /// The monetary state of the account.
    balance: Balance,
    /// Whether the account has been locked by a chargeback.
    locked: bool,
}

impl Account {
    /// Creates a new account for `client` with zero balance and unlocked state.
    pub(crate) fn new(client: u16) -> Self {
        Self {
            client,
            balance: Balance::new(),
            locked: false,
        }
    }

    /// Returns the client ID this account belongs to.
    pub fn client(&self) -> u16 {
        self.client
    }

    /// Returns the funds available for withdrawal.
    pub fn available(&self) -> Funds {
        self.balance.available()
    }

    /// Returns the funds currently held.
    pub fn held(&self) -> Amount {
        self.balance.held()
    }

    /// Returns the total balance (`available + held`).
    pub fn total(&self) -> Funds {
        self.balance.total()
    }

    /// Returns `true` if the account has been locked by a chargeback.
    pub fn locked(&self) -> bool {
        self.locked
    }

    /// Increases available funds by `amount`.
    ///
    /// Returns [`AccountError::Locked`] if the account is locked.
    pub(crate) fn deposit(&mut self, amount: Amount) -> Result<(), AccountError> {
        if self.locked {
            return Err(AccountError::Locked);
        }
        self.balance.deposit(amount);
        Ok(())
    }

    /// Decreases available funds by `amount`.
    ///
    /// Returns [`AccountError::Locked`] if the account is locked, or
    /// [`AccountError::InsufficientFunds`] if `amount` exceeds available funds.
    pub(crate) fn withdraw(&mut self, amount: Amount) -> Result<(), AccountError> {
        if self.locked {
            return Err(AccountError::Locked);
        }
        self.balance.withdraw(amount).map_err(|e| match e {
            BalanceError::InsufficientFunds => AccountError::InsufficientFunds,
        })
    }

    /// Moves `amount` from available to held.
    /// Freezes `amount` of the account's funds, preventing withdrawal.
    ///
    /// Returns [`AccountError::Locked`] if the account is locked.
    pub(crate) fn hold(&mut self, amount: Amount) -> Result<(), AccountError> {
        if self.locked {
            return Err(AccountError::Locked);
        }
        self.balance.hold(amount);
        Ok(())
    }

    /// Unfreezes `amount`, making it available for withdrawal again.
    ///
    /// Returns [`AccountError::Locked`] if the account is locked.
    pub(crate) fn release(&mut self, amount: Amount) -> Result<(), AccountError> {
        if self.locked {
            return Err(AccountError::Locked);
        }
        self.balance.release(amount);
        Ok(())
    }

    /// Permanently deducts the `amount` previously frozen by [`Account::hold`] and locks the
    /// account, rejecting all future mutations.
    ///
    /// Can leave the available balance negative if funds were withdrawn before
    /// the freeze.
    pub(crate) fn chargeback(&mut self, amount: Amount) -> Result<(), AccountError> {
        if self.locked {
            return Err(AccountError::Locked);
        }
        self.balance.chargeback(amount);
        self.locked = true;
        Ok(())
    }
}

/// All client accounts, indexed by client ID — the account aggregate.
///
/// Owns balance state and locked status. Used by the ledger to validate
/// commands against account state and to produce the final output.
#[derive(Default)]
pub struct Accounts(HashMap<ClientId, Account>);

impl Accounts {
    /// Returns a reference to the account for the given client, if it exists.
    pub fn get(&self, client: &ClientId) -> Option<&Account> {
        self.0.get(client)
    }

    /// Returns `true` if the account exists and is locked. Returns `false` for unknown clients.
    pub(crate) fn is_locked(&self, client: &ClientId) -> bool {
        self.0.get(client).is_some_and(|a| a.locked)
    }

    /// Returns `true` if the client can withdraw `amount`.
    ///
    /// Requires the account to exist, not be locked, and have sufficient available funds.
    pub(crate) fn can_withdraw(&self, client: &ClientId, amount: Amount) -> bool {
        self.0
            .get(client)
            .is_some_and(|a| !a.locked && a.available() >= amount)
    }

    /// Applies a validated event, updating account state.
    pub(crate) fn apply(&mut self, event: Event) {
        match event {
            Event::Deposited {
                client,
                tx: _,
                amount,
            } => {
                let account = self.0.entry(client).or_insert_with(|| Account::new(client));
                let _ = account.deposit(amount);
            }
            Event::Withdrawn {
                client,
                tx: _,
                amount,
            } => {
                let account = self.0.get_mut(&client).expect("account must exist");
                let _ = account.withdraw(amount);
            }
            Event::DisputeOpened {
                client,
                tx: _,
                amount,
            } => {
                let account = self.0.get_mut(&client).expect("account must exist");
                let _ = account.hold(amount);
            }
            Event::DisputeResolved {
                client,
                tx: _,
                amount,
            } => {
                let account = self.0.get_mut(&client).expect("account must exist");
                let _ = account.release(amount);
            }
            Event::ChargedBack {
                client,
                tx: _,
                amount,
            } => {
                let account = self.0.get_mut(&client).expect("account must exist");
                let _ = account.chargeback(amount);
            }
        }
    }
}

impl IntoIterator for Accounts {
    type Item = Account;
    type IntoIter = std::collections::hash_map::IntoValues<ClientId, Account>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_values()
    }
}

#[cfg(test)]
mod test {
    use super::{Account, AccountError};
    use crate::amount::Amount;

    #[test]
    fn test_deposit_on_locked_account_returns_error() {
        let mut account = Account::new(1);
        account.deposit(Amount::raw(100)).unwrap();
        account.hold(Amount::raw(100)).unwrap();
        account.chargeback(Amount::raw(100)).unwrap();
        assert!(matches!(
            account.deposit(Amount::raw(50)),
            Err(AccountError::Locked)
        ));
        assert_eq!(account.available(), 0);
        assert_eq!(account.total(), 0);
    }

    #[test]
    fn test_withdraw_on_locked_account_returns_error() {
        let mut account = Account::new(1);
        account.deposit(Amount::raw(100)).unwrap();
        account.hold(Amount::raw(100)).unwrap();
        account.chargeback(Amount::raw(100)).unwrap();
        assert!(matches!(
            account.withdraw(Amount::raw(50)),
            Err(AccountError::Locked)
        ));
    }

    #[test]
    fn test_chargeback_removes_held_and_total_and_locks() {
        let mut account = Account::new(1);
        account.deposit(Amount::raw(100)).unwrap();
        account.hold(Amount::raw(100)).unwrap();
        account.chargeback(Amount::raw(100)).unwrap();
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), 0);
        assert!(account.locked());
    }

    #[test]
    fn test_hold_on_locked_account_returns_error() {
        let mut account = Account::new(1);
        account.deposit(Amount::raw(100)).unwrap();
        account.hold(Amount::raw(100)).unwrap();
        account.chargeback(Amount::raw(100)).unwrap();
        assert!(matches!(
            account.hold(Amount::raw(10)),
            Err(AccountError::Locked)
        ));
    }

    #[test]
    fn test_release_on_locked_account_returns_error() {
        let mut account = Account::new(1);
        account.deposit(Amount::raw(100)).unwrap();
        account.hold(Amount::raw(100)).unwrap();
        account.chargeback(Amount::raw(100)).unwrap();
        assert!(matches!(
            account.release(Amount::raw(10)),
            Err(AccountError::Locked)
        ));
    }

    #[test]
    fn test_chargeback_on_locked_account_returns_error() {
        let mut account = Account::new(1);
        account.deposit(Amount::raw(100)).unwrap();
        account.hold(Amount::raw(100)).unwrap();
        account.chargeback(Amount::raw(100)).unwrap();
        assert!(matches!(
            account.chargeback(Amount::raw(10)),
            Err(AccountError::Locked)
        ));
    }
}
