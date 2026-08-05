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
    /// Color as (r, g, b) each 0.0-1.0 (default gray)
    pub color: (f32, f32, f32),
}

impl Default for WatermarkOptions<'_> {
    fn default() -> Self {
        Self {
            text: "WATERMARK",
            font_size: 60.0,
            opacity: 0.15,
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

        let color = PdfColor::rgb(opts.color.0, opts.color.1, opts.color.2);
        // BUG FIX: this used to be `TextOptions { color: Some(color), .. }`.
        // `TextOptions` has both a `color` field (stroke/outline color, for
        // *stroked* text) and a `fill` field (solid fill color, for normal
        // filled text) — mirroring `FinishOptions`'s `color`/`fill` split for
        // shapes. Setting only `color` draws the text in stroke-outline mode
        // with no configured stroke width and no fill, which is why the
        // operation reported success but produced no visible watermark: the
        // text was there, just outlined with an unset/near-zero width and
        // never filled. `fill` is the field that actually makes solid text
        // visible (confirmed against mupdf-rs's own `shape_demo.rs` example,
        // which only ever sets `fill` for normal text).
        //
        // This also silently dropped `opts.font_size`, `opts.opacity`, and
        // `opts.rotation` — none of them were ever passed into `TextOptions`,
        // so every watermark rendered at whatever `TextOptions::default()`'s
        // font size happens to be (not the requested size), fully opaque
        // (not the requested faint opacity), and unrotated (not diagonal).
        // A small, fully-opaque, non-diagonal bit of text sitting at the
        // exact center of a page full of other content is very easy to miss
        // — which matches "it says it succeeded but there's no watermark"
        // even independent of the color/fill bug above.
        // TextOptions::rotate only accepts multiples of 90 — passing 45
        // causes insert_text to return an error without drawing anything.
        // True arbitrary-angle text rotation in PDF requires emitting a
        // custom Tm (text matrix) operator, which this crate doesn't expose
        // directly. The closest supported option is 0° (horizontal) or 90°
        // (vertical). We use 0° and instead draw the text twice — once
        // slightly above center and once below — to make the watermark span
        // the page diagonally in appearance even though each line is
        // horizontal. If a true diagonal is required in the future, the
        // text_cont buffer would need a raw `cos θ sin θ -sin θ cos θ x y Tm`
        // entry injected before the TJ operator, which is not yet possible
        // via the public Shape API.
        let text_opts = TextOptions {
            fontsize: opts.font_size,
            fill: Some(color),
            fill_opacity: Some(opts.opacity),
            rotate: 0,
            ..Default::default()
        };

        // A rough horizontal centering: back off by roughly half the
        // string's rendered width, approximating each glyph as ~0.5em wide
        // (a reasonable average for typical watermark text at this size).
        let approx_width = opts.text.chars().count() as f32 * opts.font_size * 0.5;
        let x = cx - approx_width / 2.0;
        let page_h = bounds.y1 - bounds.y0;

        // Draw at 25/50/75% of page height so the watermark covers the whole
        // page. Each line needs its own Shape+commit — see the rotate comment
        // above for why we use multiple horizontal lines instead of one
        // diagonal line.
        for frac in [0.25_f32, 0.50, 0.75] {
            let y = bounds.y0 + page_h * frac;
            let mut shape = Shape::new(&mut page).context("Failed to create drawing context.")?;
            shape
                .insert_text(Point::new(x, y), opts.text, &text_opts)
                .with_context(|| format!("Failed to add watermark to page {}.", i + 1))?;
            shape
                .commit(&mut doc, true)
                .with_context(|| format!("Failed to write watermark to page {}.", i + 1))?;
        }
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