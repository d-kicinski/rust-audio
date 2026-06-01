use std::{fs, path::Path};

use crate::{wav, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WavSpec {
    pub channels: u16,
    pub sample_rate: u32,
    pub bits_per_sample: u16,
    pub sample_format: SampleFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SampleFormat {
    PcmUnsigned,
    PcmSigned,
    Float,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioBuffer<T> {
    pub channels: Vec<Vec<T>>,
}

impl<T> AudioBuffer<T> {
    pub fn new(channels: Vec<Vec<T>>) -> Self {
        Self { channels }
    }

    pub fn channel_count(&self) -> usize {
        self.channels.len()
    }

    pub fn samples_per_channel(&self) -> usize {
        self.channels.first().map_or(0, Vec::len)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SampleData {
    PcmU8(AudioBuffer<u8>),
    PcmI16(AudioBuffer<i16>),
    PcmI24(AudioBuffer<i32>),
    PcmI32(AudioBuffer<i32>),
    Float32(AudioBuffer<f32>),
}

impl SampleData {
    pub fn channel_count(&self) -> usize {
        match self {
            Self::PcmU8(buffer) => buffer.channel_count(),
            Self::PcmI16(buffer) => buffer.channel_count(),
            Self::PcmI24(buffer) => buffer.channel_count(),
            Self::PcmI32(buffer) => buffer.channel_count(),
            Self::Float32(buffer) => buffer.channel_count(),
        }
    }

    pub fn samples_per_channel(&self) -> usize {
        match self {
            Self::PcmU8(buffer) => buffer.samples_per_channel(),
            Self::PcmI16(buffer) => buffer.samples_per_channel(),
            Self::PcmI24(buffer) => buffer.samples_per_channel(),
            Self::PcmI32(buffer) => buffer.samples_per_channel(),
            Self::Float32(buffer) => buffer.samples_per_channel(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WavFile {
    spec: WavSpec,
    samples: SampleData,
}

impl WavFile {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let bytes = fs::read(path)?;
        Self::from_bytes(&bytes)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        wav::parse(bytes)
    }

    pub fn spec(&self) -> &WavSpec {
        &self.spec
    }

    pub fn samples(&self) -> &SampleData {
        &self.samples
    }

    pub(crate) fn new(spec: WavSpec, samples: SampleData) -> Self {
        Self { spec, samples }
    }

    pub fn channels(&self) -> u16 {
        self.spec.channels
    }

    pub fn sample_rate(&self) -> u32 {
        self.spec.sample_rate
    }

    pub fn bits_per_sample(&self) -> u16 {
        self.spec.bits_per_sample
    }

    pub fn sample_format(&self) -> SampleFormat {
        self.spec.sample_format
    }

    pub fn samples_per_channel(&self) -> usize {
        self.samples.samples_per_channel()
    }

    pub fn duration_seconds(&self) -> f64 {
        if self.spec.sample_rate == 0 {
            0.0
        } else {
            self.samples_per_channel() as f64 / self.spec.sample_rate as f64
        }
    }

    pub fn to_f32(&self) -> AudioBuffer<f32> {
        match &self.samples {
            SampleData::PcmU8(buffer) => {
                map_buffer(buffer, |sample| (sample as f32 - 128.0) / 127.0)
            }
            SampleData::PcmI16(buffer) => {
                map_buffer(buffer, |sample| sample as f32 / i16::MAX as f32)
            }
            SampleData::PcmI24(buffer) => map_buffer(buffer, |sample| sample as f32 / 8_388_607.0),
            SampleData::PcmI32(buffer) => {
                map_buffer(buffer, |sample| sample as f32 / i32::MAX as f32)
            }
            SampleData::Float32(buffer) => buffer.clone(),
        }
    }
}

fn map_buffer<T, F>(buffer: &AudioBuffer<T>, mut f: F) -> AudioBuffer<f32>
where
    T: Copy,
    F: FnMut(T) -> f32,
{
    AudioBuffer::new(
        buffer
            .channels
            .iter()
            .map(|channel| channel.iter().copied().map(&mut f).collect())
            .collect(),
    )
}
