#![cfg(feature = "desktop")]

use player_render::{AudioExportFormat, export_audio};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "foxide-audio-export-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn export_refuses_to_replace_an_existing_file() {
    let scratch = Scratch::new();
    let source = scratch.0.join("source.wav");
    let destination = scratch.0.join("existing.wav");
    fs::write(&source, b"source audio").unwrap();
    fs::write(&destination, b"keep this file").unwrap();

    let result = export_audio(&source, &destination, AudioExportFormat::WavPcm16);

    assert!(result.unwrap_err().contains("already exists"));
    assert_eq!(fs::read(destination).unwrap(), b"keep this file");
}

#[test]
fn export_rejects_a_destination_with_the_wrong_extension_before_loading_the_engine() {
    let scratch = Scratch::new();
    let source = scratch.0.join("source.wav");
    let destination = scratch.0.join("output.flac");
    fs::write(&source, b"source audio").unwrap();

    let result = export_audio(&source, &destination, AudioExportFormat::WavPcm16);

    assert!(result.unwrap_err().contains(".wav destination"));
    assert!(!destination.exists());
}

#[test]
fn export_rejects_a_missing_source_before_loading_the_engine() {
    let scratch = Scratch::new();
    let source = scratch.0.join("missing.wav");
    let destination = scratch.0.join("output.wav");

    let result = export_audio(&source, &destination, AudioExportFormat::WavPcm16);

    assert!(result.unwrap_err().contains("existing local media file"));
    assert!(!destination.exists());
}

#[test]
#[ignore = "Requires libmpv encoding support; run in the audio-export CI workflow"]
fn native_audio_export_creates_playable_wav_and_flac_files() {
    let scratch = Scratch::new();
    let source = scratch.0.join("source.wav");
    write_test_wav(&source);
    let wav = scratch.0.join("export.wav");
    let flac = scratch.0.join("export.flac");

    export_audio(&source, &wav, AudioExportFormat::WavPcm16).unwrap();
    export_audio(&source, &flac, AudioExportFormat::Flac).unwrap();

    let wav_bytes = fs::read(wav).unwrap();
    assert_eq!(&wav_bytes[..4], b"RIFF");
    assert_eq!(&wav_bytes[8..12], b"WAVE");
    assert!(wav_bytes.len() >= 44 + 44_100 * 2);
    let flac_bytes = fs::read(flac).unwrap();
    assert_eq!(&flac_bytes[..4], b"fLaC");
    assert!(flac_bytes.len() > 100);
    let output_samples = (wav_pcm16_samples(&wav_bytes), flac_samples(&flac_bytes));
    assert_eq!(output_samples, (44_100, 44_100));
}

fn wav_pcm16_samples(bytes: &[u8]) -> u32 {
    let mut offset = 12;
    while offset + 8 <= bytes.len() {
        let chunk = &bytes[offset..offset + 4];
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap());
        if chunk == b"data" {
            assert_eq!(size % 2, 0, "PCM16 data must contain complete samples");
            return size / 2;
        }
        offset += 8 + size as usize + (size as usize % 2);
    }
    panic!("WAV export has no data chunk");
}

fn flac_samples(bytes: &[u8]) -> u64 {
    assert_eq!(&bytes[..4], b"fLaC");
    assert_eq!(
        bytes[4] & 0x7f,
        0,
        "first FLAC metadata block must be STREAMINFO"
    );
    assert_eq!(u32::from_be_bytes([0, bytes[5], bytes[6], bytes[7]]), 34);
    let stream_info = u64::from_be_bytes(bytes[18..26].try_into().unwrap());
    stream_info & ((1_u64 << 36) - 1)
}

fn write_test_wav(path: &std::path::Path) {
    let sample_rate = 44_100u32;
    let samples = sample_rate;
    let data_len = samples * 2;
    let mut bytes = Vec::with_capacity(44 + data_len as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for index in 0..samples {
        let phase = (index as f64 * std::f64::consts::TAU * 440.0 / sample_rate as f64).sin();
        let sample = (phase * i16::MAX as f64 * 0.5) as i16;
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    fs::write(path, bytes).unwrap();
}
