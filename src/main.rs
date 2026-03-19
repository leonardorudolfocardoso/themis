use std::io;
use themis::{Processor, from_reader, to_writer};

fn main() {
    let accounts = Processor::new().process(from_reader(io::stdin()));
    to_writer(io::stdout(), accounts.into_values());
}
