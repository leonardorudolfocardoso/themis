use std::fmt;
use std::ops::{AddAssign, SubAssign};
use std::str::FromStr;

/// A non-negative monetary amount, stored as an integer in units of 0.0001.
///
/// `Amount` is the type for all transaction values — deposits, withdrawals,
/// and held funds. It is always non-negative; negative balances are
/// represented by `Funds`.
///
/// Internally, `1.2345` is stored as `12345`. This avoids floating-point
/// arithmetic in all domain logic.
///
/// # Construction
///
/// Amounts are constructed by parsing a decimal string:
///
/// ```
/// use themis::Amount;
///
/// let amount = "1.5".parse::<Amount>().unwrap();
/// assert_eq!(amount.to_string(), "1.5000");
/// ```
///
/// Negative values and values with more than 4 decimal places are rejected:
///
/// ```
/// use themis::Amount;
///
/// assert!("-1.0".parse::<Amount>().is_err());
/// assert!("1.23456".parse::<Amount>().is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Amount(u64);

impl Amount {
    #[cfg(test)]
    pub(crate) fn raw(value: u64) -> Self {
        Amount(value)
    }

    pub(crate) fn as_i128(self) -> i128 {
        self.0 as i128
    }
}

/// Parses a decimal string into an `Amount`, scaling by 10,000.
///
/// Returns `Err(())` if the value is negative, malformed, too precise, or too large.
///
/// ```
/// use themis::Amount;
///
/// assert_eq!("1.2345".parse::<Amount>().unwrap().to_string(), "1.2345");
/// assert_eq!("0.0001".parse::<Amount>().unwrap().to_string(), "0.0001");
/// assert!("1e3".parse::<Amount>().is_err());
/// ```
impl FromStr for Amount {
    type Err = ();

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let input = input.trim();
        if input.is_empty() || input.starts_with('-') {
            return Err(());
        }

        let (whole, fraction) = match input.split_once('.') {
            Some((whole, fraction)) => (whole, fraction),
            None => (input, ""),
        };

        if whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) {
            return Err(());
        }

        if fraction.len() > 4 || !fraction.bytes().all(|b| b.is_ascii_digit()) {
            return Err(());
        }

        let whole_units = whole
            .parse::<u64>()
            .map_err(|_| ())?
            .checked_mul(10_000)
            .ok_or(())?;

        let mut fraction_units = 0_u64;
        let mut scale = 1_000_u64;
        for digit in fraction.bytes() {
            fraction_units += u64::from(digit - b'0') * scale;
            scale /= 10;
        }

        Ok(Amount(whole_units.checked_add(fraction_units).ok_or(())?))
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
/// assert_eq!("1.5".parse::<Amount>().unwrap().to_string(),    "1.5000");
/// assert_eq!("1.2345".parse::<Amount>().unwrap().to_string(), "1.2345");
/// assert_eq!("0.0".parse::<Amount>().unwrap().to_string(),    "0.0000");
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
    fn test_parse_four_decimals() {
        assert_eq!("1.2345".parse::<Amount>(), Ok(Amount(12345)));
    }

    #[test]
    fn test_parse_fewer_decimals() {
        assert_eq!("1.5".parse::<Amount>(), Ok(Amount(15000)));
    }

    #[test]
    fn test_parse_no_decimal() {
        assert_eq!("100".parse::<Amount>(), Ok(Amount(1000000)));
    }

    #[test]
    fn test_parse_minimum() {
        assert_eq!("0.0001".parse::<Amount>(), Ok(Amount(1)));
    }

    #[test]
    fn test_parse_negative_rejected() {
        assert!("-1.0".parse::<Amount>().is_err());
    }

    #[test]
    fn test_parse_too_many_decimals_rejected() {
        assert!("1.23456".parse::<Amount>().is_err());
    }

    #[test]
    fn test_parse_malformed_rejected() {
        assert!("NaN".parse::<Amount>().is_err());
        assert!("inf".parse::<Amount>().is_err());
        assert!("1e3".parse::<Amount>().is_err());
        assert!("1.2.3".parse::<Amount>().is_err());
    }

    #[test]
    fn test_parse_overflow_rejected() {
        assert!("1844674407370955.1616".parse::<Amount>().is_err());
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
