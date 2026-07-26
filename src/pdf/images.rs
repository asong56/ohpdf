use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use mupdf::pdf::PdfDocument;
use mupdf::{Colorspace, Document, Image, ImageFormat, Matrix, Rect, Size};

pub fn pdf_to_images(input: &Path, dpi: u32) -> Result<Vec<PathBuf>> {
    let src = Document::open(input.to_str().context("Path contains invalid characters.")?)
        .with_context(|| format!("Failed to open file: {}", input.display()))?;

    let page_count = src.page_count().context("Failed to get page count.")?;
    let scale = dpi as f32 / 72.0;
    let matrix = Matrix::new_scale(scale, scale);

    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    let stem = input.file_stem().unwrap_or_default().to_string_lossy();

    let mut outputs = Vec::new();

    for i in 0..page_count {
        let page = src
            .load_page(i)
            .with_context(|| format!("Failed to load page {}.", i + 1))?;

        let pixmap = page
            .to_pixmap(&matrix, &Colorspace::device_rgb(), false, true)
            .with_context(|| format!("Failed to render page {}.", i + 1))?;

        let out_path = parent.join(format!("{}_p{}.png", stem, i + 1));

        pixmap
            .save_as(&out_path.to_string_lossy(), ImageFormat::PNG)
            .with_context(|| format!("Failed to save image: {}", out_path.display()))?;

        outputs.push(out_path);
    }

    log::info!(
        "Exported {} pages as PNG from {}",
        page_count,
        input.display()
    );
    Ok(outputs)
}

/// Combine image files (PNG / JPG / JPEG / WEBP / BMP) into a single PDF.
/// Each image becomes one page, sized to fit the image at 72 dpi.
pub fn images_to_pdf(images: &[PathBuf], output: &Path) -> Result<()> {
    anyhow::ensure!(!images.is_empty(), "At least one image is required.");

    let mut doc = PdfDocument::new();

    for img_path in images {
        let ext = img_path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();

        anyhow::ensure!(
            matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp" | "bmp"),
            "Unsupported image format: {}",
            img_path.display()
        );

        let image = Image::from_file(img_path.to_str().context("Image path contains invalid characters.")?)
            .with_context(|| format!("Failed to read image: {}", img_path.display()))?;

        let w = image.width() as f32;
        let h = image.height() as f32;

        let mut page = doc
            .new_page(Size { width: w, height: h })
            .with_context(|| format!("Failed to create page for: {}", img_path.display()))?;

        page.insert_image(&Rect::new(0.0, 0.0, w, h), &image)
            .with_context(|| format!("Failed to insert image: {}", img_path.display()))?;
    }

    doc.save(output.to_str().context("Output path contains invalid characters.")?)
        .context("Failed to save PDF.")?;

    log::info!("Combined {} images → {}", images.len(), output.display());
    Ok(())
}
