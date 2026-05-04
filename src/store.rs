use std::{io, time::SystemTime};

use crate::{Event, event::Recorded};

pub trait EventStore {
    fn append(&mut self, event: Event) -> io::Result<Recorded>;
    fn read_all(&self) -> impl Iterator<Item = io::Result<&Recorded>>;
}

pub struct MemoryStore(Vec<Recorded>);

impl MemoryStore {
    pub fn new() -> MemoryStore {
        MemoryStore(vec![])
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

impl EventStore for MemoryStore {
    fn append(&mut self, event: Event) -> io::Result<Recorded> {
        let recorded = Recorded {
            event,
            position: self.0.len() as u64,
            recorded_at: SystemTime::now(),
        };

        self.0.push(recorded);

        Ok(recorded)
    }
    fn read_all(&self) -> impl Iterator<Item = io::Result<&Recorded>> {
        self.0.iter().map(Ok)
    }
}
