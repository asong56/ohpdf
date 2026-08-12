//! OhPDF Core — shared PDF processing library used by both the desktop app
//! and the CLI.

pub mod pdf;
pub mod batch;

// `pdf::mod` already re-exports every public item flatly (`pdf::merge`,
// `pdf::compress`, ...); re-exporting the same list again here would just
// be a second copy of the same list to keep in sync. `pub use pdf::*`
// makes `ohpdf_core::merge(...)` etc. work directly from the crate root
// too, with one list instead of two.
pub use pdf::*;
