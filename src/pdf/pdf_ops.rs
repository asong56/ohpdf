//! Shared low-level helpers used by merge/split/pages: the single, correct
//! way to copy a page between two `PdfDocument`s, and the write options used
//! whenever we rebuild a document from scratch.
//!
//! `copy_page` mirrors MuPDF's own documented "copyPage" recipe (see MuPDF.js's
//! Advanced cookbook page, and `mutool merge`'s JS reimplementation): rather
//! than grafting the *entire* source page object — which carries a
//! `/Parent` back-reference into the *source* document's page tree, and
//! grafting that whole-cloth into a different document's tree is what caused
//! "Failed to insert page 1" errors in earlier attempts at this — we build a
//! brand new, clean page dictionary in the destination and graft over just
//! the handful of keys that matter (MediaBox, Rotate, Resources, Contents),
//! then register and insert that new dictionary.

use std::path::Path;
use anyhow::{Context, Result};
use mupdf::pdf::{PdfDocument, PdfGraftMap, PdfObject, PdfWriteOptions};

/// Copies a single page (0-indexed `src_page_no`) from `src` into `dst`,
/// inserting it at `insert_at` (`-1` to append at the end).
///
/// `graft_map` should be created once per source/destination document pair
/// (`dst.new_graft_map()`) and reused across every page copied between them,
/// so repeated/shared objects (fonts, images embedded on multiple pages) are
/// deduplicated in the output instead of being copied again per page — this
/// keeps output files smaller and copying faster.
pub fn copy_page(
    dst: &mut PdfDocument,
    src: &PdfDocument,
    src_page_no: i32,
    insert_at: i32,
    graft_map: &mut PdfGraftMap,
) -> Result<()> {
    let src_page = src
        .find_page(src_page_no)
        .with_context(|| format!("Failed to read source page {}.", src_page_no + 1))?;

    let mut new_page = dst
        .new_dict()
        .context("Failed to create a new page dictionary.")?;

    new_page
        .dict_put(
            "Type",
            dst.new_name("Page").context("Failed to create /Page name.")?,
        )
        .context("Failed to set /Type on new page.")?;

    for key in ["MediaBox", "Rotate", "Resources", "Contents"] {
        if let Some(value) = src_page
            .get_dict(key)
            .with_context(|| format!("Failed to read /{} from source page.", key))?
        {
            let grafted: PdfObject = graft_map
                .graft_object(&value)
                .with_context(|| format!("Failed to copy /{} into the new document.", key))?;
            new_page
                .dict_put(key, grafted)
                .with_context(|| format!("Failed to set /{} on new page.", key))?;
        }
    }

    let page_ref = dst
        .add_object(&new_page)
        .context("Failed to register the new page object.")?;

    dst.insert_page(insert_at, &page_ref)
        .with_context(|| format!("Failed to insert page {} into output.", src_page_no + 1))?;

    Ok(())
}

/// Write options used whenever a rewritten PDF is saved: garbage-collect
/// unused objects and compress streams. This keeps output files small
/// (particularly after deleting/reordering pages, which otherwise leaves
/// orphaned objects behind) for effectively no extra cost.
pub fn compact_write_options() -> PdfWriteOptions {
    let mut opts = PdfWriteOptions::default();
    opts.set_garbage(true).set_compress(true);
    opts
}

/// Builds a brand new `PdfDocument` containing exactly the pages of `src`
/// listed in `page_numbers` (1-indexed), in that order (duplicates allowed),
/// and saves it to `output`. Shared by `extract_pages`/`reorder_pages` (in
/// pages.rs) and `split` (which expands each requested range into a
/// contiguous list of page numbers first) — all three are really the same
/// operation (build a new document from a chosen list of source pages), just
/// with different rules for what that list of page numbers is allowed to be.
pub fn copy_pages_to_new_doc(src: &PdfDocument, page_numbers: &[u32], output: &Path) -> Result<()> {
    let mut out_doc = PdfDocument::new();
    let mut graft_map = out_doc
        .new_graft_map()
        .context("Failed to create a graft map for copying pages.")?;

    for &p in page_numbers {
        copy_page(&mut out_doc, src, (p - 1) as i32, -1, &mut graft_map)
            .with_context(|| format!("Failed to copy page {}.", p))?;
    }

    out_doc
        .save_with_options(
            output.to_str().context("Output path contains invalid characters.")?,
            compact_write_options(),
        )
        .context("Failed to save file.")?;

    Ok(())
}
