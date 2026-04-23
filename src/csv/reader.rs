use serde::Deserialize;

use crate::command::Command;

/// Raw CSV row as deserialized by serde, before validation.
///
/// The `type` column is renamed to `kind` to avoid shadowing Rust's keyword.
/// `amount` is optional because dispute, resolve, and chargeback rows omit it.
/// It stays as text until validation so decimal parsing is exact.
#[derive(Deserialize)]
struct RawEvent {
    #[serde(rename = "type")]
    kind: String,
    client: u16,
    tx: u32,
    amount: Option<String>,
}

impl TryFrom<RawEvent> for Command {
    type Error = ();

    fn try_from(row: RawEvent) -> Result<Self, Self::Error> {
        let client = row.client;
        let tx = row.tx;
        match row.kind.as_str() {
            "deposit" => Ok(Command::Deposit {
                client,
                tx,
                amount: row.amount.ok_or(())?.parse()?,
            }),
            "withdrawal" => Ok(Command::Withdrawal {
                client,
                tx,
                amount: row.amount.ok_or(())?.parse()?,
            }),
            "dispute" => Ok(Command::Dispute { client, tx }),
            "resolve" => Ok(Command::Resolve { client, tx }),
            "chargeback" => Ok(Command::Chargeback { client, tx }),
            _ => Err(()),
        }
    }
}

/// Parses a CSV stream into an iterator of [`Command`]s.
///
/// Rows that cannot be deserialized or converted to a known event type are
/// silently skipped. Leading and trailing whitespace is trimmed from all fields.
pub fn from_reader(reader: impl std::io::Read) -> impl Iterator<Item = Command> {
    csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(reader)
        .into_deserialize()
        .filter_map(|r: Result<RawEvent, _>| r.ok())
        .filter_map(|r| r.try_into().ok())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::amount::Amount;
    use crate::command::Command;

    fn parse(input: &str) -> Vec<Command> {
        from_reader(input.as_bytes()).collect()
    }

    #[test]
    fn test_parse_deposit() {
        let events = parse("type,client,tx,amount\ndeposit,1,1,1.5");
        assert!(matches!(
            events[0],
            Command::Deposit {
                client: 1,
                tx: 1,
                ..
            }
        ));
        assert_eq!(
            if let Command::Deposit { amount, .. } = events[0] {
                amount
            } else {
                unreachable!()
            },
            Amount::raw(15000)
        );
    }

    #[test]
    fn test_parse_withdrawal() {
        let events = parse("type,client,tx,amount\nwithdrawal,2,3,0.0001");
        assert!(matches!(
            events[0],
            Command::Withdrawal {
                client: 2,
                tx: 3,
                ..
            }
        ));
        assert_eq!(
            if let Command::Withdrawal { amount, .. } = events[0] {
                amount
            } else {
                unreachable!()
            },
            Amount::raw(1)
        );
    }

    #[test]
    fn test_parse_dispute() {
        let events = parse("type,client,tx,amount\ndispute,1,1,");
        assert!(matches!(events[0], Command::Dispute { client: 1, tx: 1 }));
    }

    #[test]
    fn test_parse_resolve() {
        let events = parse("type,client,tx,amount\nresolve,1,1,");
        assert!(matches!(events[0], Command::Resolve { client: 1, tx: 1 }));
    }

    #[test]
    fn test_parse_chargeback() {
        let events = parse("type,client,tx,amount\nchargeback,1,1,");
        assert!(matches!(
            events[0],
            Command::Chargeback { client: 1, tx: 1 }
        ));
    }

    #[test]
    fn test_invalid_rows_are_skipped() {
        let events = parse("type,client,tx,amount\nbadtype,1,1,1.0\ndeposit,1,2,1.0");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], Command::Deposit { tx: 2, .. }));
    }

    #[test]
    fn test_invalid_amount_rows_are_skipped() {
        let events = parse("type,client,tx,amount\ndeposit,1,1,1.23456\ndeposit,1,2,1.2345");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], Command::Deposit { tx: 2, .. }));
    }

    #[test]
    fn test_whitespace_is_trimmed() {
        let events = parse("type, client, tx, amount\n deposit , 1 , 1 , 1.5 ");
        assert!(matches!(
            events[0],
            Command::Deposit {
                client: 1,
                tx: 1,
                ..
            }
        ));
        assert_eq!(
            if let Command::Deposit { amount, .. } = events[0] {
                amount
            } else {
                unreachable!()
            },
            Amount::raw(15000)
        );
    }

    #[test]
    fn test_large_amount_does_not_flip_sign() {
        let events = parse("type,client,tx,amount\ndeposit,1,1,922337203685477.5808");
        let amount = if let Command::Deposit { amount, .. } = events[0] {
            amount
        } else {
            unreachable!()
        };
        assert_eq!(amount.to_string(), "922337203685477.5808");
    }
}
