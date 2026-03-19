/// Monetary amounts are stored as integer units of 0.0001 (4 decimal places).
/// e.g. 1.2345 is represented as 12345.
pub struct Account {
    client: u16,
    available: i64,
    held: u64,
    total: i64,
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
            available: 0,
            held: 0,
            total: 0,
            locked: false,
        }
    }

    pub fn client(&self) -> u16 {
        self.client
    }

    pub fn available(&self) -> i64 {
        self.available
    }

    pub fn held(&self) -> u64 {
        self.held
    }

    pub fn total(&self) -> i64 {
        self.total
    }

    pub fn locked(&self) -> bool {
        self.locked
    }

    pub(crate) fn deposit(&mut self, amount: u64) -> Result<(), AccountError> {
        if self.locked {
            return Err(AccountError::Locked);
        }
        self.available += amount as i64;
        self.total += amount as i64;
        Ok(())
    }

    pub(crate) fn withdraw(&mut self, amount: u64) -> Result<(), AccountError> {
        if self.locked {
            return Err(AccountError::Locked);
        }
        if self.available < amount as i64 {
            return Err(AccountError::InsufficientFunds);
        }
        self.available -= amount as i64;
        self.total -= amount as i64;
        Ok(())
    }

    pub(crate) fn hold(&mut self, amount: u64) {
        self.available -= amount as i64;
        self.held += amount;
    }

    pub(crate) fn release(&mut self, amount: u64) {
        self.available += amount as i64;
        self.held -= amount;
    }

    pub(crate) fn chargeback(&mut self, amount: u64) {
        self.held -= amount;
        self.total -= amount as i64;
        self.locked = true;
    }
}

#[cfg(test)]
mod test {
    use super::{Account, AccountError};

    #[test]
    fn test_deposit_increases_available_and_total() {
        let mut account = Account::new(1);
        assert!(account.deposit(100).is_ok());
        assert_eq!(account.available(), 100);
        assert_eq!(account.total(), 100);
        assert_eq!(account.held(), 0);
    }

    #[test]
    fn test_deposit_on_locked_account_returns_error() {
        let mut account = Account::new(1);
        account.deposit(100).unwrap();
        account.hold(100);
        account.chargeback(100);
        assert!(matches!(account.deposit(50), Err(AccountError::Locked)));
        assert_eq!(account.available(), 0);
        assert_eq!(account.total(), 0);
    }

    #[test]
    fn test_withdraw_decreases_available_and_total() {
        let mut account = Account::new(1);
        account.deposit(100).unwrap();
        assert!(account.withdraw(40).is_ok());
        assert_eq!(account.available(), 60);
        assert_eq!(account.total(), 60);
    }

    #[test]
    fn test_withdraw_insufficient_funds_returns_error() {
        let mut account = Account::new(1);
        account.deposit(100).unwrap();
        assert!(matches!(account.withdraw(200), Err(AccountError::InsufficientFunds)));
        assert_eq!(account.available(), 100);
        assert_eq!(account.total(), 100);
    }

    #[test]
    fn test_withdraw_on_locked_account_returns_error() {
        let mut account = Account::new(1);
        account.deposit(100).unwrap();
        account.hold(100);
        account.chargeback(100);
        assert!(matches!(account.withdraw(50), Err(AccountError::Locked)));
    }

    #[test]
    fn test_hold_moves_available_to_held() {
        let mut account = Account::new(1);
        account.deposit(100).unwrap();
        account.hold(40);
        assert_eq!(account.available(), 60);
        assert_eq!(account.held(), 40);
        assert_eq!(account.total(), 100);
    }

    #[test]
    fn test_release_moves_held_to_available() {
        let mut account = Account::new(1);
        account.deposit(100).unwrap();
        account.hold(40);
        account.release(40);
        assert_eq!(account.available(), 100);
        assert_eq!(account.held(), 0);
        assert_eq!(account.total(), 100);
    }

    #[test]
    fn test_chargeback_removes_held_and_total_and_locks() {
        let mut account = Account::new(1);
        account.deposit(100).unwrap();
        account.hold(100);
        account.chargeback(100);
        assert_eq!(account.held(), 0);
        assert_eq!(account.total(), 0);
        assert!(account.locked());
    }

    #[test]
    fn test_chargeback_after_withdrawal_total_is_negative() {
        // Deposit 100, withdraw 80 (total=20), hold 100, chargeback 100.
        // total = 20 - 100 = -80: the account owes the bank the withdrawn funds.
        let mut account = Account::new(1);
        account.deposit(100).unwrap();
        account.withdraw(80).unwrap();
        account.hold(100);
        account.chargeback(100);
        assert_eq!(account.available(), -80);
        assert_eq!(account.held(), 0);
        assert_eq!(account.total(), -80);
        assert!(account.locked());
    }
}
