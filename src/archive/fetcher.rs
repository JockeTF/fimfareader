//! Archive fetcher.

use std::fs::File;
use std::io::ErrorKind as IoErrorKind;
use std::io::{BufReader, Read, Seek};
use std::path::Path;
use std::sync::Mutex;

use zip::read::ZipArchive;
use zip::result::ZipError;

use super::parser::parse;
use super::story::Story;
use crate::error::{Error, Result};

pub struct Fetcher<T>
where
    T: Read + Seek,
{
    archive: Mutex<ZipArchive<T>>,
    index: Vec<Story>,
}

impl Fetcher<BufReader<File>> {
    pub fn from(path: impl AsRef<Path>) -> Result<Self> {
        use IoErrorKind::*;

        let file = File::open(path).map_err(|e| match e.kind() {
            NotFound => Error::archive("File not found"),
            _ => Error::archive("Could not open file"),
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
        use ZipError::*;

        ZipArchive::new(archive).map_err(|e| match e {
            InvalidArchive(e) => Error::archive(e),
            UnsupportedArchive(e) => Error::archive(e),
            _ => Error::archive("Unknown ZIP-file issue"),
        })
    }

    fn load(archive: &mut ZipArchive<T>) -> Result<Vec<Story>> {
        use ZipError::*;

        let file = archive.by_name("index.json").map_err(|e| match e {
            FileNotFound => Error::archive("Missing story index"),
            _ => Error::archive("Could not open story index"),
        })?;

        parse(BufReader::with_capacity(8_000_000, file)).map_err(Error::index)
    }

    pub fn fetch(&self, key: i64) -> Option<&Story> {
        match self.index.binary_search_by_key(&key, |story| story.id) {
            Ok(i) => self.index.get(i),
            Err(_) => None,
        }
    }

    pub fn read(&self, story: &Story) -> Result<Vec<u8>> {
        use ZipError::*;

        let path = &story.archive.path;

        let mut archive = self.archive.lock().map_err(|e| match e {
            _ => Error::archive("Could not acquire fetcher lock"),
        })?;

        let mut file = archive.by_name(path).map_err(|e| match e {
            FileNotFound => Error::archive("Missing story data"),
            _ => Error::archive("Could not open story data"),
        })?;

        let size = file.size() as usize;
        let mut buf = Vec::with_capacity(size);

        file.read_to_end(&mut buf).map_err(|e| match e {
            _ => Error::archive("Could not read story data"),
        })?;

        Ok(buf)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Story> {
        self.index.iter()
    }

    pub fn filter<F>(&self, function: &F) -> Vec<&Story>
    where
        F: Sync + Fn(&Story) -> bool,
    {
        self.index.iter().filter(|s| function(s)).collect()
    }
}
