//! Tauri `#[tauri::command]` 入口。前端通过 `invoke('ohpdf_xxx', {...})` 调用。
//!
//! 全部 5 个命令都以 `ohpdf_` 前缀命名，是为了在合并进你现有项目时
//! 不与已有命令（比如你 toolkit 里其他 12 个功能已经用到的名字）冲突。
//! 如果你的项目里已经有别的 `render_page` / `open_pdf` 之类的命令，
//! 这里不会跟它们打架。

use crate::ohpdf_annotation::{self, PageAnnotations};
use crate::ohpdf_pdf_reader::{self, PageSize, PdfInfo};
use base64::{engine::general_purpose::STANDARD, Engine as _};

#[tauri::command]
pub fn ohpdf_open(path: String) -> Result<PdfInfo, String> {
    ohpdf_pdf_reader::open_and_get_info(&path)
}

#[tauri::command]
pub fn ohpdf_page_size(path: String, page: i32) -> Result<PageSize, String> {
    ohpdf_pdf_reader::get_page_size(&path, page)
}

#[tauri::command]
pub fn ohpdf_render_page(path: String, page: i32, dpi: f32) -> Result<String, String> {
    let png = ohpdf_pdf_reader::render_page_png(&path, page, dpi)?;
    Ok(format!("data:image/png;base64,{}", STANDARD.encode(png)))
}

#[tauri::command]
pub fn ohpdf_load_annotations(path: String) -> Result<PageAnnotations, String> {
    ohpdf_annotation::load_sidecar(&path)
}

#[tauri::command]
pub fn ohpdf_save_annotations(path: String, data: PageAnnotations) -> Result<(), String> {
    ohpdf_annotation::save_sidecar(&path, &data)
}
