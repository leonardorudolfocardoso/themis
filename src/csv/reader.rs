use serde::Deserialize;

use crate::event::Event;

#[derive(Deserialize)]
struct RawEvent {
    #[serde(rename = "type")]
    kind: String,
    client: u16,
    tx: u32,
    amount: Option<f64>,
}

impl TryFrom<RawEvent> for Event {
    type Error = ();

    fn try_from(row: RawEvent) -> Result<Self, Self::Error> {
        let client = row.client;
        let tx = row.tx;
        match row.kind.as_str() {
            "deposit" => Ok(Event::Deposit { client, tx, amount: to_amount(row.amount.ok_or(())?).ok_or(())? }),
            "withdrawal" => Ok(Event::Withdrawal { client, tx, amount: to_amount(row.amount.ok_or(())?).ok_or(())? }),
            "dispute" => Ok(Event::Dispute { client, tx }),
            "resolve" => Ok(Event::Resolve { client, tx }),
            "chargeback" => Ok(Event::Chargeback { client, tx }),
            _ => Err(()),
        }
    }
}

pub fn from_reader(reader: impl std::io::Read) -> impl Iterator<Item = Event> {
    csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(reader)
        .into_deserialize()
        .filter_map(|r: Result<RawEvent, _>| r.ok())
        .filter_map(|r| r.try_into().ok())
}

fn to_amount(f: f64) -> Option<u64> {
    if f < 0.0 {
        return None;
    }
    Some((f * 10000.0).round() as u64)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::event::Event;

    fn parse(input: &str) -> Vec<Event> {
        from_reader(input.as_bytes()).collect()
    }

    #[test]
    fn test_parse_deposit() {
        let events = parse("type,client,tx,amount\ndeposit,1,1,1.5");
        assert!(matches!(events[0], Event::Deposit { client: 1, tx: 1, amount: 15000 }));
    }

    #[test]
    fn test_parse_withdrawal() {
        let events = parse("type,client,tx,amount\nwithdrawal,2,3,0.0001");
        assert!(matches!(events[0], Event::Withdrawal { client: 2, tx: 3, amount: 1 }));
    }

    #[test]
    fn test_parse_dispute() {
        let events = parse("type,client,tx,amount\ndispute,1,1,");
        assert!(matches!(events[0], Event::Dispute { client: 1, tx: 1 }));
    }

    #[test]
    fn test_parse_resolve() {
        let events = parse("type,client,tx,amount\nresolve,1,1,");
        assert!(matches!(events[0], Event::Resolve { client: 1, tx: 1 }));
    }

    #[test]
    fn test_parse_chargeback() {
        let events = parse("type,client,tx,amount\nchargeback,1,1,");
        assert!(matches!(events[0], Event::Chargeback { client: 1, tx: 1 }));
    }

    #[test]
    fn test_invalid_rows_are_skipped() {
        let events = parse("type,client,tx,amount\nbadtype,1,1,1.0\ndeposit,1,2,1.0");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], Event::Deposit { tx: 2, .. }));
    }

    #[test]
    fn test_whitespace_is_trimmed() {
        let events = parse("type, client, tx, amount\n deposit , 1 , 1 , 1.5 ");
        assert!(matches!(events[0], Event::Deposit { client: 1, tx: 1, amount: 15000 }));
    }

    #[test]
    fn test_amount_four_decimals() {
        assert_eq!(to_amount(1.2345), Some(12345));
    }

    #[test]
    fn test_amount_fewer_decimals() {
        assert_eq!(to_amount(1.5), Some(15000));
    }

    #[test]
    fn test_amount_no_decimal() {
        assert_eq!(to_amount(100.0), Some(1000000));
    }

    #[test]
    fn test_amount_minimum() {
        assert_eq!(to_amount(0.0001), Some(1));
    }

    #[test]
    fn test_negative_amount_is_rejected() {
        assert_eq!(to_amount(-1.0), None);
    }
}
