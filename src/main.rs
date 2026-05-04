use std::{env, error::Error, fs::File, io, path::Path};
use themis::{EventStore, FileStore, Ledger, from_reader, to_writer};

fn run(store: impl EventStore, input: impl io::Read, output: impl io::Write) -> io::Result<()> {
    let mut ledger = Ledger::replay(store);
    ledger.ingest(from_reader(input))?;
    to_writer(output, ledger.into_accounts());
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let path = env::args()
        .nth(1)
        .ok_or("usage: themis <transactions.csv>")?;
    run(
        FileStore::open(Path::new("store.jsonl"))?,
        File::open(path)?,
        io::stdout(),
    )?;
    Ok(())
}

#[cfg(test)]
mod test {
    use tempfile::NamedTempFile;
    use themis::FileStore;

    use super::run;

    #[test]
    fn test_integration() {
        let log = NamedTempFile::new().unwrap();
        let input = std::fs::read("tests/fixtures/transactions.csv").unwrap();
        let mut output = Vec::new();
        run(
            FileStore::open(log.path()).unwrap(),
            input.as_slice(),
            &mut output,
        )
        .unwrap();
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
    fn test_large_deposit_stays_positive() {
        let log = NamedTempFile::new().unwrap();
        let input = b"type,client,tx,amount\ndeposit,1,1,922337203685477.5808\n";
        let mut output = Vec::new();
        run(
            FileStore::open(log.path()).unwrap(),
            input.as_slice(),
            &mut output,
        )
        .unwrap();
        let csv = String::from_utf8(output).unwrap();
        assert!(csv.contains("1,922337203685477.5808,0.0000,922337203685477.5808,false"));
    }
}
