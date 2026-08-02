// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ohpdf_annotation;
mod ohpdf_commands;
mod ohpdf_pdf_reader;

// ---------------------------------------------------------------------
// 合并进你现有项目时：
//   1. 把上面三个 `mod ohpdf_*;` 声明加到你自己的 main.rs / lib.rs
//   2. 把下面 invoke_handler! 里的 5 个命令追加到你已有的列表里
//   3. 把 .plugin(tauri_plugin_dialog::init()) 这一行加到你的
//      Builder 链上（如果你还没用过 dialog 插件的话）
// 除此之外这个文件（main.rs 本身）不需要动你的项目。
// ---------------------------------------------------------------------
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            ohpdf_commands::ohpdf_open,
            ohpdf_commands::ohpdf_page_size,
            ohpdf_commands::ohpdf_render_page,
            ohpdf_commands::ohpdf_load_annotations,
            ohpdf_commands::ohpdf_save_annotations,
        ])
        .run(tauri::generate_context!())
        .expect("error while running ohpdf-read-patch-0801");
}
