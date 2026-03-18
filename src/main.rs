
fn main() {}

/// Monetary amounts are stored as integer units of 0.0001 (4 decimal places).
/// e.g. 1.2345 is represented as 12345.
struct Account {
    client: u16,
    available: i64,
    held: u64,
    total: u64,
    locked: bool,
}

impl Account {
    fn new(client: u16) -> Self {
        Self {
            client,
            available: 0,
            held: 0,
            total: 0,
            locked: false,
        }
    }

    fn deposit(&mut self, amount: u64) {
        self.available += amount as i64;
        self.total += amount;
    }

}

#[cfg(test)]
mod test {
    use crate::Account;

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
}
