//! Error types.

#[derive(Debug)]
pub enum Error {
    InvalidStory(),
    SourceError(&'static str),
    UserError(&'static str),
}

pub type Result<T> = std::result::Result<T, Error>;
