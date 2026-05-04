use std::{
    fs::{File, OpenOptions},
    io::{self, BufRead, BufReader, Seek, Write},
    path::Path,
    time::SystemTime,
};

use crate::{Event, event::Recorded};

pub trait EventStore {
    fn append(&mut self, event: Event) -> io::Result<Recorded>;
    fn read_all(&mut self) -> impl Iterator<Item = io::Result<Recorded>>;
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
    fn read_all(&mut self) -> impl Iterator<Item = io::Result<Recorded>> {
        self.0.clone().into_iter().map(Ok)
    }
}

pub struct FileStore {
    reader: File,
    writer: File,
    next_position: u64,
    sync_on_append: bool,
}

impl FileStore {
    fn count_existing(path: &Path) -> io::Result<u64> {
        match File::open(path) {
            Ok(file) => {
                let buf = BufReader::new(file);

                Ok(buf.lines().count() as u64)
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(0),
            Err(e) => Err(e),
        }
    }

    pub fn open(path: &Path) -> io::Result<FileStore> {
        let writer = OpenOptions::new().create(true).append(true).open(path)?;
        let reader = File::open(path)?;
        let next_position = FileStore::count_existing(path)?;

        Ok(FileStore {
            reader,
            writer,
            next_position,
            sync_on_append: true,
        })
    }

    fn write_line(&mut self, recorded: &Recorded) -> io::Result<()> {
        let mut buf = serde_json::to_vec(recorded).map_err(io::Error::other)?;
        buf.push(b'\n');
        self.writer.write_all(&buf)?;
        if self.sync_on_append {
            self.writer.sync_data()?;
        }
        Ok(())
    }
}

impl EventStore for FileStore {
    fn append(&mut self, event: Event) -> io::Result<Recorded> {
        let recorded = Recorded {
            event,
            position: self.next_position,
            recorded_at: SystemTime::now(),
        };

        self.write_line(&recorded)?;

        self.next_position += 1;
        Ok(recorded)
    }

    fn read_all(&mut self) -> impl Iterator<Item = io::Result<Recorded>> {
        self.reader.seek(io::SeekFrom::Start(0));

        BufReader::new(&self.reader)
            .lines()
            .map(|line| line.and_then(|l| serde_json::from_str(&l).map_err(io::Error::other)))
    }
}
