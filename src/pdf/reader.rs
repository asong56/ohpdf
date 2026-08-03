use std::path::Path;
use anyhow::{Context, Result};
use base64::Engine;
use mupdf::{Colorspace, Document, ImageFormat, Matrix};

/// Render a single page as a `data:image/png;base64,...` URL, for the
/// Read & Annotate view.
///
/// This mirrors `pdf_page_thumbnails` in `thumbnails.rs` — same "`Pixmap`
/// doesn't expose an in-memory PNG encode in this crate version, only
/// `save_as(path, format)`" trade-off, so a short-lived temp file is used
/// as a go-between — just for one page at a time, and at whatever DPI the
/// reader's current zoom level calls for, rather than a fixed small
/// preview size for every page at once.
///
/// `page` is 1-indexed, matching every other page number this app exposes
/// over IPC (`GetPageThumbnails`, `RotatePages`, ...). MuPDF's own
/// `load_page` is 0-indexed; that conversion happens right here, at the
/// boundary, so nothing upstream has to think about it.
pub fn render_page(input: &Path, page: u32, dpi: u32) -> Result<String> {
    anyhow::ensure!(page >= 1, "Page numbers start at 1.");

    let doc = Document::open(input.to_str().context("Path contains invalid characters.")?)
        .with_context(|| format!("Failed to open file: {}", input.display()))?;

    let total = doc.page_count().context("Failed to get page count.")? as u32;
    anyhow::ensure!(
        page <= total,
        "Page {} is out of range (document has {} pages).",
        page,
        total
    );

    // MuPDF's native unit is 1/72 inch, same convention as `pdf_to_images`.
    let scale = dpi as f32 / 72.0;
    let matrix = Matrix::new_scale(scale, scale);

    let mupdf_page = doc
        .load_page((page - 1) as i32)
        .with_context(|| format!("Failed to load page {}.", page))?;

    let pixmap = mupdf_page
        .to_pixmap(&matrix, &Colorspace::device_rgb(), false, true)
        .with_context(|| format!("Failed to render page {}.", page))?;

    let tmp_dir = tempfile::tempdir().context("Failed to create temp directory.")?;
    let tmp_path = tmp_dir.path().join("page.png");

    pixmap
        .save_as(&tmp_path.to_string_lossy(), ImageFormat::PNG)
        .with_context(|| format!("Failed to render page {}.", page))?;

    let bytes = std::fs::read(&tmp_path)
        .with_context(|| format!("Failed to read rendered page {}.", page))?;

    // `data:image/png;base64,` is 22 bytes; base64 inflates the source by
    // ~4/3. Pre-sizing avoids the reallocate-and-copy growth `format!`
    // would otherwise do as the encoded data is appended.
    let mut data_url = String::with_capacity(22 + (bytes.len() * 4 / 3) + 4);
    data_url.push_str("data:image/png;base64,");
    base64::engine::general_purpose::STANDARD.encode_string(&bytes, &mut data_url);

    Ok(data_url)
}
