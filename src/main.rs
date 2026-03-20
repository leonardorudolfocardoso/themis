use std::{env, error::Error, fs::File, io};
use themis::{Processor, from_reader, to_writer};

fn run(input: impl io::Read, output: impl io::Write) {
    let accounts = Processor::new().process(from_reader(input));
    to_writer(output, accounts.into_values());
}

fn main() -> Result<(), Box<dyn Error>> {
    let path = env::args().nth(1).ok_or("usage: themis <transactions.csv>")?;
    run(File::open(path)?, io::stdout());
    Ok(())
}
