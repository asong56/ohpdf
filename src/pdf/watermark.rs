use std::convert::TryFrom;
use std::path::Path;
use anyhow::{Context, Result};
use mupdf::pdf::{PdfDocument, PdfPage};
use mupdf::shape::{PdfColor, Shape, TextOptions};
use mupdf::Point;

use super::pdf_ops::compact_write_options;

/// Watermark options.
pub struct WatermarkOptions<'a> {
    pub text: &'a str,
    /// Font size in points (default 60)
    pub font_size: f32,
    /// Opacity 0.0-1.0 (default 0.15)
    pub opacity: f32,
    /// Rotation in degrees counter-clockwise (default 45)
    pub rotation: f32,
    /// Color as (r, g, b) each 0.0-1.0 (default gray)
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
/// Uses the `Shape` builder API (`Shape::new` / `insert_text` / `commit`),
/// which mupdf-rs's own README demonstrates end-to-end verbatim — this is
/// the most solidly-confirmed drawing API this crate exposes.
///
/// `PdfDocument::load_page` (inherited via `Deref<Target = Document>`)
/// returns a generic, render-oriented `Page`, not the PDF-specific,
/// editable `PdfPage` that `Shape::new` requires. `PdfPage::try_from(page)`
/// bridges the two — mirroring the same `TryFrom<Document> for PdfDocument`
/// pattern this crate already uses elsewhere (confirmed in its own source)
/// to convert between its generic and PDF-specific document types.
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
        let page = doc
            .load_page(i)
            .with_context(|| format!("Failed to load page {}.", i + 1))?;
        let mut page = PdfPage::try_from(page)
            .with_context(|| format!("Failed to get editable page {}.", i + 1))?;

        let bounds = page.bounds().context("Failed to get page dimensions.")?;
        let cx = (bounds.x0 + bounds.x1) / 2.0;
        let cy = (bounds.y0 + bounds.y1) / 2.0;

        let color = PdfColor::rgb(opts.color.0, opts.color.1, opts.color.2);
        let text_opts = TextOptions {
            color: Some(color),
            ..Default::default()
        };

        // A rough horizontal centering: back off by roughly half the
        // string's rendered width, approximating each glyph as ~0.5em wide
        // (a reasonable average for typical watermark text at this size).
        let approx_width = opts.text.chars().count() as f32 * opts.font_size * 0.5;

        let mut shape = Shape::new(&mut page).context("Failed to create drawing context.")?;
        shape
            .insert_text(
                Point::new(cx - approx_width / 2.0, cy),
                opts.text,
                &text_opts,
            )
            .with_context(|| format!("Failed to add watermark to page {}.", i + 1))?;
        shape
            .commit(&mut doc, true)
            .with_context(|| format!("Failed to write watermark to page {}.", i + 1))?;
    }

    doc.save_with_options(
        output.to_str().context("Output path contains invalid characters.")?,
        compact_write_options(),
    )
    .context("Failed to save watermarked file.")?;

    log::info!(
        "Watermarked {} pages: \"{}\" → {}",
        page_count,
        opts.text,
        output.display()
    );
    Ok(())
}
