//! Batch PDF processing — apply an operation to every PDF in a directory.
//!
//! # Example
//! ```no_run
//! use ohpdf_core::batch::{BatchOp, run_batch};
//! use std::path::Path;
//!
//! run_batch(Path::new("./papers"), Path::new("./output"), BatchOp::Compress { quality: Some(75) }, |p| {
//!     eprintln!("done: {}", p.display());
//! }).unwrap();
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::pdf::{
    compress::compress,
    encrypt::{decrypt, encrypt},
    merge::merge,
    watermark::{add_watermark, WatermarkOptions},
};

/// Which operation to run on each PDF in the batch.
#[derive(Debug, Clone)]
pub enum BatchOp {
    Compress {
        quality: Option<u8>,
    },
    Encrypt {
        password: String,
    },
    Decrypt {
        password: String,
    },
    Watermark {
        text: String,
        font_size: Option<f32>,
        opacity: Option<f32>,
    },
    /// Merge all PDFs in the directory into one combined file.
    MergeAll {
        output_name: String,
    },
}

/// Outcome for a single file.
#[derive(Debug)]
pub struct FileResult {
    pub input: PathBuf,
    pub output: PathBuf,
    pub error: Option<String>,
}

/// Run `op` on every `*.pdf` inside `input_dir`, writing results to
/// `output_dir` (which is created if it does not exist).
///
/// `on_progress` is called after each file (successfully or not) so callers
/// can display a progress indicator.
///
/// Returns a summary of per-file results; partial failures are recorded
/// rather than aborting the whole batch.
pub fn run_batch<F>(
    input_dir: &Path,
    output_dir: &Path,
    op: BatchOp,
    on_progress: F,
) -> Result<Vec<FileResult>>
where
    F: Fn(&FileResult),
{
    fs::create_dir_all(output_dir)
        .with_context(|| format!("Cannot create output directory {}", output_dir.display()))?;

    let pdfs = collect_pdfs(input_dir)?;
    if pdfs.is_empty() {
        return Ok(vec![]);
    }

    // MergeAll is special — one output for all inputs.
    if let BatchOp::MergeAll { ref output_name } = op {
        let output = output_dir.join(output_name);
        let result = match merge(&pdfs, &output) {
            Ok(_) => FileResult {
                input: input_dir.to_path_buf(),
                output: output.clone(),
                error: None,
            },
            Err(e) => FileResult {
                input: input_dir.to_path_buf(),
                output: output.clone(),
                error: Some(e.to_string()),
            },
        };
        on_progress(&result);
        return Ok(vec![result]);
    }

    let mut results = Vec::with_capacity(pdfs.len());

    for src in &pdfs {
        let stem = src
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();
        let suffix = op_suffix(&op);
        let out = output_dir.join(format!("{}_{}.pdf", stem, suffix));

        let outcome = apply_op(src, &out, &op);
        let result = match outcome {
            Ok(_) => FileResult {
                input: src.clone(),
                output: out,
                error: None,
            },
            Err(e) => FileResult {
                input: src.clone(),
                output: out,
                error: Some(e.to_string()),
            },
        };
        on_progress(&result);
        results.push(result);
    }

    Ok(results)
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn collect_pdfs(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)
        .with_context(|| format!("Cannot read directory {}", dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .map(|e| e.to_ascii_lowercase() == "pdf")
                    .unwrap_or(false)
        })
        .collect();
    paths.sort();
    Ok(paths)
}

fn op_suffix(op: &BatchOp) -> &'static str {
    match op {
        BatchOp::Compress { .. } => "compressed",
        BatchOp::Encrypt { .. } => "encrypted",
        BatchOp::Decrypt { .. } => "decrypted",
        BatchOp::Watermark { .. } => "watermarked",
        BatchOp::MergeAll { .. } => "merged", // unused path
    }
}

fn apply_op(src: &Path, out: &Path, op: &BatchOp) -> Result<()> {
    match op {
        BatchOp::Compress { quality } => compress(src, out, *quality),
        BatchOp::Encrypt { password } => encrypt(src, out, password),
        BatchOp::Decrypt { password } => decrypt(src, out, password),
        BatchOp::Watermark {
            text,
            font_size,
            opacity,
        } => {
            let opts = WatermarkOptions {
                text,
                font_size: font_size.unwrap_or(60.0),
                opacity: opacity.unwrap_or(0.15),
                color: (0.5, 0.5, 0.5),
            };
            add_watermark(src, out, &opts)
        }
        BatchOp::MergeAll { .. } => unreachable!(),
    }
}
