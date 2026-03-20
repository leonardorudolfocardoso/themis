use crate::amount::Amount;
use crate::balance::{Balance, BalanceError};
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

#[cfg(test)]
mod test {
    use super::{Account, AccountError};
    use crate::amount::Amount;

    #[test]
    fn test_deposit_increases_available_and_total() {
        let mut account = Account::new(1);
        assert!(account.deposit(Amount::raw(100)).is_ok());
        assert_eq!(account.available(), 100);
        assert_eq!(account.total(), 100);
        assert_eq!(account.held(), Amount::default());
    }

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
    fn test_withdraw_decreases_available_and_total() {
        let mut account = Account::new(1);
        account.deposit(Amount::raw(100)).unwrap();
        assert!(account.withdraw(Amount::raw(40)).is_ok());
        assert_eq!(account.available(), 60);
        assert_eq!(account.total(), 60);
    }

    #[test]
    fn test_withdraw_insufficient_funds_returns_error() {
        let mut account = Account::new(1);
        account.deposit(Amount::raw(100)).unwrap();
        assert!(matches!(
            account.withdraw(Amount::raw(200)),
            Err(AccountError::InsufficientFunds)
        ));
        assert_eq!(account.available(), 100);
        assert_eq!(account.total(), 100);
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
    fn test_hold_moves_available_to_held() {
        let mut account = Account::new(1);
        account.deposit(Amount::raw(100)).unwrap();
        account.hold(Amount::raw(40)).unwrap();
        assert_eq!(account.available(), 60);
        assert_eq!(account.held(), Amount::raw(40));
        assert_eq!(account.total(), 100);
    }

    #[test]
    fn test_release_moves_held_to_available() {
        let mut account = Account::new(1);
        account.deposit(Amount::raw(100)).unwrap();
        account.hold(Amount::raw(40)).unwrap();
        account.release(Amount::raw(40)).unwrap();
        assert_eq!(account.available(), 100);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), 100);
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
    fn test_chargeback_after_withdrawal_total_is_negative() {
        // Deposit 100, withdraw 80 (total=20), hold 100, chargeback 100.
        // total = 20 - 100 = -80: the account owes the bank the withdrawn funds.
        let mut account = Account::new(1);
        account.deposit(Amount::raw(100)).unwrap();
        account.withdraw(Amount::raw(80)).unwrap();
        account.hold(Amount::raw(100)).unwrap();
        account.chargeback(Amount::raw(100)).unwrap();
        assert_eq!(account.available(), -80);
        assert_eq!(account.held(), Amount::default());
        assert_eq!(account.total(), -80);
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
