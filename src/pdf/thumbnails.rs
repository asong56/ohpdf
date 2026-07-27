use std::path::Path;
use anyhow::{Context, Result};
use base64::Engine;
use mupdf::{Colorspace, Document, ImageFormat, Matrix};

/// One rendered page thumbnail, ready to drop straight into an `<img src>`.
pub struct Thumbnail {
    /// 1-indexed page number.
    pub page: u32,
    /// A `data:image/png;base64,...` URL.
    pub data_url: String,
}

/// Render every page of a PDF as small preview thumbnails (base64 PNG data
/// URLs), for showing real page previews in the delete/reorder pickers
/// instead of plain numbered chips or filename rows.
///
/// This deliberately renders at a low, fixed resolution — these are just
/// previews for picking/reordering pages, not the full-quality export that
/// `pdf_to_images` produces, so keeping them small keeps things fast even
/// for large documents.
///
/// `Pixmap` doesn't expose a documented "encode straight to an in-memory
/// buffer" method in this crate version, only `save_as(path, format)`, so
/// each page is written to a short-lived temp file and immediately read back
/// in to base64-encode it; the temp file is removed right after.
pub fn pdf_page_thumbnails(input: &Path) -> Result<Vec<Thumbnail>> {
    const THUMB_DPI: u32 = 72; // small on purpose — these are previews, not exports
    let scale = THUMB_DPI as f32 / 72.0;
    let matrix = Matrix::new_scale(scale, scale);

    let doc = Document::open(input.to_str().context("Path contains invalid characters.")?)
        .with_context(|| format!("Failed to open file: {}", input.display()))?;

    let page_count = doc.page_count().context("Failed to get page count.")?;

    let tmp_dir = tempfile::tempdir().context("Failed to create temp directory.")?;

    let mut thumbs = Vec::with_capacity(page_count as usize);

    for i in 0..page_count {
        let page = doc
            .load_page(i)
            .with_context(|| format!("Failed to load page {}.", i + 1))?;

        let pixmap = page
            .to_pixmap(&matrix, &Colorspace::device_rgb(), false, true)
            .with_context(|| format!("Failed to render page {}.", i + 1))?;

        let tmp_path = tmp_dir.path().join(format!("thumb_{}.png", i));
        pixmap
            .save_as(&tmp_path.to_string_lossy(), ImageFormat::PNG)
            .with_context(|| format!("Failed to render thumbnail for page {}.", i + 1))?;

        let bytes = std::fs::read(&tmp_path)
            .with_context(|| format!("Failed to read rendered thumbnail for page {}.", i + 1))?;
        let _ = std::fs::remove_file(&tmp_path);

        let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
        thumbs.push(Thumbnail {
            page: i + 1,
            data_url: format!("data:image/png;base64,{}", encoded),
        });
    }

    log::info!(
        "Rendered {} page thumbnail(s) for {}",
        page_count,
        input.display()
    );
    Ok(thumbs)
}
