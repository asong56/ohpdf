use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use mupdf::pdf::PdfDocument;

use super::pdf_ops::copy_pages_to_new_doc;

/// Split a PDF by page ranges.
///
/// `ranges` is a list of (start, end) pairs (1-indexed, inclusive).
/// Each range produces one output file in `outputs`.
pub fn split(input: &Path, ranges: &[(u32, u32)], outputs: &[PathBuf]) -> Result<()> {
    anyhow::ensure!(
        ranges.len() == outputs.len(),
        "The number of ranges and outputs must match."
    );

    let src = PdfDocument::open(input.to_str().context("Path contains invalid characters.")?)
        .with_context(|| format!("Failed to open file: {}", input.display()))?;

    let total = src.page_count().context("Failed to get page count.")? as u32;

    for ((start, end), out_path) in ranges.iter().zip(outputs.iter()) {
        anyhow::ensure!(
            *start >= 1 && *end >= *start && *end <= total,
            "Invalid page range: {}-{} (document has {} pages).",
            start,
            end,
            total
        );

        let page_numbers: Vec<u32> = (*start..=*end).collect();
        copy_pages_to_new_doc(&src, &page_numbers, out_path)
            .with_context(|| format!("Failed to save file: {}", out_path.display()))?;

        log::info!("Split pages {}-{} → {}", start, end, out_path.display());
    }

    Ok(())
}
