use crate::amount::Amount;
use crate::funds::Funds;

/// Error type for balance operations.
#[derive(Debug)]
pub(crate) enum BalanceError {
    /// The requested withdrawal exceeds the available funds.
    InsufficientFunds,
}

/// Holds the monetary state of an account.
///
/// `Balance` tracks available funds and held funds separately. Held funds
/// cannot be withdrawn until they are released or removed.
///
/// `total` is always derived as `available + held` and is never stored
/// independently, preventing inconsistency.
///
/// Note: `available` can go negative after held funds are removed following
/// a prior withdrawal.
#[derive(Default)]
pub(crate) struct Balance {
    /// Funds available for withdrawal. Can go negative.
    available: Funds,
    /// Funds currently held and unavailable for withdrawal.
    held: Amount,
}

impl Balance {
    /// Creates a new balance with zero available and zero held.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Returns the funds available for withdrawal.
    pub fn available(&self) -> Funds {
        self.available
    }

    /// Returns the funds currently held.
    pub fn held(&self) -> Amount {
        self.held
    }

    /// Returns the total balance (`available + held`).
    pub fn total(&self) -> Funds {
        self.available + self.held
    }

    /// Increases available funds by `amount`.
    pub(crate) fn deposit(&mut self, amount: Amount) {
        self.available += amount;
    }

    /// Decreases available funds by `amount`.
    ///
    /// Returns [`BalanceError::InsufficientFunds`] if `amount` exceeds
    /// available funds, leaving the balance unchanged.
    pub(crate) fn withdraw(&mut self, amount: Amount) -> Result<(), BalanceError> {
        if self.available < amount {
            return Err(BalanceError::InsufficientFunds);
        }
        self.available -= amount;
        Ok(())
    }

    /// Moves `amount` from available to held.
    pub(crate) fn hold(&mut self, amount: Amount) {
        self.available -= amount;
        self.held += amount;
    }

    /// Moves `amount` from held back to available.
    pub(crate) fn release(&mut self, amount: Amount) {
        self.available += amount;
        self.held -= amount;
    }

    /// Removes `amount` from held without returning it to available.
    ///
    /// This can leave `available` negative if funds were withdrawn before
    /// the hold was placed.
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
