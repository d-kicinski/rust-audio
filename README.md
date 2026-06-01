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

## Run Sample Binary

```bash
cargo run -- references/AudioFile/tests/test-audio/wav_stereo_16bit_44100.wav
```

The binary prints basic WAV metadata and the native sample storage variant.
