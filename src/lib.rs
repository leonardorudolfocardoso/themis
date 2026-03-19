use std::{collections::HashMap, fmt::Display};

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

#[derive(Debug)]
pub enum Error {
    NotEnoughFunds,
    AccountLocked,
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotEnoughFunds => f.write_str("not enough funds"),
            Error::AccountLocked => f.write_str("account is locked"),
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

pub struct Processor {
    transactions: Vec<Transaction>,
}

impl Processor {
    pub fn new() -> Self {
        Self {
            transactions: Vec::new(),
        }
    }

    pub fn ingest(&mut self, transaction: Transaction) {
        self.transactions.push(transaction);
    }

    pub fn accounts(&self) -> HashMap<u16, Account> {
        todo!()
    }
}

