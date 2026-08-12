use std::path::Path;
use anyhow::{Context, Result};
use mupdf::Document;

use super::pdf_ops::{path_str, render_page_to_data_url};

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

    let doc = Document::open(path_str(input)?)
        .with_context(|| format!("Failed to open file: {}", input.display()))?;

    let page_count = doc.page_count().context("Failed to get page count.")?;

    let tmp_dir = tempfile::tempdir().context("Failed to create temp directory.")?;
    // Pages are rendered one at a time (not in parallel — `Document` isn't
    // confirmed `Send`/`Sync` in this crate version, so we don't risk
    // sharing it across threads), so a single reused path is enough; no
    // need for a uniquely-numbered filename per page.
    let tmp_path = tmp_dir.path().join("thumb.png");

    let mut thumbs = Vec::with_capacity(page_count as usize);

    for i in 0..page_count {
        let page = doc
            .load_page(i)
            .with_context(|| format!("Failed to load page {}.", i + 1))?;

        let data_url = render_page_to_data_url(&page, THUMB_DPI, &tmp_path)
            .with_context(|| format!("Failed to render thumbnail for page {}.", i + 1))?;

        thumbs.push(Thumbnail {
            page: (i + 1) as u32,
            data_url,
        });
    }
    // `tmp_dir` (and the single `thumb.png` inside it) is removed here when
    // it drops, rather than per-page — one filesystem cleanup instead of N.

    log::info!(
        "Rendered {} page thumbnail(s) for {}",
        page_count,
        input.display()
    );
    Ok(thumbs)
}
