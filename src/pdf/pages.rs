use std::path::Path;
use anyhow::{Context, Result};
use mupdf::pdf::PdfDocument;
use std::collections::HashSet;

/// Get total page count of a PDF.
pub fn page_count(input: &Path) -> Result<u32> {
    let doc = PdfDocument::open(input.to_str().context("Path contains invalid characters.")?)
        .with_context(|| format!("Failed to open file: {}", input.display()))?;
    let count = doc.page_count().context("Failed to get page count.")? as u32;
    Ok(count)
}

/// Delete specific pages (1-indexed) from a PDF.
pub fn delete_pages(input: &Path, output: &Path, pages: &[u32]) -> Result<()> {
    anyhow::ensure!(!pages.is_empty(), "At least one page must be specified.");

    let src = PdfDocument::open(input.to_str().context("Path contains invalid characters.")?)
        .with_context(|| format!("Failed to open file: {}", input.display()))?;

    let total = src.page_count().context("Failed to get page count.")? as u32;
    let to_delete: HashSet<u32> = pages.iter().copied().collect();

    for &p in &to_delete {
        anyhow::ensure!(
            p >= 1 && p <= total,
            "Page {} is out of range (document has {} pages).",
            p,
            total
        );
    }

    let mut out_doc = PdfDocument::new();

    for i in 1..=total {
        if !to_delete.contains(&i) {
            let page_obj = src
                .find_page((i - 1) as i32)
                .with_context(|| format!("Failed to read page {}.", i))?;
            let grafted = out_doc
                .graft_object(&page_obj)
                .with_context(|| format!("Failed to copy page {}.", i))?;
            out_doc
                .insert_page(-1, &grafted)
                .with_context(|| format!("Failed to insert page {}.", i))?;
        }
    }

    out_doc
        .save(output.to_str().context("Output path contains invalid characters.")?)
        .context("Failed to save file.")?;

    log::info!(
        "Deleted {} pages from {} → {}",
        pages.len(),
        input.display(),
        output.display()
    );
    Ok(())
}

/// Extract specific pages (1-indexed) into a new PDF.
pub fn extract_pages(input: &Path, output: &Path, pages: &[u32]) -> Result<()> {
    anyhow::ensure!(!pages.is_empty(), "At least one page must be specified.");

    let src = PdfDocument::open(input.to_str().context("Path contains invalid characters.")?)
        .with_context(|| format!("Failed to open file: {}", input.display()))?;

    let total = src.page_count().context("Failed to get page count.")? as u32;

    for &p in pages {
        anyhow::ensure!(
            p >= 1 && p <= total,
            "Page {} is out of range (document has {} pages).",
            p,
            total
        );
    }

    let mut out_doc = PdfDocument::new();

    for &p in pages {
        let page_obj = src
            .find_page((p - 1) as i32)
            .with_context(|| format!("Failed to read page {}.", p))?;
        let grafted = out_doc
            .graft_object(&page_obj)
            .with_context(|| format!("Failed to copy page {}.", p))?;
        out_doc
            .insert_page(-1, &grafted)
            .with_context(|| format!("Failed to insert page {}.", p))?;
    }

    out_doc
        .save(output.to_str().context("Output path contains invalid characters.")?)
        .context("Failed to save file.")?;

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
/// mupdf-rs has no `Document::rotate_page` method (it never existed). Page
/// rotation in a PDF is just the integer `/Rotate` key on the page
/// dictionary, so we read/write it directly via `find_page` + `dict_put`,
/// which are real, documented `PdfDocument`/`PdfObject` methods.
pub fn rotate_pages(input: &Path, output: &Path, pages: &[u32], degrees: i32) -> Result<()> {
    anyhow::ensure!(
        matches!(degrees, 90 | 180 | 270 | -90 | -180 | -270),
        "Rotation must be 90, 180, or 270 degrees."
    );

    let delta = ((degrees % 360) + 360) % 360;

    let doc = PdfDocument::open(input.to_str().context("Path contains invalid characters.")?)
        .with_context(|| format!("Failed to open file: {}", input.display()))?;

    let total = doc.page_count().context("Failed to get page count.")? as u32;

    for &p in pages {
        anyhow::ensure!(
            p >= 1 && p <= total,
            "Page {} is out of range (document has {} pages).",
            p,
            total
        );
    }
    let to_rotate: HashSet<u32> = pages.iter().copied().collect();

    for i in 0..total {
        let page_no = i + 1;
        if to_rotate.contains(&page_no) {
            let page_obj = doc
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

    doc.save(output.to_str().context("Output path contains invalid characters.")?)
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

    for &p in order {
        anyhow::ensure!(
            p >= 1 && p <= total,
            "Page {} is out of range (document has {} pages).",
            p,
            total
        );
    }

    let mut out_doc = PdfDocument::new();

    for &p in order {
        let page_obj = src
            .find_page((p - 1) as i32)
            .with_context(|| format!("Failed to read page {}.", p))?;
        let grafted = out_doc
            .graft_object(&page_obj)
            .with_context(|| format!("Failed to copy page {}.", p))?;
        out_doc
            .insert_page(-1, &grafted)
            .with_context(|| format!("Failed to insert page {}.", p))?;
    }

    out_doc
        .save(output.to_str().context("Output path contains invalid characters.")?)
        .context("Failed to save file.")?;

    log::info!(
        "Reordered {} pages: {} → {}",
        total,
        input.display(),
        output.display()
    );
    Ok(())
}
