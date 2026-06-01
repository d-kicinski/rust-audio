use std::path::Path;

use rust_audio::{AudioError, SampleData, SampleFormat, WavFile};

fn fixture(name: &str) -> String {
    Path::new("references/AudioFile/tests/test-audio")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

#[test]
fn reads_reference_fixture_metadata_and_storage_variants() {
    let cases = [
        ("wav_stereo_8bit_44100.wav", 2, 44_100, 8, 352_800, "u8"),
        ("wav_stereo_8bit_48000.wav", 2, 48_000, 8, 384_000, "u8"),
        ("wav_stereo_16bit_44100.wav", 2, 44_100, 16, 352_800, "i16"),
        ("wav_stereo_16bit_48000.wav", 2, 48_000, 16, 384_000, "i16"),
        ("wav_mono_16bit_44100.wav", 1, 44_100, 16, 352_800, "i16"),
        ("wav_mono_16bit_48000.wav", 1, 48_000, 16, 384_000, "i16"),
        ("wav_stereo_24bit_44100.wav", 2, 44_100, 24, 352_800, "i24"),
        ("wav_stereo_24bit_48000.wav", 2, 48_000, 24, 384_000, "i24"),
        ("wav_stereo_32bit_44100.wav", 2, 44_100, 32, 384_873, "f32"),
        ("wav_stereo_32bit_48000.wav", 2, 48_000, 32, 418_909, "f32"),
        ("wav_8chan_24bit_48000.wav", 8, 48_000, 24, 191_524, "i24"),
    ];

    for (file, channels, sample_rate, bit_depth, samples_per_channel, storage) in cases {
        let wav = WavFile::open(fixture(file)).unwrap();
        assert_eq!(wav.channels(), channels, "{file}");
        assert_eq!(wav.sample_rate(), sample_rate, "{file}");
        assert_eq!(wav.bits_per_sample(), bit_depth, "{file}");
        assert_eq!(wav.samples_per_channel(), samples_per_channel, "{file}");
        assert_eq!(
            wav.samples().channel_count(),
            usize::from(channels),
            "{file}"
        );
        assert_eq!(
            wav.sample_format(),
            match storage {
                "u8" => SampleFormat::PcmUnsigned,
                "f32" => SampleFormat::Float,
                _ => SampleFormat::PcmSigned,
            },
            "{file}"
        );

        match (storage, wav.samples()) {
            ("u8", SampleData::PcmU8(_)) => {}
            ("i16", SampleData::PcmI16(_)) => {}
            ("i24", SampleData::PcmI24(_)) => {}
            ("i32", SampleData::PcmI32(_)) => {}
            ("f32", SampleData::Float32(_)) => {}
            _ => panic!("unexpected storage for {file}: {:?}", wav.samples()),
        }
    }
}

#[test]
fn preserves_native_pcm_values_and_converts_explicitly_to_f32() {
    let wav = WavFile::open(fixture("wav_stereo_16bit_44100.wav")).unwrap();
    let SampleData::PcmI16(samples) = wav.samples() else {
        panic!("expected i16 samples");
    };

    assert_eq!(
        &samples.channels[0][0..8],
        &[0, -3, -18, -39, -42, 65, 346, 475]
    );
    assert_eq!(
        &samples.channels[1][0..8],
        &[0, -3, -21, -50, -57, 60, 400, 600]
    );

    let converted = wav.to_f32();
    assert_close(converted.channels[0][1], -9.1552734e-05, 0.0000001);
    assert_close(converted.channels[0][6], 346.0 / 32767.0, 0.0000001);
    assert_close(converted.channels[1][7], 600.0 / 32767.0, 0.0000001);
}

#[test]
fn preserves_unsigned_8_bit_pcm_and_converts_explicitly_to_f32() {
    let wav = WavFile::open(fixture("wav_stereo_8bit_44100.wav")).unwrap();
    let SampleData::PcmU8(samples) = wav.samples() else {
        panic!("expected u8 samples");
    };

    assert_eq!(
        &samples.channels[0][0..8],
        &[128, 128, 128, 127, 129, 127, 129, 127]
    );
    assert_eq!(
        &samples.channels[1][0..8],
        &[128, 128, 129, 126, 130, 125, 131, 124]
    );

    let converted = wav.to_f32();
    assert_close(converted.channels[0][3], -0.007874016, 0.0000001);
    assert_close(converted.channels[1][6], 0.023622047, 0.0000001);
}

#[test]
fn sign_extends_24_bit_pcm_into_i32() {
    let wav = WavFile::open(fixture("wav_stereo_24bit_44100.wav")).unwrap();
    let SampleData::PcmI24(samples) = wav.samples() else {
        panic!("expected i24 samples");
    };

    assert_eq!(
        &samples.channels[0][0..8],
        &[652, 3690, 56566, 137423, 127077, 28654, -104877, -182413]
    );
    assert_eq!(
        &samples.channels[1][0..8],
        &[1219, 2539, 51362, 128474, 119243, 49259, -29961, -96478]
    );
}

#[test]
fn rejects_invalid_inputs() {
    assert!(matches!(
        WavFile::from_bytes(&[]),
        Err(AudioError::UnexpectedEof)
    ));

    assert!(matches!(
        WavFile::from_bytes(b"not a wav file"),
        Err(AudioError::InvalidFormat(_))
    ));

    let mut missing_data = Vec::new();
    missing_data.extend_from_slice(b"RIFF");
    missing_data.extend_from_slice(&36u32.to_le_bytes());
    missing_data.extend_from_slice(b"WAVE");
    missing_data.extend_from_slice(b"fmt ");
    missing_data.extend_from_slice(&16u32.to_le_bytes());
    missing_data.extend_from_slice(&1u16.to_le_bytes());
    missing_data.extend_from_slice(&1u16.to_le_bytes());
    missing_data.extend_from_slice(&44_100u32.to_le_bytes());
    missing_data.extend_from_slice(&88_200u32.to_le_bytes());
    missing_data.extend_from_slice(&2u16.to_le_bytes());
    missing_data.extend_from_slice(&16u16.to_le_bytes());

    assert!(matches!(
        WavFile::from_bytes(&missing_data),
        Err(AudioError::InvalidFormat("missing data chunk"))
    ));
}

fn assert_close(actual: f32, expected: f32, tolerance: f32) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {actual} to be within {tolerance} of {expected}"
    );
}
