/// Monetary amounts are stored as integer units of 0.0001 (4 decimal places).
/// e.g. 1.2345 is represented as 12345.
pub struct Account {
    client: u16,
    available: i64,
    held: u64,
    total: u64,
    locked: bool,
}

#[derive(Debug)]
pub(crate) enum AccountError {
    Locked,
    InsufficientFunds,
}

impl Account {
    pub(crate) fn new(client: u16) -> Self {
        Self {
            client,
            available: 0,
            held: 0,
            total: 0,
            locked: false,
        }
    }

    pub fn client(&self) -> u16 {
        self.client
    }

    pub fn available(&self) -> i64 {
        self.available
    }

    pub fn held(&self) -> u64 {
        self.held
    }

    pub fn total(&self) -> u64 {
        self.total
    }

    pub fn locked(&self) -> bool {
        self.locked
    }

    pub(crate) fn deposit(&mut self, amount: u64) -> Result<(), AccountError> {
        if self.locked {
            return Err(AccountError::Locked);
        }
        self.available += amount as i64;
        self.total += amount;
        Ok(())
    }

    pub(crate) fn withdraw(&mut self, amount: u64) -> Result<(), AccountError> {
        if self.locked {
            return Err(AccountError::Locked);
        }
        if self.available < amount as i64 {
            return Err(AccountError::InsufficientFunds);
        }
        self.available -= amount as i64;
        self.total -= amount;
        Ok(())
    }

    pub(crate) fn hold(&mut self, amount: u64) {
        self.available -= amount as i64;
        self.held += amount;
    }

    pub(crate) fn release(&mut self, amount: u64) {
        self.available += amount as i64;
        self.held -= amount;
    }

    pub(crate) fn chargeback(&mut self, amount: u64) {
        self.held -= amount;
        self.total -= amount;
        self.locked = true;
    }
}
