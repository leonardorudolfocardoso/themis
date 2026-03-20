use crate::amount::Amount;
use crate::funds::Funds;

#[derive(Debug)]
pub(crate) enum BalanceError {
    InsufficientFunds,
}

#[derive(Default)]
pub(crate) struct Balance {
    available: Funds,
    held: Amount,
}

impl Balance {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub fn available(&self) -> Funds {
        self.available
    }

    pub fn held(&self) -> Amount {
        self.held
    }

    pub fn total(&self) -> Funds {
        self.available + self.held
    }

    pub(crate) fn deposit(&mut self, amount: Amount) {
        self.available += amount;
    }

    pub(crate) fn withdraw(&mut self, amount: Amount) -> Result<(), BalanceError> {
        if self.available < amount {
            return Err(BalanceError::InsufficientFunds);
        }
        self.available -= amount;
        Ok(())
    }

    pub(crate) fn hold(&mut self, amount: Amount) {
        self.available -= amount;
        self.held += amount;
    }

    pub(crate) fn release(&mut self, amount: Amount) {
        self.available += amount;
        self.held -= amount;
    }

    pub(crate) fn chargeback(&mut self, amount: Amount) {
        self.held -= amount;
    }
}

#[cfg(test)]
mod test {
    use super::Balance;
    use crate::amount::Amount;

    #[test]
    fn test_deposit_increases_available() {
        let mut b = Balance::new();
        b.deposit(Amount::raw(100));
        assert_eq!(b.available(), 100);
        assert_eq!(b.held(), Amount::default());
        assert_eq!(b.total(), 100);
    }

    #[test]
    fn test_withdraw_decreases_available() {
        let mut b = Balance::new();
        b.deposit(Amount::raw(100));
        assert!(b.withdraw(Amount::raw(40)).is_ok());
        assert_eq!(b.available(), 60);
        assert_eq!(b.total(), 60);
    }

    #[test]
    fn test_withdraw_insufficient_funds_returns_error() {
        let mut b = Balance::new();
        b.deposit(Amount::raw(100));
        assert!(b.withdraw(Amount::raw(200)).is_err());
        assert_eq!(b.available(), 100);
    }

    #[test]
    fn test_hold_moves_available_to_held() {
        let mut b = Balance::new();
        b.deposit(Amount::raw(100));
        b.hold(Amount::raw(40));
        assert_eq!(b.available(), 60);
        assert_eq!(b.held(), Amount::raw(40));
        assert_eq!(b.total(), 100);
    }

    #[test]
    fn test_release_moves_held_to_available() {
        let mut b = Balance::new();
        b.deposit(Amount::raw(100));
        b.hold(Amount::raw(40));
        b.release(Amount::raw(40));
        assert_eq!(b.available(), 100);
        assert_eq!(b.held(), Amount::default());
        assert_eq!(b.total(), 100);
    }

    #[test]
    fn test_chargeback_removes_held_and_decreases_total() {
        let mut b = Balance::new();
        b.deposit(Amount::raw(100));
        b.hold(Amount::raw(100));
        b.chargeback(Amount::raw(100));
        assert_eq!(b.held(), Amount::default());
        assert_eq!(b.total(), 0);
    }

    #[test]
    fn test_chargeback_after_withdrawal_total_is_negative() {
        let mut b = Balance::new();
        b.deposit(Amount::raw(100));
        b.withdraw(Amount::raw(80)).unwrap();
        b.hold(Amount::raw(100));
        b.chargeback(Amount::raw(100));
        assert_eq!(b.available(), -80);
        assert_eq!(b.held(), Amount::default());
        assert_eq!(b.total(), -80);
    }
}
