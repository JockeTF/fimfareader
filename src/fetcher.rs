//! Archive fetcher.

use std::fs::File;
use std::io::{BufReader, Read, Seek};
use std::path::Path;
use std::sync::Mutex;

use zip::read::ZipArchive;
use zip::result::ZipError;

use crate::error::{Error, Result};
use crate::parser::parse;
use crate::story::Story;

pub struct Fetcher<T>
where
    T: Read + Seek,
{
    archive: Mutex<ZipArchive<T>>,
    index: Vec<Story>,
}

impl Fetcher<BufReader<File>> {
    pub fn from(path: impl AsRef<Path>) -> Result<Self> {
        use Error::*;

        let file = File::open(path).map_err(|e| match e {
            _ => SourceError("Could not open archive file."),
        })?;

        Self::from_reader(BufReader::with_capacity(8_000_000, file))
    }
}

impl<T> Fetcher<T>
where
    T: Read + Seek,
{
    pub fn from_reader(reader: T) -> Result<Self> {
        let mut handle = Self::open(reader)?;
        let index = Self::load(&mut handle)?;
        let archive = Mutex::new(handle);

        Ok(Self { archive, index })
    }

    fn open(archive: T) -> Result<ZipArchive<T>> {
        use Error::*;
        use ZipError::*;

        ZipArchive::new(archive).map_err(|e| match e {
            InvalidArchive(e) => SourceError(e),
            UnsupportedArchive(e) => SourceError(e),
            _ => SourceError("Could not read archive."),
        })
    }

    fn load(archive: &mut ZipArchive<T>) -> Result<Vec<Story>> {
        use Error::*;
        use ZipError::*;

        let file = archive.by_name("index.json").map_err(|e| match e {
            FileNotFound => SourceError("Missing archive index."),
            _ => SourceError("Could not open archive index."),
        })?;

        parse(BufReader::with_capacity(8_000_000, file))
    }

    pub fn fetch(&self, key: i64) -> Option<&Story> {
        match self.index.binary_search_by_key(&key, |story| story.id) {
            Ok(i) => self.index.get(i),
            Err(_) => None,
        }
    }

    pub fn read(&self, path: &str) -> Result<Vec<u8>> {
        use Error::*;
        use ZipError::*;

        let mut archive = self.archive.lock().map_err(|e| match e {
            _ => SourceError("Could not acquire archive lock."),
        })?;

        let mut file = archive.by_name(path).map_err(|e| match e {
            FileNotFound => SourceError("File not found."),
            _ => SourceError("Could not open file."),
        })?;

        let size = file.size() as usize;
        let mut buf = Vec::with_capacity(size);

        file.read_to_end(&mut buf)
            .map_err(|_| SourceError("Could not read file."))?;

        Ok(buf)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Story> {
        self.index.iter()
    }
}
