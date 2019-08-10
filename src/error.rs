//! Error types.

use std::error::Error as StdError;
use std::fmt::Result as FmtResult;
use std::fmt::{Display, Formatter};
use std::result::Result as StdResult;

use serde_json::error::Error as SerdeError;

use self::ErrorKind::*;

pub type Result<T> = StdResult<T, Error>;

#[derive(Clone, Debug)]
pub enum ErrorKind {
    ArchiveError,
    IndexError,
    InvalidStory,
    UsageError,
    QueryError,
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    message: Option<String>,
    source: Option<Box<dyn StdError + 'static>>,
}

pub struct ErrorBuilder(Error);

impl ErrorBuilder {
    pub fn new(kind: ErrorKind) -> Self {
        ErrorBuilder(Error {
            kind,
            message: None,
            source: None,
        })
    }

    pub fn message(mut self, message: impl ToString) -> Self {
        self.0.message = Some(message.to_string());
        self
    }

    pub fn source(mut self, source: impl StdError + 'static) -> Self {
        self.0.source = Some(Box::new(source));
        self
    }

    pub fn build(self) -> Error {
        self.0
    }
}

impl Error {
    pub fn archive(message: impl ToString) -> Self {
        ErrorBuilder::new(ArchiveError).message(message).build()
    }

    pub fn index(error: SerdeError) -> Self {
        ErrorBuilder::new(IndexError)
            .message(&error)
            .source(error)
            .build()
    }

    pub fn invalid() -> Self {
        ErrorBuilder::new(InvalidStory).build()
    }

    pub fn usage(message: impl ToString) -> Self {
        ErrorBuilder::new(UsageError).message(message).build()
    }

    pub fn query(message: impl ToString) -> Self {
        ErrorBuilder::new(QueryError).message(message).build()
    }

    pub fn kind(&self) -> ErrorKind {
        self.kind.clone()
    }

    pub fn message(&self) -> Option<&String> {
        self.message.as_ref()
    }
}

fn lower(message: &str) -> String {
    let mut chars = message.chars();

    let head: String = match chars.next() {
        Some(c) => c.to_lowercase().collect(),
        None => return String::from(message),
    };

    format!("{}{}", head, chars.as_str())
}

impl Display for ErrorKind {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        let message = match self {
            ArchiveError => "Archive error",
            IndexError => "Index error",
            InvalidStory => "Invalid story",
            UsageError => "Usage error",
            QueryError => "Query error",
        };

        write!(f, "{}", message)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        let message = match &self.message {
            Some(message) => message,
            None => "Unknown cause",
        };

        let kind = self.kind();
        let info = lower(message);

        write!(f, "{}, {}.", kind, info)
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source.as_ref().map(Box::as_ref)
    }
}
