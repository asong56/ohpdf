use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use mupdf::pdf::PdfDocument;
use mupdf::shape::Shape;
use mupdf::{Colorspace, Document, Image, ImageFormat, Matrix, Rect, Size};

/// Render every page of a PDF as PNG images.
///
/// Returns the list of output file paths (one per page).
/// Images are saved alongside the source PDF:
///   report.pdf → report_p1.png, report_p2.png, …
pub fn pdf_to_images(input: &Path, dpi: u32) -> Result<Vec<PathBuf>> {
    let src = Document::open(input.to_str().context("Path contains invalid characters.")?)
        .with_context(|| format!("Failed to open file: {}", input.display()))?;

    let page_count = src.page_count().context("Failed to get page count.")?;
    let scale = dpi as f32 / 72.0; // MuPDF internal unit is 72 dpi
    let matrix = Matrix::new_scale(scale, scale);

    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    let stem = input.file_stem().unwrap_or_default().to_string_lossy();

    let mut outputs = Vec::new();

    for i in 0..page_count {
        let page = src
            .load_page(i)
            .with_context(|| format!("Failed to load page {}.", i + 1))?;

        // `to_pixmap`'s real signature is
        // (matrix, colorspace, alpha: bool, show_extras: bool) — the old
        // code passed `0.0` (a float) where a `bool` (alpha) is expected.
        let pixmap = page
            .to_pixmap(&matrix, &Colorspace::device_rgb(), false, true)
            .with_context(|| format!("Failed to render page {}.", i + 1))?;

        let out_path = parent.join(format!("{}_p{}.png", stem, i + 1));

        // `save_as`'s second argument is the `ImageFormat` enum, not the
        // string literal `"png"` the old code passed.
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
///
/// Each image becomes one page, sized to fit the image at 72 dpi.
///
/// NOTE: `PdfDocument::add_page` and `insert_image_on_page` never existed in
/// mupdf-rs. The real building blocks are `PdfDocument::add_image` (loads an
/// `Image` in as an XObject and returns its `PdfObject`) plus the `Shape`
/// API to actually paint that image onto a page's content stream.
/// `Shape::insert_image` mirrors PyMuPDF's `Page.insert_image`; if your
/// installed mupdf-rs version names/signs this differently, this is the one
/// call in this module to double-check with `cargo doc -p mupdf --open`.
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

        // New page sized exactly to the image (points == pixels at 72 dpi).
        let mut page = doc
            .new_page(Size { width: w, height: h })
            .with_context(|| format!("Failed to create page for: {}", img_path.display()))?;

        let mut shape = Shape::new(&mut page).context("Failed to create drawing context.")?;
        shape
            .insert_image(&Rect::new(0.0, 0.0, w, h), &image)
            .with_context(|| format!("Failed to insert image: {}", img_path.display()))?;
        shape
            .commit(&mut doc, false)
            .with_context(|| format!("Failed to write image page for: {}", img_path.display()))?;
    }

    doc.save(output.to_str().context("Output path contains invalid characters.")?)
        .context("Failed to save PDF.")?;

    log::info!("Combined {} images → {}", images.len(), output.display());
    Ok(())
}
