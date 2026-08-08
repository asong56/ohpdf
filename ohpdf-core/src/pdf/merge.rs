use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use mupdf::pdf::PdfDocument;

use super::pdf_ops::{compact_write_options, copy_page};

/// Merge multiple PDF files into a single output PDF, in the order given.
pub fn merge(inputs: &[PathBuf], output: &Path) -> Result<()> {
    anyhow::ensure!(!inputs.is_empty(), "At least one input file is required.");

    let mut out_doc = PdfDocument::new();

    for path in inputs {
        let src = PdfDocument::open(path.to_str().context("Path contains invalid characters.")?)
            .with_context(|| format!("Failed to open file: {}", path.display()))?;

        let page_count = src
            .page_count()
            .with_context(|| format!("Failed to get page count for: {}", path.display()))?;

        // A fresh graft map per source document: objects shared *within*
        // one source file (e.g. a font used on every page) get deduplicated
        // in the output, without needing to track mappings across unrelated
        // source files.
        let mut graft_map = out_doc
            .new_graft_map()
            .context("Failed to create a graft map for copying pages.")?;

        for i in 0..page_count {
            copy_page(&mut out_doc, &src, i, -1, &mut graft_map)
                .with_context(|| format!("Failed while copying pages from: {}", path.display()))?;
        }
    }

    out_doc
        .save_with_options(
            output.to_str().context("Output path contains invalid characters.")?,
            compact_write_options(),
        )
        .context("Failed to save merged file.")?;

    log::info!("Merged {} files → {}", inputs.len(), output.display());
    Ok(())
}
