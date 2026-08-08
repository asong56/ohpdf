//! OhPDF Core — shared PDF processing library used by both the desktop app
//! and the CLI.

pub mod pdf;
pub mod batch;

pub use pdf::annotations::{load_annotations, save_annotations, PageAnnotations};
pub use pdf::compress::compress;
pub use pdf::encrypt::{decrypt, encrypt};
pub use pdf::images::{images_to_pdf, pdf_to_images};
pub use pdf::merge::merge;
pub use pdf::pages::{delete_pages, extract_pages, page_count, reorder_pages, rotate_pages};
pub use pdf::reader::render_page;
pub use pdf::split::split;
pub use pdf::thumbnails::pdf_page_thumbnails;
pub use pdf::watermark::{add_watermark, WatermarkOptions};
