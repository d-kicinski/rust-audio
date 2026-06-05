use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rust_audio::{
    dsp::resample::{resample_to_rate, resample_to_rate_into, Converter, ResampleOptions},
    AudioBuffer,
};

const CONVERTERS: &[(Converter, &str)] = &[
    (Converter::ZeroOrderHold, "zero_order_hold"),
    (Converter::Linear, "linear"),
    (Converter::SincFastest, "sinc_fastest"),
    (Converter::SincMedium, "sinc_medium"),
    (Converter::SincBest, "sinc_best"),
];

const CASES: &[(u32, u32, u64, &str)] = &[
    (48_000, 16_000, 1_000, "1s"),
    (48_000, 16_000, 5_000, "5s"),
    (48_000, 16_000, 10_000, "10s"),
    (48_000, 16_000, 30_000, "30s"),
    (44_100, 16_000, 1_000, "1s"),
    (44_100, 16_000, 5_000, "5s"),
    (44_100, 16_000, 10_000, "10s"),
    (44_100, 16_000, 30_000, "30s"),
];

fn make_audio(duration_ms: u64, sample_rate: u32) -> AudioBuffer<f32> {
    let frames = (u64::from(sample_rate) * duration_ms / 1_000) as usize;
    let channel = (0..frames)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            (2.0 * std::f32::consts::PI * 440.0 * t).sin()
        })
        .collect();

    AudioBuffer::new(vec![channel])
}

fn options(converter: Converter) -> ResampleOptions {
    ResampleOptions { converter }
}

fn bench_resamplers(c: &mut Criterion) {
    let mut group = c.benchmark_group("resampling");

    for &(input_rate, output_rate, duration_ms, duration_name) in CASES {
        let input = make_audio(duration_ms, input_rate);
        let input_len = input.samples_per_channel();
        let case_name = format!("{input_rate}_to_{output_rate}_{duration_name}");

        group.throughput(Throughput::Elements(input_len as u64));

        for &(converter, converter_name) in CONVERTERS {
            let options = options(converter);

            group.bench_with_input(
                BenchmarkId::new(converter_name, &case_name),
                &(&input, input_rate, output_rate, options),
                |b, &(input, input_rate, output_rate, options)| {
                    b.iter(|| {
                        let output = resample_to_rate(
                            black_box(input),
                            black_box(input_rate),
                            black_box(output_rate),
                            black_box(options),
                        )
                        .unwrap();
                        black_box(output);
                    });
                },
            );

            let mut output = AudioBuffer::new(Vec::new());
            let converter_into_name = format!("{converter_name}_into");

            group.bench_with_input(
                BenchmarkId::new(converter_into_name, &case_name),
                &(&input, input_rate, output_rate, options),
                |b, &(input, input_rate, output_rate, options)| {
                    b.iter(|| {
                        resample_to_rate_into(
                            black_box(input),
                            black_box(&mut output),
                            black_box(input_rate),
                            black_box(output_rate),
                            black_box(options),
                        )
                        .unwrap();
                        black_box(&output);
                    });
                },
            );
        }
    }

    group.finish();
}

criterion_group!(benches, bench_resamplers);
criterion_main!(benches);
