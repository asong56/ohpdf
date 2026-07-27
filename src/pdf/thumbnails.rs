use std::path::Path;
use anyhow::{Context, Result};
use base64::Engine;
use mupdf::{Colorspace, Document, ImageFormat, Matrix};

pub struct Thumbnail {
    pub page: u32,
    pub data_url: String,
}

pub fn pdf_page_thumbnails(input: &Path) -> Result<Vec<Thumbnail>> {
    const THUMB_DPI: u32 = 72;
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
            page: (i + 1) as u32,
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
