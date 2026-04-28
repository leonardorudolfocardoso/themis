use serde::Serialize;

use crate::account::{Account, Accounts};

/// Serialization-ready snapshot of an account for CSV output.
///
/// Monetary values are formatted as decimal strings with 4 decimal places
/// rather than floats, ensuring exact representation.
#[derive(Serialize)]
struct OutputRow {
    client: u16,
    available: String,
    held: String,
    total: String,
    locked: bool,
}

impl From<&Account> for OutputRow {
    fn from(account: &Account) -> Self {
        Self {
            client: account.client(),
            available: account.available().to_string(),
            held: account.held().to_string(),
            total: account.total().to_string(),
            locked: account.locked(),
        }
    }
}

/// Writes account state to `writer` as CSV, sorted by client ID.
pub fn to_writer(writer: impl std::io::Write, accounts: Accounts) {
    let mut accounts: Vec<Account> = accounts.into_iter().collect();
    accounts.sort_by_key(|a| a.client());
    let mut wtr = csv::Writer::from_writer(writer);
    for account in &accounts {
        wtr.serialize(OutputRow::from(account)).unwrap();
    }
    wtr.flush().unwrap();
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::amount::Amount;
    use crate::command::Command;
    use crate::ledger::Ledger;

    fn accounts_from(commands: Vec<Command>) -> Accounts {
        let mut ledger = Ledger::new();
        ledger.ingest(commands.into_iter());
        ledger.into_accounts()
    }

    #[test]
    fn test_writes_header_and_row() {
        let accounts = accounts_from(vec![Command::Deposit {
            client: 1,
            tx: 1,
            amount: Amount::raw(10000),
        }]);
        let mut buf = Vec::new();
        to_writer(&mut buf, accounts);
        let output = String::from_utf8(buf).unwrap();
        assert_eq!(
            output,
            "client,available,held,total,locked\n1,1.0000,0.0000,1.0000,false\n"
        );
    }

    #[test]
    fn test_accounts_sorted_by_client() {
        let accounts = accounts_from(vec![
            Command::Deposit {
                client: 2,
                tx: 1,
                amount: Amount::raw(10000),
            },
            Command::Deposit {
                client: 1,
                tx: 2,
                amount: Amount::raw(20000),
            },
        ]);
        let mut buf = Vec::new();
        to_writer(&mut buf, accounts);
        let output = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = output.lines().collect();
        assert!(lines[1].starts_with("1,"));
        assert!(lines[2].starts_with("2,"));
    }
}
