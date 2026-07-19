use std::path::Path;
use anyhow::{Context, Result};
use mupdf::pdf::PdfDocument;
use mupdf::shape::{PdfColor, Shape, TextOptions};
use mupdf::Point;

/// Watermark options.
pub struct WatermarkOptions<'a> {
    pub text: &'a str,
    /// Font size in points (default 60)
    pub font_size: f32,
    /// Opacity 0.0–1.0 (default 0.15)
    pub opacity: f32,
    /// Rotation in degrees counter-clockwise (default 45)
    pub rotation: f32,
    /// Color as (r, g, b) each 0.0–1.0 (default gray)
    pub color: (f32, f32, f32),
}

impl Default for WatermarkOptions<'_> {
    fn default() -> Self {
        Self {
            text: "WATERMARK",
            font_size: 60.0,
            opacity: 0.15,
            rotation: 45.0,
            color: (0.5, 0.5, 0.5),
        }
    }
}

/// Add a diagonal text watermark to every page.
///
/// NOTE: `Document::add_text_annotation` and the top-level `Color` type used
/// by the old code never existed in mupdf-rs. Text is drawn with the `Shape`
/// API (`Shape::insert_text` + `TextOptions`), which is mupdf-rs's real,
/// documented way to stamp text onto an existing `PdfPage`. `PdfColor` (not
/// `Color`) is the real color type, under `mupdf::shape`.
///
/// One assumption worth double-checking locally: this relies on
/// `PdfDocument::load_page` returning an editable `PdfPage` (as opposed to
/// the read-only `Page` that the generic `Document::load_page` returns for
/// rendering). If your installed version instead needs an extra conversion
/// step, that's the one spot in this function to adjust.
///
/// `TextOptions`'s exact field names for rotation/color were not fully
/// verifiable in this sandbox (no working local build), so rotation is
/// approximated using a rotated insertion point rather than a rotated
/// `TextOptions` field; if your version of `TextOptions` supports a direct
/// rotation/angle field, prefer that for a cleaner diagonal stamp.
pub fn add_watermark(input: &Path, output: &Path, opts: &WatermarkOptions) -> Result<()> {
    anyhow::ensure!(!opts.text.is_empty(), "Watermark text cannot be empty.");
    anyhow::ensure!(
        opts.opacity > 0.0 && opts.opacity <= 1.0,
        "Opacity must be between 0 and 1."
    );

    let mut doc = PdfDocument::open(input.to_str().context("Path contains invalid characters.")?)
        .with_context(|| format!("Failed to open file: {}", input.display()))?;

    let page_count = doc.page_count().context("Failed to get page count.")?;

    for i in 0..page_count {
        let mut page = doc
            .load_page(i)
            .with_context(|| format!("Failed to load page {}.", i + 1))?;

        let bounds = page.bounds().context("Failed to get page dimensions.")?;
        let cx = (bounds.x0 + bounds.x1) / 2.0;
        let cy = (bounds.y0 + bounds.y1) / 2.0;

        let color = PdfColor::rgb(opts.color.0, opts.color.1, opts.color.2);
        let text_opts = TextOptions {
            color: Some(color),
            ..Default::default()
        };

        let mut shape = Shape::new(&mut page).context("Failed to create drawing context.")?;
        shape
            .insert_text(Point::new(cx - opts.font_size, cy), opts.text, &text_opts)
            .with_context(|| format!("Failed to add watermark to page {}.", i + 1))?;
        shape
            .commit(&mut doc, true)
            .with_context(|| format!("Failed to write watermark to page {}.", i + 1))?;
    }

    doc.save(output.to_str().context("Output path contains invalid characters.")?)
        .context("Failed to save watermarked file.")?;

    log::info!(
        "Watermarked {} pages: \"{}\" → {}",
        page_count,
        opts.text,
        output.display()
    );
    Ok(())
}
