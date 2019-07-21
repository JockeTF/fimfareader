//! Error types.

#[derive(Debug)]
pub enum Error {
    InvalidStory(),
    SourceError(&'static str),
}

pub type Result<T> = std::result::Result<T, Error>;
