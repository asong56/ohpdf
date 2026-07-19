mod merge;
mod split;
mod compress;
mod encrypt;
mod pages;
mod images;
mod watermark;

pub use merge::merge;
pub use split::split;
pub use compress::compress;
pub use encrypt::{encrypt, decrypt};
pub use pages::{delete_pages, extract_pages, rotate_pages, reorder_pages, page_count};
pub use images::{pdf_to_images, images_to_pdf};
pub use watermark::{add_watermark, WatermarkOptions};
