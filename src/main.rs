use std::{
    env,
    error::Error,
    io::{self, stdin, stdout},
    path::Path,
};
use themis::{FileStore, Ledger, csv};

fn ingest(log_path: &Path, input: impl io::Read, output: impl io::Write) -> io::Result<()> {
    let log = FileStore::open(log_path)?;
    let mut ledger = Ledger::replay(log)?;
    ledger.ingest(csv::from_reader(input))?;
    csv::to_writer(output, ledger.into_accounts());
    Ok(())
}

fn replay(log_path: &Path, output: impl io::Write) -> io::Result<()> {
    let log = FileStore::open(log_path)?;
    let ledger = Ledger::replay(log)?;
    csv::to_writer(output, ledger.into_accounts());
    Ok(())
}

const USAGE: &str = "usage: themis replay/ingest <log.jsonl>";

fn main() -> Result<(), Box<dyn Error>> {
    let command = env::args().nth(1).ok_or(USAGE)?;
    let log_arg = env::args().nth(2).ok_or(USAGE)?;
    let log_path = Path::new(&log_arg);

    match command.as_str() {
        "replay" => replay(log_path, stdout())?,
        "ingest" => ingest(log_path, stdin(), stdout())?,
        _ => eprintln!("unknown command {}\n{}", command, USAGE),
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use std::path::Path;
    use tempfile::NamedTempFile;

    use super::ingest;
    use super::replay;

    #[test]
    fn test_ingest_produces_expected_accounts() {
        let log = NamedTempFile::new().unwrap();
        let input = std::fs::read("tests/fixtures/transactions.csv").unwrap();
        let mut output = Vec::new();
        ingest(log.path(), input.as_slice(), &mut output).unwrap();

        let csv = String::from_utf8(output).unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "client,available,held,total,locked");

        let row = |client: &str| {
            lines
                .iter()
                .find(|l| l.starts_with(&format!("{},", client)))
                .copied()
                .unwrap()
        };

        assert_eq!(row("1"), "1,120.0000,0.0000,120.0000,false");
        assert_eq!(row("2"), "2,-80.0000,0.0000,-80.0000,true");
        assert_eq!(row("3"), "3,100.0000,0.0000,100.0000,false");
        assert_eq!(row("4"), "4,10.0000,0.0000,10.0000,true");
        assert_eq!(row("5"), "5,1300.0000,0.0000,1300.0000,false");
        assert_eq!(row("6"), "6,0.0000,0.0000,0.0000,false");
        assert_eq!(row("7"), "7,25.5000,0.0000,25.5000,true");
        assert_eq!(row("8"), "8,50.0000,0.0000,50.0000,false");
        assert_eq!(row("9"), "9,-1000.0000,0.0000,-1000.0000,true");
        assert_eq!(row("10"), "10,0.0000,0.0000,0.0000,true");
    }

    #[test]
    fn test_replay_from_fixture_log() {
        let mut output = Vec::new();
        replay(Path::new("tests/fixtures/store.jsonl"), &mut output).unwrap();
        let csv = String::from_utf8(output).unwrap();

        assert!(csv.contains("1,120.0000,0.0000,120.0000,false"));
        assert!(csv.contains("2,0.0000,0.0000,0.0000,true"));
    }

    #[test]
    fn test_replay_reproduces_ingest_output() {
        let log = NamedTempFile::new().unwrap();
        let input = std::fs::read("tests/fixtures/transactions.csv").unwrap();

        let mut ingest_out = Vec::new();
        ingest(log.path(), input.as_slice(), &mut ingest_out).unwrap();

        let mut replay_out = Vec::new();
        replay(log.path(), &mut replay_out).unwrap();

        assert_eq!(ingest_out, replay_out);
    }

    #[test]
    fn test_ingest_supports_large_deposit() {
        let log = NamedTempFile::new().unwrap();
        let input = b"type,client,tx,amount\ndeposit,1,1,922337203685477.5808\n";
        let mut output = Vec::new();
        ingest(log.path(), input.as_slice(), &mut output).unwrap();
        let csv = String::from_utf8(output).unwrap();
        assert!(csv.contains("1,922337203685477.5808,0.0000,922337203685477.5808,false"));
    }
}
