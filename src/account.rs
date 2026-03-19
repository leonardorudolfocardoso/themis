use crate::amount::Amount;
use crate::balance::{Balance, BalanceError};
use crate::funds::Funds;

pub struct Account {
    client: u16,
    balance: Balance,
    locked: bool,
}

#[derive(Debug)]
pub(crate) enum AccountError {
    Locked,
    InsufficientFunds,
}

impl Account {
    pub(crate) fn new(client: u16) -> Self {
        Self {
            client,
            balance: Balance::new(),
            locked: false,
        }
    }

    pub fn client(&self) -> u16 {
        self.client
    }

    pub fn available(&self) -> Funds {
        self.balance.available()
    }

    pub fn held(&self) -> Amount {
        self.balance.held()
    }

    pub fn total(&self) -> Funds {
        self.balance.total()
    }

    pub fn locked(&self) -> bool {
        self.locked
    }

    pub(crate) fn deposit(&mut self, amount: Amount) -> Result<(), AccountError> {
        if self.locked {
            return Err(AccountError::Locked);
        }
        self.balance.deposit(amount);
        Ok(())
    }

    pub(crate) fn withdraw(&mut self, amount: Amount) -> Result<(), AccountError> {
        if self.locked {
            return Err(AccountError::Locked);
        }
        self.balance.withdraw(amount).map_err(|e| match e {
            BalanceError::InsufficientFunds => AccountError::InsufficientFunds,
        })
    }

    pub(crate) fn hold(&mut self, amount: Amount) {
        self.balance.hold(amount);
    }

    pub(crate) fn release(&mut self, amount: Amount) {
        self.balance.release(amount);
    }

    pub(crate) fn chargeback(&mut self, amount: Amount) {
        self.balance.chargeback(amount);
        self.locked = true;
    }
}

#[cfg(test)]
mod test {
    use super::{Account, AccountError};
    use crate::amount::Amount;

    #[test]
    fn test_deposit_increases_available_and_total() {
        let mut account = Account::new(1);
        assert!(account.deposit(Amount::from(100)).is_ok());
        assert_eq!(account.available(), 100);
        assert_eq!(account.total(), 100);
        assert_eq!(account.held(), 0);
    }

    #[test]
    fn test_deposit_on_locked_account_returns_error() {
        let mut account = Account::new(1);
        account.deposit(Amount::from(100)).unwrap();
        account.hold(Amount::from(100));
        account.chargeback(Amount::from(100));
        assert!(matches!(account.deposit(Amount::from(50)), Err(AccountError::Locked)));
        assert_eq!(account.available(), 0);
        assert_eq!(account.total(), 0);
    }

    #[test]
    fn test_withdraw_decreases_available_and_total() {
        let mut account = Account::new(1);
        account.deposit(Amount::from(100)).unwrap();
        assert!(account.withdraw(Amount::from(40)).is_ok());
        assert_eq!(account.available(), 60);
        assert_eq!(account.total(), 60);
    }

    #[test]
    fn test_withdraw_insufficient_funds_returns_error() {
        let mut account = Account::new(1);
        account.deposit(Amount::from(100)).unwrap();
        assert!(matches!(account.withdraw(Amount::from(200)), Err(AccountError::InsufficientFunds)));
        assert_eq!(account.available(), 100);
        assert_eq!(account.total(), 100);
    }

    #[test]
    fn test_withdraw_on_locked_account_returns_error() {
        let mut account = Account::new(1);
        account.deposit(Amount::from(100)).unwrap();
        account.hold(Amount::from(100));
        account.chargeback(Amount::from(100));
        assert!(matches!(account.withdraw(Amount::from(50)), Err(AccountError::Locked)));
    }

    #[test]
    fn test_hold_moves_available_to_held() {
        let mut account = Account::new(1);
        account.deposit(Amount::from(100)).unwrap();
        account.hold(Amount::from(40));
        assert_eq!(account.available(), 60);
        assert_eq!(account.held(), 40);
        assert_eq!(account.total(), 100);
    }

    #[test]
    fn test_release_moves_held_to_available() {
        let mut account = Account::new(1);
        account.deposit(Amount::from(100)).unwrap();
        account.hold(Amount::from(40));
        account.release(Amount::from(40));
        assert_eq!(account.available(), 100);
        assert_eq!(account.held(), 0);
        assert_eq!(account.total(), 100);
    }

    #[test]
    fn test_chargeback_removes_held_and_total_and_locks() {
        let mut account = Account::new(1);
        account.deposit(Amount::from(100)).unwrap();
        account.hold(Amount::from(100));
        account.chargeback(Amount::from(100));
        assert_eq!(account.held(), 0);
        assert_eq!(account.total(), 0);
        assert!(account.locked());
    }

    #[test]
    fn test_chargeback_after_withdrawal_total_is_negative() {
        // Deposit 100, withdraw 80 (total=20), hold 100, chargeback 100.
        // total = 20 - 100 = -80: the account owes the bank the withdrawn funds.
        let mut account = Account::new(1);
        account.deposit(Amount::from(100)).unwrap();
        account.withdraw(Amount::from(80)).unwrap();
        account.hold(Amount::from(100));
        account.chargeback(Amount::from(100));
        assert_eq!(account.available(), -80);
        assert_eq!(account.held(), 0);
        assert_eq!(account.total(), -80);
        assert!(account.locked());
    }
}
