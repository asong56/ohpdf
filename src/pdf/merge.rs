use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use mupdf::pdf::PdfDocument;

/// Merge multiple PDF files into a single output PDF.
///
/// mupdf-rs has no `copy_page_from` method (it never existed in the published
/// crate). The real way to copy a page between two `PdfDocument`s is:
///   1. `find_page` on the source to get the page's object (a `PdfObject`)
///   2. `graft_object` on the destination, which deep-copies that object
///      (and everything it references) into the destination's own xref table
///   3. `insert_page` on the destination with the grafted object
pub fn merge(inputs: &[PathBuf], output: &Path) -> Result<()> {
    anyhow::ensure!(!inputs.is_empty(), "At least one input file is required.");

    let mut out_doc = PdfDocument::new();

    for path in inputs {
        let src = PdfDocument::open(path.to_str().context("Path contains invalid characters.")?)
            .with_context(|| format!("Failed to open file: {}", path.display()))?;

        let page_count = src.page_count().context("Failed to get page count.")?;

        for i in 0..page_count {
            let page_obj = src
                .find_page(i)
                .with_context(|| format!("Failed to read page {}.", i + 1))?;
            let grafted = out_doc
                .graft_object(&page_obj)
                .with_context(|| format!("Failed to copy page {}.", i + 1))?;
            // -1 == append at the end of the destination's page tree.
            out_doc
                .insert_page(-1, &grafted)
                .with_context(|| format!("Failed to insert page {}.", i + 1))?;
        }
    }

    out_doc
        .save(output.to_str().context("Output path contains invalid characters.")?)
        .context("Failed to save merged file.")?;

    log::info!("Merged {} files → {}", inputs.len(), output.display());
    Ok(())
}
