use std::path::Path;
use anyhow::{Context, Result};
use mupdf::Document;

use super::pdf_ops::{path_str, render_page_to_data_url};

/// Render a single page as a `data:image/png;base64,...` URL, for the
/// Read & Annotate view.
///
/// The actual rasterize-and-encode work is shared with `pdf_page_thumbnails`
/// (in `thumbnails.rs`) via `render_page_to_data_url` — this function's own
/// job is just resolving which single page to render, at whatever DPI the
/// reader's current zoom level calls for, rather than every page at a fixed
/// small preview size.
///
/// `page` is 1-indexed, matching every other page number this app exposes
/// over IPC (`GetPageThumbnails`, `RotatePages`, ...). MuPDF's own
/// `load_page` is 0-indexed; that conversion happens right here, at the
/// boundary, so nothing upstream has to think about it.
pub fn render_page(input: &Path, page: u32, dpi: u32) -> Result<String> {
    anyhow::ensure!(page >= 1, "Page numbers start at 1.");

    let doc = Document::open(path_str(input)?)
        .with_context(|| format!("Failed to open file: {}", input.display()))?;

    let total = doc.page_count().context("Failed to get page count.")? as u32;
    anyhow::ensure!(
        page <= total,
        "Page {} is out of range (document has {} pages).",
        page,
        total
    );

    let mupdf_page = doc
        .load_page((page - 1) as i32)
        .with_context(|| format!("Failed to load page {}.", page))?;

    let tmp_dir = tempfile::tempdir().context("Failed to create temp directory.")?;
    let tmp_path = tmp_dir.path().join("page.png");

    render_page_to_data_url(&mupdf_page, dpi, &tmp_path)
        .with_context(|| format!("Failed to render page {}.", page))
}
