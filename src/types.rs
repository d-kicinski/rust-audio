use std::{fs, path::Path};

use crate::{wav, AudioError, Result};

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

    pub fn from_f32(sample_rate: u32, samples: AudioBuffer<f32>) -> Result<Self> {
        validate_f32_buffer(sample_rate, &samples)?;

        let spec = WavSpec {
            channels: samples.channel_count() as u16,
            sample_rate,
            bits_per_sample: 32,
            sample_format: SampleFormat::Float,
        };

        Ok(Self::new(spec, SampleData::Float32(samples)))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        wav::parse(bytes)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        fs::write(path, self.to_bytes()?)?;
        Ok(())
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        wav::encode(self)
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

fn validate_f32_buffer(sample_rate: u32, samples: &AudioBuffer<f32>) -> Result<()> {
    if sample_rate == 0 {
        return Err(AudioError::InvalidBuffer(
            "sample rate must be greater than zero".to_string(),
        ));
    }

    let channel_count = samples.channel_count();
    if channel_count == 0 || channel_count > 128 {
        return Err(AudioError::InvalidBuffer(format!(
            "channel count must be in 1..=128, got {channel_count}"
        )));
    }

    let samples_per_channel = samples.samples_per_channel();
    if samples_per_channel == 0 {
        return Err(AudioError::InvalidBuffer(
            "channels must contain at least one sample".to_string(),
        ));
    }

    if samples
        .channels
        .iter()
        .any(|channel| channel.len() != samples_per_channel)
    {
        return Err(AudioError::InvalidBuffer(
            "all channels must have the same number of samples".to_string(),
        ));
    }

    let data_size = channel_count
        .checked_mul(samples_per_channel)
        .and_then(|sample_count| sample_count.checked_mul(std::mem::size_of::<f32>()))
        .ok_or_else(|| AudioError::InvalidBuffer("buffer is too large".to_string()))?;
    let riff_size = 4usize
        .checked_add(8 + 18)
        .and_then(|size| size.checked_add(8))
        .and_then(|size| size.checked_add(data_size))
        .ok_or_else(|| AudioError::InvalidBuffer("buffer is too large".to_string()))?;

    if data_size > u32::MAX as usize || riff_size > u32::MAX as usize {
        return Err(AudioError::InvalidBuffer(
            "buffer is too large for a WAV file".to_string(),
        ));
    }

    Ok(())
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
