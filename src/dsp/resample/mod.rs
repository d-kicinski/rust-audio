mod coeffs;

use crate::{AudioBuffer, AudioError, Result};

const MAX_RATIO: f64 = 256.0;
const MAX_CHANNELS: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Converter {
    ZeroOrderHold,
    Linear,
    SincFastest,
    SincMedium,
    SincBest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResampleOptions {
    pub converter: Converter,
}

impl Default for ResampleOptions {
    fn default() -> Self {
        Self {
            converter: Converter::SincMedium,
        }
    }
}

pub fn resample(
    input: &AudioBuffer<f32>,
    ratio: f64,
    options: ResampleOptions,
) -> Result<AudioBuffer<f32>> {
    validate_ratio(ratio)?;
    validate_buffer(input)?;

    let input_frames = input.samples_per_channel();
    let output_frames = output_frame_count(input_frames, ratio)?;
    let channels = input.channel_count();
    let mut output = Vec::with_capacity(channels);

    for input_channel in &input.channels {
        let mut output_channel = Vec::with_capacity(output_frames);
        resample_channel_into(
            input_channel,
            &mut output_channel,
            output_frames,
            ratio,
            options,
        );
        output.push(output_channel);
    }

    Ok(AudioBuffer::new(output))
}

pub fn resample_into(
    input: &AudioBuffer<f32>,
    output: &mut AudioBuffer<f32>,
    ratio: f64,
    options: ResampleOptions,
) -> Result<()> {
    validate_ratio(ratio)?;
    validate_buffer(input)?;

    let input_frames = input.samples_per_channel();
    let output_frames = output_frame_count(input_frames, ratio)?;

    let channels = input.channel_count();
    output.channels.truncate(channels);
    output
        .channels
        .resize_with(channels, || Vec::with_capacity(output_frames));

    for (input_channel, output_channel) in input.channels.iter().zip(&mut output.channels) {
        resample_channel_into(input_channel, output_channel, output_frames, ratio, options);
    }

    Ok(())
}

pub fn resample_to_rate(
    input: &AudioBuffer<f32>,
    input_rate: u32,
    output_rate: u32,
    options: ResampleOptions,
) -> Result<AudioBuffer<f32>> {
    if input_rate == 0 || output_rate == 0 {
        return Err(AudioError::InvalidTransform(
            "sample rates must be greater than zero".to_string(),
        ));
    }

    resample(
        input,
        f64::from(output_rate) / f64::from(input_rate),
        options,
    )
}

pub fn resample_to_rate_into(
    input: &AudioBuffer<f32>,
    output: &mut AudioBuffer<f32>,
    input_rate: u32,
    output_rate: u32,
    options: ResampleOptions,
) -> Result<()> {
    if input_rate == 0 || output_rate == 0 {
        return Err(AudioError::InvalidTransform(
            "sample rates must be greater than zero".to_string(),
        ));
    }

    resample_into(
        input,
        output,
        f64::from(output_rate) / f64::from(input_rate),
        options,
    )
}

fn validate_ratio(ratio: f64) -> Result<()> {
    if !ratio.is_finite() {
        return Err(AudioError::InvalidTransform(
            "ratio must be finite".to_string(),
        ));
    }
    if !(1.0 / MAX_RATIO..=MAX_RATIO).contains(&ratio) {
        return Err(AudioError::InvalidTransform(format!(
            "ratio must be in [1/{MAX_RATIO:.0}, {MAX_RATIO:.0}], got {ratio}"
        )));
    }
    Ok(())
}

fn validate_buffer(input: &AudioBuffer<f32>) -> Result<()> {
    let channel_count = input.channel_count();
    if channel_count == 0 || channel_count > MAX_CHANNELS {
        return Err(AudioError::InvalidTransform(format!(
            "channel count must be in 1..={MAX_CHANNELS}, got {channel_count}"
        )));
    }

    let frames = input.samples_per_channel();
    if frames == 0 {
        return Err(AudioError::InvalidTransform(
            "channels must contain at least one sample".to_string(),
        ));
    }

    if input.channels.iter().any(|channel| channel.len() != frames) {
        return Err(AudioError::InvalidTransform(
            "all channels must have the same number of samples".to_string(),
        ));
    }

    Ok(())
}

fn output_frame_count(input_frames: usize, ratio: f64) -> Result<usize> {
    let frames = (input_frames as f64 * ratio).floor().max(1.0);
    if frames > usize::MAX as f64 {
        return Err(AudioError::InvalidTransform(
            "resampled output frame count is invalid".to_string(),
        ));
    }
    Ok(frames as usize)
}

fn resample_channel_into(
    input: &[f32],
    output: &mut Vec<f32>,
    output_frames: usize,
    ratio: f64,
    options: ResampleOptions,
) {
    match options.converter {
        Converter::ZeroOrderHold => resample_zoh_into(input, output, output_frames, ratio),
        Converter::Linear => resample_linear_into(input, output, output_frames, ratio),
        Converter::SincFastest => {
            resample_sinc_into(input, output, output_frames, ratio, coeffs::FASTEST)
        }
        Converter::SincMedium => {
            resample_sinc_into(input, output, output_frames, ratio, coeffs::MEDIUM)
        }
        Converter::SincBest => {
            resample_sinc_into(input, output, output_frames, ratio, coeffs::BEST)
        }
    }
}

fn resample_zoh_into(input: &[f32], output: &mut Vec<f32>, output_frames: usize, ratio: f64) {
    output.clear();
    output.reserve(output_frames);
    for output_index in 0..output_frames {
        let input_index = (output_index as f64 / ratio).floor() as usize;
        output.push(input[input_index.min(input.len() - 1)]);
    }
}

fn resample_linear_into(input: &[f32], output: &mut Vec<f32>, output_frames: usize, ratio: f64) {
    output.clear();
    output.reserve(output_frames);
    for output_index in 0..output_frames {
        let position = output_index as f64 / ratio;
        let left = position.floor() as usize;
        let right = (left + 1).min(input.len() - 1);
        let fraction = position - left as f64;
        let sample = input[left] as f64 + fraction * (input[right] as f64 - input[left] as f64);
        output.push(sample as f32);
    }
}

fn resample_sinc_into(
    input: &[f32],
    output: &mut Vec<f32>,
    output_frames: usize,
    ratio: f64,
    coeffs: coeffs::SincCoeffs,
) {
    output.clear();
    output.reserve(output_frames);
    let scale = ratio.min(1.0);
    let table_step = coeffs.increment as f64 * scale;
    let half_len = ((coeffs.values.len() - 2) as f64 / table_step).ceil() as isize;

    for output_index in 0..output_frames {
        let position = output_index as f64 / ratio;
        let center = position.floor() as isize;
        let mut acc = 0.0;

        for input_index in center - half_len..=center + half_len {
            if input_index < 0 || input_index >= input.len() as isize {
                continue;
            }

            let distance = (position - input_index as f64).abs() * table_step;
            let Some(coeff) = interpolated_coeff(coeffs.values, distance) else {
                continue;
            };
            acc += coeff * f64::from(input[input_index as usize]);
        }

        output.push((acc * scale) as f32);
    }
}

fn interpolated_coeff(coeffs: &[f32], index: f64) -> Option<f64> {
    if index < 0.0 {
        return None;
    }

    let whole = index.floor() as usize;
    if whole + 1 >= coeffs.len() {
        return None;
    }

    let fraction = index - whole as f64;
    let left = f64::from(coeffs[whole]);
    let right = f64::from(coeffs[whole + 1]);
    Some(left + fraction * (right - left))
}
