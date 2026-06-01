use crate::{AudioBuffer, AudioError, Result, SampleData, SampleFormat, WavFile, WavSpec};

const PCM: u16 = 0x0001;
const IEEE_FLOAT: u16 = 0x0003;
const EXTENSIBLE: u16 = 0xfffe;

const PCM_SUBTYPE: [u8; 16] = [
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71,
];
const IEEE_FLOAT_SUBTYPE: [u8; 16] = [
    0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WavEncoding {
    Pcm,
    Float,
}

#[derive(Debug, Clone, Copy)]
struct FormatChunk {
    encoding: WavEncoding,
    channels: u16,
    sample_rate: u32,
    byte_rate: u32,
    block_align: u16,
    bits_per_sample: u16,
}

pub fn parse(bytes: &[u8]) -> Result<WavFile> {
    if bytes.len() < 12 {
        return Err(AudioError::UnexpectedEof);
    }
    if &bytes[0..4] != b"RIFF" {
        return Err(AudioError::InvalidFormat("missing RIFF header"));
    }
    if &bytes[8..12] != b"WAVE" {
        return Err(AudioError::InvalidFormat("missing WAVE header"));
    }

    let mut fmt = None;
    let mut data = None;

    for chunk in ChunkIter::new(bytes, 12) {
        let chunk = chunk?;
        if chunk.id == *b"fmt " {
            fmt = Some(parse_format_chunk(chunk.data)?);
        } else if chunk.id == *b"data" {
            data = Some(chunk.data);
            break;
        }
    }

    let fmt = fmt.ok_or(AudioError::InvalidFormat("missing fmt chunk"))?;
    let data = data.ok_or(AudioError::InvalidFormat("missing data chunk"))?;
    validate_format(fmt, data.len())?;

    let spec = WavSpec {
        channels: fmt.channels,
        sample_rate: fmt.sample_rate,
        bits_per_sample: fmt.bits_per_sample,
        sample_format: match (fmt.encoding, fmt.bits_per_sample) {
            (WavEncoding::Pcm, 8) => SampleFormat::PcmUnsigned,
            (WavEncoding::Pcm, _) => SampleFormat::PcmSigned,
            (WavEncoding::Float, _) => SampleFormat::Float,
        },
    };

    let samples = decode_samples(fmt, data)?;
    Ok(WavFile::new(spec, samples))
}

#[derive(Debug)]
struct Chunk<'a> {
    id: [u8; 4],
    data: &'a [u8],
}

struct ChunkIter<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> ChunkIter<'a> {
    fn new(bytes: &'a [u8], offset: usize) -> Self {
        Self { bytes, offset }
    }
}

impl<'a> Iterator for ChunkIter<'a> {
    type Item = Result<Chunk<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset == self.bytes.len() {
            return None;
        }
        if self.offset + 8 > self.bytes.len() {
            self.offset = self.bytes.len();
            return Some(Err(AudioError::UnexpectedEof));
        }

        let id = [
            self.bytes[self.offset],
            self.bytes[self.offset + 1],
            self.bytes[self.offset + 2],
            self.bytes[self.offset + 3],
        ];
        let size = read_u32_le(self.bytes, self.offset + 4) as usize;
        let data_start = self.offset + 8;
        let data_end = match data_start.checked_add(size) {
            Some(end) if end <= self.bytes.len() => end,
            _ => {
                self.offset = self.bytes.len();
                return Some(Err(AudioError::UnexpectedEof));
            }
        };

        self.offset = data_end + (size % 2);
        if self.offset > self.bytes.len() {
            self.offset = self.bytes.len();
        }

        Some(Ok(Chunk {
            id,
            data: &self.bytes[data_start..data_end],
        }))
    }
}

fn parse_format_chunk(data: &[u8]) -> Result<FormatChunk> {
    if data.len() < 16 {
        return Err(AudioError::UnexpectedEof);
    }

    let audio_format = read_u16_le(data, 0);
    let channels = read_u16_le(data, 2);
    let sample_rate = read_u32_le(data, 4);
    let byte_rate = read_u32_le(data, 8);
    let block_align = read_u16_le(data, 12);
    let bits_per_sample = read_u16_le(data, 14);

    let encoding = match audio_format {
        PCM => WavEncoding::Pcm,
        IEEE_FLOAT => WavEncoding::Float,
        EXTENSIBLE => parse_extensible_encoding(data)?,
        other => {
            return Err(AudioError::UnsupportedFormat(format!(
                "WAV format code {other}"
            )))
        }
    };

    Ok(FormatChunk {
        encoding,
        channels,
        sample_rate,
        byte_rate,
        block_align,
        bits_per_sample,
    })
}

fn parse_extensible_encoding(data: &[u8]) -> Result<WavEncoding> {
    if data.len() < 40 {
        return Err(AudioError::UnexpectedEof);
    }

    let extension_size = read_u16_le(data, 16);
    if extension_size < 22 {
        return Err(AudioError::InvalidFormat(
            "WAVE_FORMAT_EXTENSIBLE chunk is too small",
        ));
    }

    let subtype = &data[24..40];
    if subtype == PCM_SUBTYPE {
        Ok(WavEncoding::Pcm)
    } else if subtype == IEEE_FLOAT_SUBTYPE {
        Ok(WavEncoding::Float)
    } else {
        Err(AudioError::UnsupportedFormat(
            "unsupported WAVE_FORMAT_EXTENSIBLE subtype".to_string(),
        ))
    }
}

fn validate_format(fmt: FormatChunk, data_len: usize) -> Result<()> {
    if fmt.channels == 0 || fmt.channels > 128 {
        return Err(AudioError::InconsistentHeader(format!(
            "invalid channel count {}",
            fmt.channels
        )));
    }
    if fmt.sample_rate == 0 {
        return Err(AudioError::InconsistentHeader(
            "sample rate must be greater than zero".to_string(),
        ));
    }
    if !matches!(fmt.bits_per_sample, 8 | 16 | 24 | 32) {
        return Err(AudioError::UnsupportedFormat(format!(
            "{} bits per sample",
            fmt.bits_per_sample
        )));
    }
    if fmt.encoding == WavEncoding::Float && fmt.bits_per_sample != 32 {
        return Err(AudioError::UnsupportedFormat(
            "only 32-bit IEEE float WAV files are supported".to_string(),
        ));
    }

    let expected_block_align = fmt.channels * (fmt.bits_per_sample / 8);
    if fmt.block_align != expected_block_align {
        return Err(AudioError::InconsistentHeader(format!(
            "block_align {} does not match expected {}",
            fmt.block_align, expected_block_align
        )));
    }

    let expected_byte_rate = fmt.sample_rate * u32::from(expected_block_align);
    if fmt.byte_rate != expected_byte_rate {
        return Err(AudioError::InconsistentHeader(format!(
            "byte_rate {} does not match expected {}",
            fmt.byte_rate, expected_byte_rate
        )));
    }

    if data_len % usize::from(fmt.block_align) != 0 {
        return Err(AudioError::InconsistentHeader(
            "data chunk does not contain whole sample frames".to_string(),
        ));
    }

    Ok(())
}

fn decode_samples(fmt: FormatChunk, data: &[u8]) -> Result<SampleData> {
    match (fmt.encoding, fmt.bits_per_sample) {
        (WavEncoding::Pcm, 8) => Ok(SampleData::PcmU8(decode_frames(
            fmt.channels,
            data,
            1,
            |bytes| bytes[0],
        ))),
        (WavEncoding::Pcm, 16) => Ok(SampleData::PcmI16(decode_frames(
            fmt.channels,
            data,
            2,
            |bytes| i16::from_le_bytes([bytes[0], bytes[1]]),
        ))),
        (WavEncoding::Pcm, 24) => Ok(SampleData::PcmI24(decode_frames(
            fmt.channels,
            data,
            3,
            read_i24_le,
        ))),
        (WavEncoding::Pcm, 32) => Ok(SampleData::PcmI32(decode_frames(
            fmt.channels,
            data,
            4,
            |bytes| i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        ))),
        (WavEncoding::Float, 32) => Ok(SampleData::Float32(decode_frames(
            fmt.channels,
            data,
            4,
            |bytes| f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        ))),
        _ => Err(AudioError::UnsupportedFormat(format!(
            "{:?} {} bits per sample",
            fmt.encoding, fmt.bits_per_sample
        ))),
    }
}

fn decode_frames<T, F>(
    channels: u16,
    data: &[u8],
    bytes_per_sample: usize,
    mut decode: F,
) -> AudioBuffer<T>
where
    F: FnMut(&[u8]) -> T,
{
    let channels = usize::from(channels);
    let frame_size = channels * bytes_per_sample;
    let frame_count = data.len() / frame_size;
    let mut output = Vec::with_capacity(channels);
    for _ in 0..channels {
        output.push(Vec::with_capacity(frame_count));
    }

    for frame in data.chunks_exact(frame_size) {
        for (channel, channel_samples) in output.iter_mut().enumerate() {
            let start = channel * bytes_per_sample;
            let end = start + bytes_per_sample;
            channel_samples.push(decode(&frame[start..end]));
        }
    }

    AudioBuffer::new(output)
}

fn read_i24_le(bytes: &[u8]) -> i32 {
    let value = i32::from(bytes[0]) | (i32::from(bytes[1]) << 8) | (i32::from(bytes[2]) << 16);
    if value & 0x80_0000 != 0 {
        value | !0xFF_FFFF
    } else {
        value
    }
}

fn read_u16_le(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn read_u32_le(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}
