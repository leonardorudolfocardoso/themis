//! A financial transaction processor that reads CSV event streams and outputs account balances.
mod account;
mod amount;
mod balance;
mod command;
mod csv;
mod decider;
mod event;
mod event_log;
mod funds;
mod id;
mod processor;
mod projection;
mod transaction;

pub use account::Account;
pub use amount::Amount;
pub use command::Command;
pub use csv::from_reader;
pub use csv::to_writer;
pub use event::Event;
pub use processor::ApplyResult;
pub use processor::Processor;
