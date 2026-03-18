use std::fmt::Display;

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
}

#[derive(Debug)]
pub enum Error {
    NotEnoughFunds,
    AccountLocked,
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotEnoughFunds => f.write_str("not enough funds"),
            Error::AccountLocked => f.write_str("account is locked"),
        }
    }
}

pub struct Processor;

impl Processor {
    pub fn deposit(account: &mut Account, amount: u64) {
        account.available += amount as i64;
        account.total += amount;
    }

    pub fn withdraw(account: &mut Account, amount: u64) -> Result<(), Error> {
        if account.locked {
            return Err(Error::AccountLocked);
        }
        if account.available < amount as i64 {
            return Err(Error::NotEnoughFunds);
        }
        account.available -= amount as i64;
        account.total -= amount;

        Ok(())
    }

    pub fn dispute(account: &mut Account, amount: u64) {
        account.available -= amount as i64;
        account.held += amount;
    }
}

#[cfg(test)]
mod test {
    use super::{Account, Error, Processor};

    #[test]
    fn test_new_account() {
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
        Processor::deposit(&mut account, 100);
        assert_eq!(account.available, 100);
        assert_eq!(account.total, 100);
        assert_eq!(account.held, 0);
    }

    #[test]
    fn test_withdraw_success() {
        let mut account = Account::new(0);
        Processor::deposit(&mut account, 100);
        let result = Processor::withdraw(&mut account, 20);
        assert!(result.is_ok());
        assert_eq!(account.available, 80);
        assert_eq!(account.total, 80);
        assert_eq!(account.held, 0);
    }

    #[test]
    fn test_withdraw_insufficient_funds() {
        let mut account = Account::new(0);
        Processor::deposit(&mut account, 100);
        let result = Processor::withdraw(&mut account, 200);
        assert!(matches!(result, Err(Error::NotEnoughFunds)));
        assert_eq!(account.available, 100);
        assert_eq!(account.total, 100);
    }

    #[test]
    fn test_withdraw_locked() {
        let mut account = Account::new(0);
        Processor::deposit(&mut account, 100);
        account.locked = true;
        let result = Processor::withdraw(&mut account, 50);
        assert!(matches!(result, Err(Error::AccountLocked)));
        assert_eq!(account.available, 100);
        assert_eq!(account.total, 100);
    }

    #[test]
    fn test_dispute() {
        let mut account = Account::new(0);
        Processor::deposit(&mut account, 100);
        Processor::dispute(&mut account, 40);
        assert_eq!(account.available, 60);
        assert_eq!(account.held, 40);
        assert_eq!(account.total, 100);
    }

    #[test]
    fn test_dispute_makes_available_negative() {
        let mut account = Account::new(0);
        Processor::deposit(&mut account, 100);
        Processor::dispute(&mut account, 150);
        assert_eq!(account.available, -50);
        assert_eq!(account.held, 150);
        assert_eq!(account.total, 100);
    }
}
