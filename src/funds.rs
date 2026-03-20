use std::fmt;
use std::ops::{Add, AddAssign, Sub, SubAssign};

use crate::amount::Amount;

/// Signed monetary balance used for account state.
///
/// Unlike [`Amount`](crate::amount::Amount), `Funds` can go negative —
/// for example when a deposit is charged back after the funds have already
/// been withdrawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Funds(i128);

impl Add<Amount> for Funds {
    type Output = Funds;
    fn add(self, rhs: Amount) -> Funds {
        Funds(self.0 + rhs.as_i128())
    }
}

impl Sub<Amount> for Funds {
    type Output = Funds;
    fn sub(self, rhs: Amount) -> Funds {
        Funds(self.0 - rhs.as_i128())
    }
}

impl AddAssign<Amount> for Funds {
    fn add_assign(&mut self, rhs: Amount) {
        self.0 += rhs.as_i128();
    }
}

impl SubAssign<Amount> for Funds {
    fn sub_assign(&mut self, rhs: Amount) {
        self.0 -= rhs.as_i128();
    }
}

impl PartialEq<Amount> for Funds {
    fn eq(&self, other: &Amount) -> bool {
        self.0 == other.as_i128()
    }
}

impl PartialOrd<Amount> for Funds {
    fn partial_cmp(&self, other: &Amount) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.as_i128())
    }
}

impl From<i64> for Funds {
    fn from(value: i64) -> Self {
        Funds(value as i128)
    }
}

impl From<i128> for Funds {
    fn from(value: i128) -> Self {
        Funds(value)
    }
}

impl PartialEq<i64> for Funds {
    fn eq(&self, other: &i64) -> bool {
        self.0 == *other as i128
    }
}

impl fmt::Display for Funds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let abs = self.0.unsigned_abs();
        write!(f, "{}{}.{:04}", sign, abs / 10000, abs % 10000)
    }
}

#[cfg(test)]
mod test {
    use super::Funds;

    #[test]
    fn test_display_positive() {
        assert_eq!(Funds::from(15000_i64).to_string(), "1.5000");
    }

    #[test]
    fn test_display_negative() {
        assert_eq!(Funds::from(-12345_i64).to_string(), "-1.2345");
    }

    #[test]
    fn test_display_zero() {
        assert_eq!(Funds::from(0_i64).to_string(), "0.0000");
    }
}
