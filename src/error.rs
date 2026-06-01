use std::{fmt, io};

pub type Result<T> = std::result::Result<T, AudioError>;

#[derive(Debug)]
pub enum AudioError {
    Io(io::Error),
    InvalidFormat(&'static str),
    UnsupportedFormat(String),
    InconsistentHeader(String),
    InvalidBuffer(String),
    UnexpectedEof,
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::InvalidFormat(message) => write!(f, "invalid audio format: {message}"),
            Self::UnsupportedFormat(message) => write!(f, "unsupported audio format: {message}"),
            Self::InconsistentHeader(message) => write!(f, "inconsistent WAV header: {message}"),
            Self::InvalidBuffer(message) => write!(f, "invalid audio buffer: {message}"),
            Self::UnexpectedEof => write!(f, "unexpected end of file"),
        }
    }
}

impl std::error::Error for AudioError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for AudioError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}
