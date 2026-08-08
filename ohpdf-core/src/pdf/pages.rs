use std::collections::HashSet;
use std::path::Path;
use anyhow::{Context, Result};
use mupdf::pdf::PdfDocument;

use super::pdf_ops::{compact_write_options, copy_pages_to_new_doc};

/// Get total page count of a PDF.
pub fn page_count(input: &Path) -> Result<u32> {
    let doc = PdfDocument::open(input.to_str().context("Path contains invalid characters.")?)
        .with_context(|| format!("Failed to open file: {}", input.display()))?;
    let count = doc.page_count().context("Failed to get page count.")? as u32;
    Ok(count)
}

fn validate_pages(pages: &[u32], total: u32) -> Result<()> {
    for &p in pages {
        anyhow::ensure!(
            p >= 1 && p <= total,
            "Page {} is out of range (document has {} pages).",
            p,
            total
        );
    }
    Ok(())
}

/// Delete specific pages (1-indexed) from a PDF.
///
/// Deletes in-place on the already-open document via the real, documented
/// `PdfDocument::delete_page` (singular — there is no `delete_pages` in this
/// crate), working from the highest page number down so removing one page
/// doesn't shift the indices of pages still queued for deletion. Unlike
/// extract/reorder, deletion never needs to rebuild the document via
/// graft/copy, which keeps this the cheapest and fastest of these
/// operations.
pub fn delete_pages(input: &Path, output: &Path, pages: &[u32]) -> Result<()> {
    anyhow::ensure!(!pages.is_empty(), "At least one page must be specified.");

    let mut doc = PdfDocument::open(input.to_str().context("Path contains invalid characters.")?)
        .with_context(|| format!("Failed to open file: {}", input.display()))?;

    let total = doc.page_count().context("Failed to get page count.")? as u32;
    let to_delete: HashSet<u32> = pages.iter().copied().collect();
    validate_pages(pages, total)?;

    anyhow::ensure!(
        to_delete.len() < total as usize,
        "Cannot delete every page — the document must have at least one page left."
    );

    let mut sorted: Vec<u32> = to_delete.into_iter().collect();
    sorted.sort_unstable();
    for &page_no in sorted.iter().rev() {
        // `delete_page` is 0-indexed.
        doc.delete_page((page_no - 1) as i32)
            .with_context(|| format!("Failed to delete page {}.", page_no))?;
    }

    doc.save_with_options(
        output.to_str().context("Output path contains invalid characters.")?,
        compact_write_options(),
    )
    .context("Failed to save file.")?;

    log::info!(
        "Deleted {} pages from {} → {}",
        pages.len(),
        input.display(),
        output.display()
    );
    Ok(())
}

/// Extract specific pages (1-indexed) into a new PDF, in the given order
/// (duplicates allowed).
pub fn extract_pages(input: &Path, output: &Path, pages: &[u32]) -> Result<()> {
    anyhow::ensure!(!pages.is_empty(), "At least one page must be specified.");

    let src = PdfDocument::open(input.to_str().context("Path contains invalid characters.")?)
        .with_context(|| format!("Failed to open file: {}", input.display()))?;

    let total = src.page_count().context("Failed to get page count.")? as u32;
    validate_pages(pages, total)?;

    copy_pages_to_new_doc(&src, pages, output)?;

    log::info!(
        "Extracted {} pages from {} → {}",
        pages.len(),
        input.display(),
        output.display()
    );
    Ok(())
}

/// Rotate specific pages (1-indexed) by `degrees` (must be +/-90, 180 or 270).
///
/// Page rotation in a PDF is the integer `/Rotate` key on the page
/// dictionary, so this reads/writes it directly via `find_page` + `dict_put`
/// (both real, documented `PdfDocument`/`PdfObject` methods). This never
/// rebuilds the document, so it's the cheapest operation in this file.
pub fn rotate_pages(input: &Path, output: &Path, pages: &[u32], degrees: i32) -> Result<()> {
    anyhow::ensure!(
        matches!(degrees, 90 | 180 | 270 | -90 | -180 | -270),
        "Rotation must be 90, 180, or 270 degrees."
    );

    let delta = ((degrees % 360) + 360) % 360;

    let doc = PdfDocument::open(input.to_str().context("Path contains invalid characters.")?)
        .with_context(|| format!("Failed to open file: {}", input.display()))?;

    let total = doc.page_count().context("Failed to get page count.")? as u32;
    validate_pages(pages, total)?;
    let to_rotate: HashSet<u32> = pages.iter().copied().collect();

    for i in 0..total {
        let page_no = i + 1;
        if to_rotate.contains(&page_no) {
            let mut page_obj = doc
                .find_page(i as i32)
                .with_context(|| format!("Failed to read page {}.", page_no))?;

            let existing = page_obj
                .get_dict("Rotate")
                .ok()
                .flatten()
                .and_then(|o| o.as_int().ok())
                .unwrap_or(0);
            let new_rotation = ((existing + delta) % 360 + 360) % 360;

            let rotate_val = doc
                .new_int(new_rotation)
                .context("Failed to create PDF integer object.")?;
            page_obj
                .dict_put("Rotate", rotate_val)
                .with_context(|| format!("Failed to rotate page {}.", page_no))?;
        }
    }

    doc.save_with_options(
        output.to_str().context("Output path contains invalid characters.")?,
        compact_write_options(),
    )
    .context("Failed to save file.")?;

    log::info!(
        "Rotated {} pages by {}° in {} → {}",
        pages.len(),
        degrees,
        input.display(),
        output.display()
    );
    Ok(())
}

/// Reorder pages. `order` is a 1-indexed slice that maps new positions
/// to old page numbers. For example, `[3, 1, 2]` puts old page 3 first,
/// then 1, then 2.
///
/// Builds a fresh document and copies every page over via `copy_page` in the
/// requested order, sharing one `PdfGraftMap` across all of them so repeated
/// objects (fonts, images) aren't duplicated in the output.
pub fn reorder_pages(input: &Path, output: &Path, order: &[u32]) -> Result<()> {
    anyhow::ensure!(!order.is_empty(), "Page order cannot be empty.");

    let src = PdfDocument::open(input.to_str().context("Path contains invalid characters.")?)
        .with_context(|| format!("Failed to open file: {}", input.display()))?;

    let total = src.page_count().context("Failed to get page count.")? as u32;

    anyhow::ensure!(
        order.len() as u32 == total,
        "Order length ({}) does not match page count ({}).",
        order.len(),
        total
    );
    validate_pages(order, total)?;

    copy_pages_to_new_doc(&src, order, output)?;

    log::info!(
        "Reordered {} pages: {} → {}",
        total,
        input.display(),
        output.display()
    );
    Ok(())
}
