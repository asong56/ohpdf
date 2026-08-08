//! Pipeline mode: read a PDF from stdin, apply one operation, write to stdout.
//!
//! This enables shell pipelines like:
//!   cat in.pdf | ohpdf-cli rotate 90 | ohpdf-cli compress | ohpdf-cli encrypt secret > out.pdf
//!
//! Each stage:
//!   1. Reads raw bytes from stdin into a temp file
//!   2. Applies the requested operation
//!   3. Writes the result to stdout
//!   4. Cleans up temp files

use anyhow::{anyhow, Result};
use std::io::{Read, Write};
use tempfile::NamedTempFile;
use ohpdf_core::pdf;

/// Stage names that can appear as the first CLI argument in pipeline mode.
const STAGES: &[&str] = &["rotate", "compress", "encrypt", "decrypt", "watermark"];

pub fn is_pipeline_stage(arg: &str) -> bool {
    // A pipeline stage is one of the known stage names AND stdin is not a tty
    // (i.e., something is actually being piped in).
    STAGES.contains(&arg) && !stdin_is_tty()
}

pub fn run_stage(args: &[String]) -> Result<()> {
    let stage = &args[0];
    let rest   = &args[1..];

    // Read stdin → temp file
    let mut input_bytes = Vec::new();
    std::io::stdin()
        .read_to_end(&mut input_bytes)
        .map_err(|e| anyhow!("Failed to read stdin: {}", e))?;

    if input_bytes.is_empty() {
        return Err(anyhow!("No input received on stdin"));
    }

    let mut input_tmp = NamedTempFile::new()
        .map_err(|e| anyhow!("Failed to create temp file: {}", e))?;
    input_tmp.write_all(&input_bytes)
        .map_err(|e| anyhow!("Failed to write temp file: {}", e))?;

    let output_tmp = NamedTempFile::new()
        .map_err(|e| anyhow!("Failed to create temp file: {}", e))?;

    let src = input_tmp.path().to_path_buf();
    let dst = output_tmp.path().to_path_buf();

    match stage.as_str() {
        "rotate" => {
            let degrees: i32 = rest.first()
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| anyhow!("rotate requires a degrees argument (90, 180, 270)"))?;
            // Empty pages vec means all pages
            pdf::rotate_pages(&src, &dst, &[], degrees)?;
        }
        "compress" => {
            let quality: Option<u8> = rest.first().and_then(|s| s.parse().ok());
            pdf::compress(&src, &dst, quality)?;
        }
        "encrypt" => {
            let password = rest.first()
                .ok_or_else(|| anyhow!("encrypt requires a password argument"))?;
            pdf::encrypt(&src, &dst, password)?;
        }
        "decrypt" => {
            let password = rest.first()
                .ok_or_else(|| anyhow!("decrypt requires a password argument"))?;
            pdf::decrypt(&src, &dst, password)?;
        }
        "watermark" => {
            let text = rest.first().map(|s| s.as_str()).unwrap_or("WATERMARK");
            let opts = pdf::WatermarkOptions {
                text,
                font_size: 60.0,
                opacity:   0.15,
                color:     (0.5, 0.5, 0.5),
            };
            pdf::add_watermark(&src, &dst, &opts)?;
        }
        s => return Err(anyhow!("Unknown pipeline stage: '{}'", s)),
    }

    // Write result to stdout
    let result = std::fs::read(&dst)
        .map_err(|e| anyhow!("Failed to read pipeline output: {}", e))?;
    std::io::stdout()
        .write_all(&result)
        .map_err(|e| anyhow!("Failed to write to stdout: {}", e))?;

    Ok(())
}

fn stdin_is_tty() -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        libc_isatty(std::io::stdin().as_raw_fd())
    }
    #[cfg(not(unix))]
    {
        false
    }
}

#[cfg(unix)]
fn libc_isatty(fd: std::os::unix::io::RawFd) -> bool {
    extern "C" {
        fn isatty(fd: i32) -> i32;
    }
    unsafe { isatty(fd) != 0 }
}
