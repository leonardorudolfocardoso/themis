//! A financial transaction processor that reads CSV command streams and outputs account balances.
mod account;
mod amount;
mod balance;
mod command;
mod csv;
mod funds;
mod id;
mod processor;
mod transaction;

pub use account::Account;
pub use amount::Amount;
pub use command::Command;
pub use csv::from_reader;
pub use csv::to_writer;
pub use processor::Processor;
