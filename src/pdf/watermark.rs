use std::convert::TryFrom;
use std::path::Path;
use anyhow::{Context, Result};
use mupdf::pdf::{PdfDocument, PdfPage};
use mupdf::shape::{PdfColor, Shape, TextOptions};
use mupdf::Point;

pub struct WatermarkOptions<'a> {
    pub text: &'a str,
    pub font_size: f32,
    pub opacity: f32,
    pub rotation: f32,
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
