use crate::amount::Amount;

#[derive(Debug)]
pub(crate) enum BalanceError {
    InsufficientFunds,
}

pub(crate) struct Balance {
    available: i64,
    held: u64,
}

impl Balance {
    pub(crate) fn new() -> Self {
        Self { available: 0, held: 0 }
    }

    pub fn available(&self) -> i64 {
        self.available
    }

    pub fn held(&self) -> u64 {
        self.held
    }

    pub fn total(&self) -> i64 {
        self.available + self.held as i64
    }

    pub(crate) fn deposit(&mut self, amount: Amount) {
        self.available += amount.as_i64();
    }

    pub(crate) fn withdraw(&mut self, amount: Amount) -> Result<(), BalanceError> {
        if self.available < amount.as_i64() {
            return Err(BalanceError::InsufficientFunds);
        }
        self.available -= amount.as_i64();
        Ok(())
    }

    pub(crate) fn hold(&mut self, amount: Amount) {
        self.available -= amount.as_i64();
        self.held += amount.as_u64();
    }

    pub(crate) fn release(&mut self, amount: Amount) {
        self.available += amount.as_i64();
        self.held -= amount.as_u64();
    }

    pub(crate) fn chargeback(&mut self, amount: Amount) {
        self.held -= amount.as_u64();
    }
}

#[cfg(test)]
mod test {
    use super::Balance;
    use crate::amount::Amount;

    #[test]
    fn test_deposit_increases_available() {
        let mut b = Balance::new();
        b.deposit(Amount::from(100));
        assert_eq!(b.available(), 100);
        assert_eq!(b.held(), 0);
        assert_eq!(b.total(), 100);
    }

    #[test]
    fn test_withdraw_decreases_available() {
        let mut b = Balance::new();
        b.deposit(Amount::from(100));
        assert!(b.withdraw(Amount::from(40)).is_ok());
        assert_eq!(b.available(), 60);
        assert_eq!(b.total(), 60);
    }

    #[test]
    fn test_withdraw_insufficient_funds_returns_error() {
        let mut b = Balance::new();
        b.deposit(Amount::from(100));
        assert!(b.withdraw(Amount::from(200)).is_err());
        assert_eq!(b.available(), 100);
    }

    #[test]
    fn test_hold_moves_available_to_held() {
        let mut b = Balance::new();
        b.deposit(Amount::from(100));
        b.hold(Amount::from(40));
        assert_eq!(b.available(), 60);
        assert_eq!(b.held(), 40);
        assert_eq!(b.total(), 100);
    }

    #[test]
    fn test_release_moves_held_to_available() {
        let mut b = Balance::new();
        b.deposit(Amount::from(100));
        b.hold(Amount::from(40));
        b.release(Amount::from(40));
        assert_eq!(b.available(), 100);
        assert_eq!(b.held(), 0);
        assert_eq!(b.total(), 100);
    }

    #[test]
    fn test_chargeback_removes_held_and_decreases_total() {
        let mut b = Balance::new();
        b.deposit(Amount::from(100));
        b.hold(Amount::from(100));
        b.chargeback(Amount::from(100));
        assert_eq!(b.held(), 0);
        assert_eq!(b.total(), 0);
    }

    #[test]
    fn test_chargeback_after_withdrawal_total_is_negative() {
        let mut b = Balance::new();
        b.deposit(Amount::from(100));
        b.withdraw(Amount::from(80)).unwrap();
        b.hold(Amount::from(100));
        b.chargeback(Amount::from(100));
        assert_eq!(b.available(), -80);
        assert_eq!(b.held(), 0);
        assert_eq!(b.total(), -80);
    }
}
