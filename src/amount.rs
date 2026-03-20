use std::fmt;
use std::ops::{AddAssign, SubAssign};

/// Monetary amounts are stored as integer units of 0.0001 (4 decimal places).
/// e.g. 1.2345 is represented as 12345.
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
