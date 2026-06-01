mod error;
mod types;
mod wav;

pub use error::{AudioError, Result};
pub use types::{AudioBuffer, SampleData, SampleFormat, WavFile, WavSpec};
