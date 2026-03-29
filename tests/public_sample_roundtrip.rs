#![cfg(feature = "std")]

use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

use rvoip_g729ab::FRAME_SAMPLES;

const SAMPLE_URL: &str =
    "https://www.voiptroubleshooter.com/open_speech/american/OSR_us_000_0030_8k.wav";
const FRAME_BYTES: usize = FRAME_SAMPLES * 2;

#[test]
#[ignore = "downloads a public telephony sample and runs an end-to-end CLI round trip"]
fn public_sample_roundtrip_via_cli() -> Result<(), Box<dyn Error>> {
    let temp_dir = create_temp_dir()?;
    let wav_path = temp_dir.join("sample.wav");
    let input_pcm_path = temp_dir.join("input.pcm");
    let encoded_path = temp_dir.join("sample.g729");
    let decoded_path = temp_dir.join("decoded.pcm");

    let result = (|| -> Result<(), Box<dyn Error>> {
        download_sample(&wav_path)?;
        let wav_bytes = fs::read(&wav_path)?;
        let pcm_bytes = extract_mono_8k_pcm16(&wav_bytes)?;
        let usable_len = (pcm_bytes.len() / FRAME_BYTES) * FRAME_BYTES;
        if usable_len == 0 {
            return Err("downloaded sample did not contain a complete 80-sample frame".into());
        }
        fs::write(&input_pcm_path, &pcm_bytes[..usable_len])?;

        run_cli([
            "encode",
            "--no-vad",
            input_pcm_path.to_str().ok_or("non-utf8 input path")?,
            encoded_path.to_str().ok_or("non-utf8 encoded path")?,
        ])?;
        run_cli([
            "decode",
            encoded_path.to_str().ok_or("non-utf8 encoded path")?,
            decoded_path.to_str().ok_or("non-utf8 decoded path")?,
        ])?;

        let decoded = fs::read(&decoded_path)?;
        if decoded.len() != usable_len {
            return Err(format!(
                "decoded PCM length mismatch: expected {usable_len} bytes, got {}",
                decoded.len()
            )
            .into());
        }

        let energy: i64 = decoded
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]) as i64)
            .map(i64::abs)
            .sum();
        if energy == 0 {
            return Err("decoded PCM was entirely silent".into());
        }

        Ok(())
    })();

    let _ = fs::remove_dir_all(&temp_dir);
    result
}

fn create_temp_dir() -> Result<PathBuf, Box<dyn Error>> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = env::temp_dir().join(format!(
        "rvoip_g729ab_public_sample_{}_{}",
        process::id(),
        stamp
    ));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn download_sample(dest: &Path) -> Result<(), Box<dyn Error>> {
    match Command::new("curl")
        .args(["-fsSL", SAMPLE_URL, "-o"])
        .arg(dest)
        .status()
    {
        Ok(status) if status.success() => return Ok(()),
        Ok(status) => {
            return Err(format!("curl failed with exit status {status}").into());
        }
        Err(err) if err.kind() != std::io::ErrorKind::NotFound => {
            return Err(format!("failed to start curl: {err}").into());
        }
        Err(_) => {}
    }

    let status = Command::new("python3")
        .args([
            "-c",
            "import sys, urllib.request; urllib.request.urlretrieve(sys.argv[1], sys.argv[2])",
            SAMPLE_URL,
        ])
        .arg(dest)
        .status()
        .map_err(|err| format!("failed to start python3 fallback downloader: {err}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("python3 fallback downloader failed with exit status {status}").into())
    }
}

fn run_cli<const N: usize>(args: [&str; N]) -> Result<(), Box<dyn Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_g729-cli"))
        .args(args)
        .output()?;

    if output.status.success() {
        return Ok(());
    }

    Err(format!(
        "g729-cli {:?} failed with status {}:\nstdout:\n{}\nstderr:\n{}",
        &args,
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
    .into())
}

fn extract_mono_8k_pcm16(wav: &[u8]) -> Result<&[u8], Box<dyn Error>> {
    if wav.len() < 12 || &wav[..4] != b"RIFF" || &wav[8..12] != b"WAVE" {
        return Err("downloaded file was not a RIFF/WAVE file".into());
    }

    let mut pos = 12usize;
    let mut fmt_seen = false;
    let mut data = None;

    while pos + 8 <= wav.len() {
        let id = &wav[pos..pos + 4];
        let size = u32::from_le_bytes(wav[pos + 4..pos + 8].try_into()?) as usize;
        pos += 8;
        if pos + size > wav.len() {
            return Err("malformed WAV chunk length".into());
        }
        let chunk = &wav[pos..pos + size];

        if id == b"fmt " {
            if chunk.len() < 16 {
                return Err("fmt chunk too short".into());
            }
            let audio_format = u16::from_le_bytes(chunk[0..2].try_into()?);
            let channels = u16::from_le_bytes(chunk[2..4].try_into()?);
            let sample_rate = u32::from_le_bytes(chunk[4..8].try_into()?);
            let bits_per_sample = u16::from_le_bytes(chunk[14..16].try_into()?);
            if audio_format != 1 {
                return Err(format!("expected PCM WAV, got format code {audio_format}").into());
            }
            if channels != 1 || sample_rate != 8000 || bits_per_sample != 16 {
                return Err(format!(
                    "expected mono 8kHz 16-bit WAV, got channels={channels}, sample_rate={sample_rate}, bits_per_sample={bits_per_sample}"
                )
                .into());
            }
            fmt_seen = true;
        } else if id == b"data" {
            data = Some(chunk);
        }

        pos += size;
        if size % 2 == 1 {
            pos += 1;
        }
    }

    if !fmt_seen {
        return Err("WAV file did not contain a fmt chunk".into());
    }

    data.ok_or_else(|| "WAV file did not contain a data chunk".into())
}
