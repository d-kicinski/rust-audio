# rust-audio

Rust WAV reader experiment based on the C++ `AudioFile` reference library.

## Build

```bash
cargo build
```

## Test

```bash
cargo test
```

## Resampling

The library includes zero-order hold, linear, and sinc-based resampling
converters. See [Audio Resampling Algorithms](docs/resampling.md) for the
method behind each converter and guidance on choosing one.

## Run Sample Binary

```bash
cargo run -- references/AudioFile/tests/test-audio/wav_stereo_16bit_44100.wav
```

The binary prints basic WAV metadata and the native sample storage variant.
