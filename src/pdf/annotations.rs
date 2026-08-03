//! Sidecar-file storage for reader annotations (highlights, sticky notes,
//! freehand ink strokes) made in the "Read & Annotate" tool.
//!
//! Deliberately does **not** write annotations into the PDF's own
//! `/Annots` structures. Every other operation in this module follows the
//! same rule: the source file is never modified in place, and the result
//! of an operation is always a separate file next to it (`report.pdf` →
//! `report_merged.pdf`, `report_compressed.pdf`, ...). Annotations follow
//! that same rule — they live in a plain JSON file next to the PDF
//! (`<name>.pdf.ohpdf-annot.json`), so opening, reading, and marking up a
//! document can never corrupt or silently rewrite the original.
//!
//! If native (Adobe-visible) PDF annotations are ever wanted instead, the
//! page-point coordinate system already used here maps directly onto
//! MuPDF's own `PdfAnnotation` API — this file would just gain an export
//! step; the data model would not need to change.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

/// One annotation. `type` is used as the serde tag, so on the wire this
/// looks like `{"type":"highlight","rect":{...},"color":"#f2c40c"}` —
/// matching, field for field, what the reader draws on its overlay canvas.
/// Coordinates are in PDF points (1/72 inch), independent of the zoom
/// level the page happened to be rendered at when the mark was made.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Annotation {
    Highlight { rect: Rect, color: String },
    Note { point: Point, text: String, color: String },
    Ink { points: Vec<Point>, color: String, width: f32 },
}

/// Page number (as a string key, 1-indexed to match every other page
/// number this app exposes over IPC — see `GetPageThumbnails`,
/// `RotatePages`, etc.) -> that page's annotations, in the order they were
/// drawn.
///
/// A newtype around the map rather than a bare type alias so it can carry
/// its own doc comment; serde treats a single-field tuple struct as
/// transparent, so this still (de)serializes as a plain
/// `{"1": [...], "2": [...]}` object with no extra wrapping — the JS side
/// never has to know or care that it isn't a raw `HashMap`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PageAnnotations(pub HashMap<String, Vec<Annotation>>);

fn sidecar_path(pdf_path: &Path) -> PathBuf {
    let mut name = pdf_path.as_os_str().to_os_string();
    name.push(".ohpdf-annot.json");
    PathBuf::from(name)
}

/// Load previously-saved annotations for `pdf_path`. A missing sidecar
/// file is not an error — it just means the document has no annotations
/// yet, so this returns an empty `PageAnnotations` instead.
pub fn load_annotations(pdf_path: &Path) -> Result<PageAnnotations> {
    let path = sidecar_path(pdf_path);
    if !path.exists() {
        return Ok(PageAnnotations::default());
    }

    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read annotation file: {}", path.display()))?;
    if content.trim().is_empty() {
        return Ok(PageAnnotations::default());
    }

    serde_json::from_str(&content)
        .with_context(|| format!("Annotation file is not valid JSON: {}", path.display()))
}

/// Save `data` as the sidecar file for `pdf_path`, overwriting whatever was
/// there before. `pdf_path` itself is never opened or touched.
pub fn save_annotations(pdf_path: &Path, data: &PageAnnotations) -> Result<()> {
    let path = sidecar_path(pdf_path);
    let content =
        serde_json::to_string_pretty(data).context("Failed to serialize annotations.")?;
    std::fs::write(&path, content)
        .with_context(|| format!("Failed to write annotation file: {}", path.display()))?;

    log::info!(
        "Saved annotations for {} → {}",
        pdf_path.display(),
        path.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_sidecar_returns_default() {
        let tmp = std::env::temp_dir().join("ohpdf_annot_test_missing.pdf");
        let loaded = load_annotations(&tmp).unwrap();
        assert!(loaded.0.is_empty());
    }

    #[test]
    fn roundtrip_with_data() {
        let tmp = std::env::temp_dir().join("ohpdf_annot_test_roundtrip.pdf");
        let mut pages = HashMap::new();
        pages.insert(
            "1".to_string(),
            vec![Annotation::Highlight {
                rect: Rect {
                    x: 10.0,
                    y: 20.0,
                    w: 100.0,
                    h: 12.0,
                },
                color: "#f2c40c".to_string(),
            }],
        );
        let data = PageAnnotations(pages);

        save_annotations(&tmp, &data).unwrap();
        let loaded = load_annotations(&tmp).unwrap();
        assert_eq!(loaded.0.get("1").unwrap().len(), 1);

        let _ = std::fs::remove_file(sidecar_path(&tmp));
    }
}
