//! A financial transaction processor that reads CSV event streams and outputs account balances.
mod account;
mod amount;
mod balance;
mod csv;
mod command;
mod funds;
mod id;
mod processor;
mod transaction;

pub use account::Account;
pub use amount::Amount;
pub use csv::from_reader;
pub use csv::to_writer;
pub use command::Command;
pub use processor::ApplyResult;
pub use processor::Processor;
