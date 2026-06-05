use rust_audio::{
    dsp::resample::{
        resample, resample_into, resample_to_rate, resample_to_rate_into, Converter,
        ResampleOptions,
    },
    AudioBuffer, AudioError,
};

fn options(converter: Converter) -> ResampleOptions {
    ResampleOptions { converter }
}

#[test]
fn rejects_invalid_ratios_and_buffers() {
    let input = AudioBuffer::new(vec![vec![0.0, 1.0, 0.0]]);

    for ratio in [0.0, 1.0 / 257.0, 257.0, f64::NAN, f64::INFINITY] {
        assert!(matches!(
            resample(&input, ratio, options(Converter::Linear)),
            Err(AudioError::InvalidTransform(_))
        ));
    }

    assert!(resample(&input, 1.0 / 256.0, options(Converter::Linear)).is_ok());
    assert!(resample(&input, 256.0, options(Converter::Linear)).is_ok());

    assert!(matches!(
        resample(
            &AudioBuffer::new(vec![vec![0.0], vec![0.0, 1.0]]),
            1.0,
            options(Converter::Linear)
        ),
        Err(AudioError::InvalidTransform(_))
    ));

    assert!(matches!(
        resample_to_rate(&input, 0, 48_000, options(Converter::Linear)),
        Err(AudioError::InvalidTransform(_))
    ));
}

#[test]
fn preserves_shape_and_uses_rate_ratio() {
    let input = AudioBuffer::new(vec![
        vec![0.0, 1.0, 0.0, -1.0, 0.0],
        vec![1.0, 0.5, 0.0, -0.5, -1.0],
    ]);

    let output = resample_to_rate(&input, 24_000, 48_000, options(Converter::Linear)).unwrap();

    assert_eq!(output.channel_count(), 2);
    assert_eq!(output.samples_per_channel(), 10);
    assert_ne!(output.channels[0], output.channels[1]);
}

#[test]
fn zero_order_hold_repeats_source_samples_when_upsampling() {
    let input = AudioBuffer::new(vec![vec![0.0, 1.0, 0.0]]);
    let output = resample(&input, 2.0, options(Converter::ZeroOrderHold)).unwrap();

    assert_eq!(output.channels[0], vec![0.0, 0.0, 1.0, 1.0, 0.0, 0.0]);
}

#[test]
fn linear_interpolates_source_samples_when_upsampling() {
    let input = AudioBuffer::new(vec![vec![0.0, 1.0, 0.0]]);
    let output = resample(&input, 2.0, options(Converter::Linear)).unwrap();

    assert_close(output.channels[0][0], 0.0, 0.0);
    assert_close(output.channels[0][1], 0.5, 0.000001);
    assert_close(output.channels[0][2], 1.0, 0.0);
    assert_close(output.channels[0][3], 0.5, 0.000001);
}

#[test]
fn all_converters_handle_identity_ratio() {
    let input = AudioBuffer::new(vec![vec![0.0, 0.25, -0.5, 0.75, 0.0]]);

    for converter in [
        Converter::ZeroOrderHold,
        Converter::Linear,
        Converter::SincFastest,
        Converter::SincMedium,
        Converter::SincBest,
    ] {
        let output = resample(&input, 1.0, options(converter)).unwrap();
        assert_eq!(output.samples_per_channel(), input.samples_per_channel());
        assert!(output.channels[0].iter().all(|sample| sample.is_finite()));
    }
}

#[test]
fn sinc_converters_produce_finite_nonzero_channel_independent_output() {
    let input = AudioBuffer::new(vec![
        vec![0.0, 0.5, 1.0, 0.5, 0.0, -0.5, -1.0, -0.5],
        vec![1.0, 0.75, 0.25, -0.25, -0.75, -1.0, -0.75, -0.25],
    ]);

    for converter in [
        Converter::SincFastest,
        Converter::SincMedium,
        Converter::SincBest,
    ] {
        let output = resample(&input, 1.5, options(converter)).unwrap();
        assert_eq!(output.channel_count(), 2);
        assert_eq!(output.samples_per_channel(), 12);
        assert!(output
            .channels
            .iter()
            .flatten()
            .all(|sample| sample.is_finite()));
        assert!(output.channels[0].iter().any(|sample| sample.abs() > 0.001));
        assert_ne!(output.channels[0], output.channels[1]);
    }
}

#[test]
fn resample_into_matches_allocating_api() {
    let input = AudioBuffer::new(vec![
        vec![0.0, 0.25, 0.75, 1.0, 0.75, 0.25, 0.0],
        vec![1.0, 0.5, 0.0, -0.5, -1.0, -0.5, 0.0],
    ]);

    for converter in [Converter::Linear, Converter::SincMedium] {
        let options = options(converter);
        let allocated = resample(&input, 1.5, options).unwrap();
        let mut reused = AudioBuffer::new(vec![vec![123.0, 456.0]]);

        resample_into(&input, &mut reused, 1.5, options).unwrap();

        assert_eq!(reused, allocated);
    }
}

#[test]
fn resample_to_rate_into_reuses_existing_channel_storage() {
    let input = AudioBuffer::new(vec![vec![0.0, 1.0, 0.0]]);
    let mut channel = Vec::with_capacity(64);
    channel.extend([99.0, 100.0, 101.0]);
    let capacity = channel.capacity();
    let mut output = AudioBuffer::new(vec![channel]);

    resample_to_rate_into(
        &input,
        &mut output,
        24_000,
        48_000,
        options(Converter::Linear),
    )
    .unwrap();

    assert_eq!(output.channels[0].capacity(), capacity);
    assert_eq!(output.channels[0], vec![0.0, 0.5, 1.0, 0.5, 0.0, 0.0]);
}

#[test]
fn resample_to_rate_into_rejects_invalid_sample_rates() {
    let input = AudioBuffer::new(vec![vec![0.0, 1.0, 0.0]]);
    let mut output = AudioBuffer::new(Vec::new());

    assert!(matches!(
        resample_to_rate_into(&input, &mut output, 0, 48_000, options(Converter::Linear)),
        Err(AudioError::InvalidTransform(_))
    ));
}

fn assert_close(actual: f32, expected: f32, tolerance: f32) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "expected {actual} to be within {tolerance} of {expected}"
    );
}
