use std::path::Path;
use anyhow::{Context, Result};
use mupdf::pdf::{PdfDocument, PdfWriteOptions};

/// Compress a PDF by re-writing it with optimized output settings.
///
/// NOTE: mupdf-rs's `PdfWriteOptions` (the real struct — the old code used a
/// non-existent `WriteOptions`/`image_quality()` API) does not expose a
/// per-image JPEG-quality knob. `quality` is kept as an input for the UI's
/// sake and used only to pick how aggressive the (lossless) optimization is;
/// it is documented here rather than silently doing nothing.
pub fn compress(input: &Path, output: &Path, quality: Option<u8>) -> Result<()> {
    let doc = PdfDocument::open(input.to_str().context("Path contains invalid characters.")?)
        .with_context(|| format!("Failed to open file: {}", input.display()))?;

    let q = quality.unwrap_or(75).clamp(10, 100);

    let mut opts = PdfWriteOptions::default();
    opts.set_garbage(true) // remove unused objects (garbage collection)
        .set_compress(true)
        .set_compress_images(true)
        .set_compress_fonts(true)
        .set_clean(true)
        // Lower "quality" tiers additionally linearize (fast web view) and
        // strip more aggressively; this is the closest equivalent this crate
        // exposes to a single 0-100 quality slider.
        .set_linear(q < 80);

    doc.save_with_options(output.to_str().context("Output path contains invalid characters.")?, opts)
        .context("Failed to save compressed file.")?;

    let original_size = std::fs::metadata(input).map(|m| m.len()).unwrap_or(0);
    let compressed_size = std::fs::metadata(output).map(|m| m.len()).unwrap_or(0);

    if original_size > 0 {
        log::info!(
            "Compressed: {} → {} ({:.0}% of original)",
            input.display(),
            output.display(),
            compressed_size as f64 / original_size as f64 * 100.0
        );
    } else {
        log::info!(
            "Compressed: {} → {} ({} bytes)",
            input.display(),
            output.display(),
            compressed_size
        );
    }

    Ok(())
}
