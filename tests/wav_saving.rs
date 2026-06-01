use std::{fs, path::Path};

use rust_audio::{AudioBuffer, AudioError, SampleData, SampleFormat, WavFile};

fn fixture(name: &str) -> String {
    Path::new("references/AudioFile/tests/test-audio")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

#[test]
fn saves_processed_f32_buffer_as_float_wav_bytes() {
    let input = WavFile::open(fixture("wav_stereo_16bit_44100.wav")).unwrap();
    let mut processed = input.to_f32();

    for channel in &mut processed.channels {
        for sample in channel.iter_mut().take(16) {
            *sample *= 0.5;
        }
    }

    let output = WavFile::from_f32(input.sample_rate(), processed.clone()).unwrap();
    let bytes = output.to_bytes().unwrap();
    let reread = WavFile::from_bytes(&bytes).unwrap();

    assert_eq!(reread.sample_format(), SampleFormat::Float);
    assert_eq!(reread.bits_per_sample(), 32);
    assert_eq!(reread.channels(), input.channels());
    assert_eq!(reread.sample_rate(), input.sample_rate());
    assert_eq!(reread.samples_per_channel(), input.samples_per_channel());

    let SampleData::Float32(samples) = reread.samples() else {
        panic!("expected Float32 samples");
    };

    assert_close(samples.channels[0][1], processed.channels[0][1], 0.0);
    assert_close(samples.channels[0][6], processed.channels[0][6], 0.0);
    assert_close(samples.channels[1][7], processed.channels[1][7], 0.0);
}

#[test]
fn writes_standard_float_wav_header() {
    let samples = AudioBuffer::new(vec![vec![0.0, 0.25], vec![-0.5, 1.0]]);
    let wav = WavFile::from_f32(48_000, samples).unwrap();
    let bytes = wav.to_bytes().unwrap();

    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    assert_eq!(&bytes[12..16], b"fmt ");
    assert_eq!(u32::from_le_bytes(bytes[16..20].try_into().unwrap()), 18);
    assert_eq!(u16::from_le_bytes(bytes[20..22].try_into().unwrap()), 3);
    assert_eq!(u16::from_le_bytes(bytes[22..24].try_into().unwrap()), 2);
    assert_eq!(
        u32::from_le_bytes(bytes[24..28].try_into().unwrap()),
        48_000
    );
    assert_eq!(u16::from_le_bytes(bytes[32..34].try_into().unwrap()), 8);
    assert_eq!(u16::from_le_bytes(bytes[34..36].try_into().unwrap()), 32);
    assert_eq!(u16::from_le_bytes(bytes[36..38].try_into().unwrap()), 0);
    assert_eq!(&bytes[38..42], b"data");
    assert_eq!(u32::from_le_bytes(bytes[42..46].try_into().unwrap()), 16);
}

#[test]
fn save_writes_file_that_can_be_read_back() {
    let path =
        std::env::temp_dir().join(format!("rust-audio-save-test-{}.wav", std::process::id()));
    let samples = AudioBuffer::new(vec![vec![0.0, 0.25, -0.25]]);
    let wav = WavFile::from_f32(22_050, samples).unwrap();

    wav.save(&path).unwrap();
    let reread = WavFile::open(&path).unwrap();
    fs::remove_file(&path).unwrap();

    assert_eq!(reread.channels(), 1);
    assert_eq!(reread.sample_rate(), 22_050);
    assert_eq!(reread.bits_per_sample(), 32);
    assert_eq!(reread.sample_format(), SampleFormat::Float);

    let SampleData::Float32(samples) = reread.samples() else {
        panic!("expected Float32 samples");
    };
    assert_eq!(samples.channels[0], vec![0.0, 0.25, -0.25]);
}

#[test]
fn rejects_invalid_f32_buffers() {
    assert!(matches!(
        WavFile::from_f32(0, AudioBuffer::new(vec![vec![0.0]])),
        Err(AudioError::InvalidBuffer(_))
    ));

    assert!(matches!(
        WavFile::from_f32(44_100, AudioBuffer::new(Vec::new())),
        Err(AudioError::InvalidBuffer(_))
    ));

    assert!(matches!(
        WavFile::from_f32(44_100, AudioBuffer::new(vec![Vec::new()])),
        Err(AudioError::InvalidBuffer(_))
    ));

    assert!(matches!(
        WavFile::from_f32(44_100, AudioBuffer::new(vec![vec![0.0], vec![0.0, 1.0]])),
        Err(AudioError::InvalidBuffer(_))
    ));
}

#[test]
fn rejects_saving_native_pcm_until_explicitly_converted() {
    let input = WavFile::open(fixture("wav_stereo_16bit_44100.wav")).unwrap();

    assert!(matches!(
        input.to_bytes(),
        Err(AudioError::UnsupportedFormat(_))
    ));
}

fn assert_close(actual: f32, expected: f32, tolerance: f32) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {actual} to be within {tolerance} of {expected}"
    );
}
