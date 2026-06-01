use std::{env, process};

use rust_audio::{SampleData, WavFile};

fn main() {
    let Some(path) = env::args().nth(1) else {
        eprintln!("usage: rust-audio <path-to-wav>");
        process::exit(2);
    };

    match WavFile::open(&path) {
        Ok(wav) => {
            println!("file: {path}");
            println!("format: {:?}", wav.sample_format());
            println!("channels: {}", wav.channels());
            println!("sample_rate: {}", wav.sample_rate());
            println!("bits_per_sample: {}", wav.bits_per_sample());
            println!("samples_per_channel: {}", wav.samples_per_channel());
            println!("duration_seconds: {:.6}", wav.duration_seconds());
            println!("storage: {}", storage_name(wav.samples()));
        }
        Err(err) => {
            eprintln!("error: {err}");
            process::exit(1);
        }
    }
}

fn storage_name(samples: &SampleData) -> &'static str {
    match samples {
        SampleData::PcmU8(_) => "PcmU8",
        SampleData::PcmI16(_) => "PcmI16",
        SampleData::PcmI24(_) => "PcmI24",
        SampleData::PcmI32(_) => "PcmI32",
        SampleData::Float32(_) => "Float32",
    }
}
