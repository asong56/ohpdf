pub mod annotations;
pub mod compress;
pub mod encrypt;
pub mod images;
pub mod merge;
pub mod pages;
pub mod pdf_ops;
pub mod reader;
pub mod split;
pub mod thumbnails;
pub mod watermark;

// Flat re-exports so callers can write `pdf::merge(...)`, `pdf::compress(...)`,
// etc. instead of reaching into each submodule — mirrors how main's
// (private, binary-crate) pdf module was organized.
pub use annotations::{load_annotations, save_annotations, PageAnnotations};
pub use compress::compress;
pub use encrypt::{decrypt, encrypt};
pub use images::{images_to_pdf, pdf_to_images};
pub use merge::merge;
pub use pages::{delete_pages, extract_pages, page_count, reorder_pages, rotate_pages};
pub use reader::render_page;
pub use split::split;
pub use thumbnails::pdf_page_thumbnails;
pub use watermark::{add_watermark, WatermarkOptions};
