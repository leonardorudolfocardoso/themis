use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub enum WithdrawalError {
    NotEnoughFunds,
    AccountLocked,
}

impl Error for WithdrawalError {}

impl Display for WithdrawalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WithdrawalError::NotEnoughFunds => f.write_str("not enough funds"),
            WithdrawalError::AccountLocked => f.write_str("account is locked"),
        }
    }
}

/// Monetary amounts are stored as integer units of 0.0001 (4 decimal places).
/// e.g. 1.2345 is represented as 12345.
pub struct Account {
    pub client: u16,
    pub available: i64,
    pub held: u64,
    pub total: u64,
    pub locked: bool,
}

impl Account {
    pub fn new(client: u16) -> Self {
        Self {
            client,
            available: 0,
            held: 0,
            total: 0,
            locked: false,
        }
    }

    pub fn deposit(&mut self, amount: u64) {
        self.available += amount as i64;
        self.total += amount;
    }

    pub fn withdraw(&mut self, amount: u64) -> Result<(), WithdrawalError> {
        if self.locked {
            return Err(WithdrawalError::AccountLocked);
        }
        if self.available < amount as i64 {
            return Err(WithdrawalError::NotEnoughFunds);
        }
        self.available -= amount as i64;
        self.total -= amount;

        Ok(())
    }

    pub fn dispute(&mut self, amount: u64) {
        self.available -= amount as i64;
        self.held += amount;
    }
}

#[cfg(test)]
mod test {
    use super::{Account, WithdrawalError};

    #[test]
    fn test_new() {
        let account = Account::new(42);
        assert_eq!(account.client, 42);
        assert_eq!(account.available, 0);
        assert_eq!(account.held, 0);
        assert_eq!(account.total, 0);
        assert!(!account.locked);
    }

    #[test]
    fn test_deposit() {
        let mut account = Account::new(0);
        account.deposit(100);
        assert_eq!(account.client, 0);
        assert_eq!(account.available, 100);
        assert_eq!(account.total, 100);
        assert_eq!(account.held, 0);
        assert!(!account.locked);
    }

    #[test]
    fn test_withdraw_success() {
        let mut account = Account::new(0);
        account.deposit(100);
        let result = account.withdraw(20);
        assert!(result.is_ok());
        assert_eq!(account.client, 0);
        assert_eq!(account.available, 80);
        assert_eq!(account.total, 80);
        assert_eq!(account.held, 0);
        assert!(!account.locked);
    }

    #[test]
    fn test_withdraw_insufficient_funds() {
        let mut account = Account::new(0);
        account.deposit(100);
        let result = account.withdraw(200);
        assert!(matches!(result, Err(WithdrawalError::NotEnoughFunds)));
        assert_eq!(account.available, 100);
        assert_eq!(account.total, 100);
    }

    #[test]
    fn test_withdraw_locked() {
        let mut account = Account::new(0);
        account.deposit(100);
        account.locked = true;
        let result = account.withdraw(50);
        assert!(matches!(result, Err(WithdrawalError::AccountLocked)));
        assert_eq!(account.available, 100);
        assert_eq!(account.total, 100);
    }

    #[test]
    fn test_dispute() {
        let mut account = Account::new(0);
        account.deposit(100);
        account.dispute(40);
        assert_eq!(account.available, 60);
        assert_eq!(account.held, 40);
        assert_eq!(account.total, 100);
    }

    #[test]
    fn test_dispute_makes_available_negative() {
        let mut account = Account::new(0);
        account.deposit(100);
        account.dispute(150);
        assert_eq!(account.available, -50);
        assert_eq!(account.held, 150);
        assert_eq!(account.total, 100);
    }
}
