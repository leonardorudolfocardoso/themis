use std::collections::HashMap;

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

pub enum Transaction {
    Deposit { client: u16, tx: u32, amount: u64 },
    Withdrawal { client: u16, tx: u32, amount: u64 },
    Dispute { client: u16, tx: u32 },
    Resolve { client: u16, tx: u32 },
    Chargeback { client: u16, tx: u32 },
}

enum TransactionState {
    Valid,
    Disputed,
    Resolved,
    Chargedback,
}

struct TransactionRecord {
    client: u16,
    amount: u64,
    state: TransactionState,
}

pub struct Processor {
    records: HashMap<u32, TransactionRecord>,
}

impl Processor {
    pub fn new() -> Self {
        Self {
            records: HashMap::new(),
        }
    }

    pub fn process(&self) -> HashMap<u16, Account> {
        todo!()
    }
}
