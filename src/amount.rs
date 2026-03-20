use std::fmt;
use std::ops::{AddAssign, SubAssign};

/// A non-negative monetary amount, stored as an integer in units of 0.0001.
///
/// `Amount` is the type for all transaction values — deposits, withdrawals,
/// and held funds. It is always non-negative; negative balances are
/// represented by [`Funds`](crate::funds::Funds).
///
/// Internally, `1.2345` is stored as `12345`. This avoids floating-point
/// arithmetic in all domain logic.
///
/// # Construction
///
/// Amounts are constructed from a `f64` decimal at the input boundary:
///
/// ```
/// use themis::Amount;
///
/// let amount = Amount::try_from(1.5).unwrap();
/// assert_eq!(amount.to_string(), "1.5000");
/// ```
///
/// Negative values are rejected:
///
/// ```
/// use themis::Amount;
///
/// assert!(Amount::try_from(-1.0).is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Amount(u64);

impl Amount {
    #[cfg(test)]
    pub(crate) fn raw(value: u64) -> Self {
        Amount(value)
    }

    pub(crate) fn as_i64(self) -> i64 {
        self.0 as i64
    }
}

/// Parses a decimal value into an `Amount`, scaling by 10,000.
///
/// Returns `Err(())` if the value is negative.
///
/// ```
/// use themis::Amount;
///
/// assert_eq!(Amount::try_from(1.2345).unwrap().to_string(), "1.2345");
/// assert_eq!(Amount::try_from(0.0001).unwrap().to_string(), "0.0001");
/// assert!(Amount::try_from(-1.0).is_err());
/// ```
impl TryFrom<f64> for Amount {
    type Error = ();

    fn try_from(f: f64) -> Result<Self, Self::Error> {
        if f < 0.0 {
            return Err(());
        }
        Ok(Amount((f * 10000.0).round() as u64))
    }
}

impl AddAssign for Amount {
    fn add_assign(&mut self, rhs: Amount) {
        self.0 += rhs.0;
    }
}

impl SubAssign for Amount {
    fn sub_assign(&mut self, rhs: Amount) {
        self.0 -= rhs.0;
    }
}

/// Formats the amount as a decimal with exactly 4 decimal places.
///
/// ```
/// use themis::Amount;
///
/// assert_eq!(Amount::try_from(1.5).unwrap().to_string(),    "1.5000");
/// assert_eq!(Amount::try_from(1.2345).unwrap().to_string(), "1.2345");
/// assert_eq!(Amount::try_from(0.0).unwrap().to_string(),    "0.0000");
/// ```
impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{:04}", self.0 / 10000, self.0 % 10000)
    }
}

#[cfg(test)]
mod test {
    use super::Amount;

    #[test]
    fn test_from_f64_four_decimals() {
        assert_eq!(Amount::try_from(1.2345), Ok(Amount(12345)));
    }

    #[test]
    fn test_from_f64_fewer_decimals() {
        assert_eq!(Amount::try_from(1.5), Ok(Amount(15000)));
    }

    #[test]
    fn test_from_f64_no_decimal() {
        assert_eq!(Amount::try_from(100.0), Ok(Amount(1000000)));
    }

    #[test]
    fn test_from_f64_minimum() {
        assert_eq!(Amount::try_from(0.0001), Ok(Amount(1)));
    }

    #[test]
    fn test_from_f64_negative_rejected() {
        assert!(Amount::try_from(-1.0).is_err());
    }

    #[test]
    fn test_display() {
        assert_eq!(Amount(12345).to_string(), "1.2345");
    }

    #[test]
    fn test_display_pads_decimals() {
        assert_eq!(Amount(15000).to_string(), "1.5000");
    }

    #[test]
    fn test_display_zero() {
        assert_eq!(Amount(0).to_string(), "0.0000");
    }
}
