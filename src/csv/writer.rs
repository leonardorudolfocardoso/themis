use serde::Serialize;

use crate::account::Account;

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

pub fn to_writer(writer: impl std::io::Write, accounts: impl Iterator<Item = Account>) {
    let mut accounts: Vec<Account> = accounts.collect();
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
    use crate::event::Event;
    use crate::processor::Processor;

    fn account_with(client: u16, available: i64, held: u64, locked: bool) -> crate::account::Account {
        let mut events: Vec<Event> = vec![Event::Deposit { client, tx: 1, amount: Amount::from((available + held as i64) as u64) }];
        if held > 0 {
            events.push(Event::Dispute { client, tx: 1 });
        }
        if locked {
            events.push(Event::Chargeback { client, tx: 1 });
        }
        Processor::new()
            .process(events.into_iter())
            .remove(&client)
            .unwrap()
    }

    #[test]
    fn test_writes_header_and_row() {
        let account = account_with(1, 10000, 0, false);
        let mut buf = Vec::new();
        to_writer(&mut buf, vec![account].into_iter());
        let output = String::from_utf8(buf).unwrap();
        assert_eq!(output, "client,available,held,total,locked\n1,1.0000,0.0000,1.0000,false\n");
    }

    #[test]
    fn test_accounts_sorted_by_client() {
        let a1 = account_with(2, 10000, 0, false);
        let a2 = account_with(1, 20000, 0, false);
        let mut buf = Vec::new();
        to_writer(&mut buf, vec![a1, a2].into_iter());
        let output = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = output.lines().collect();
        assert!(lines[1].starts_with("1,"));
        assert!(lines[2].starts_with("2,"));
    }
}
