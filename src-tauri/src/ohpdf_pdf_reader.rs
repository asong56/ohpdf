//! PDF 打开 / 渲染 / 取页面尺寸。
//!
//! ⚠️ [VERIFY] 本文件调用了 `mupdf` crate（messense/mupdf-rs）的 API。
//! 由于当前沙箱既没有 Rust 工具链、也没有网络访问 crates.io，
//! 这些调用**没有被 `cargo build` 实际编译验证过**。
//!
//! 你已经有一个可编译的 rust+mupdf+tauri 项目，所以最快的验证方式是：
//! 把这个文件丢进去跑一次 `cargo check -p <你的crate>`，把报错贴回来，
//! 我可以照着编译器的实际签名逐条修正——这比我在这里凭记忆继续猜测更可靠。
//!
//! 已知最可能需要调整的两处，已在下面用 [VERIFY] 标出：
//!   1. `Pixmap` 编码为 PNG 字节的具体方法名（`to_png()` 是我的最佳猜测，
//!      不同版本可能是 `write_to()` / 需要经 `Buffer` 中转 / 或只暴露原始
//!      像素样本需要自己拼 PNG）。
//!   2. `Page::to_pixmap` 的确切参数类型（尤其是 alpha 参数是 `bool` 还是
//!      `f32`，我在下面按 `bool` 写）。
//!
//! 设计选择：**不保留任何跨调用的共享 Document 状态**——每个命令都用完整
//! 路径重新 `Document::open()`。这是刻意的取舍：MuPDF 的 `fz_context`
//! 本身不是线程安全的，mupdf-rs 通常靠线程本地 context 处理，如果把
//! `Document` 塞进 `tauri::State<Mutex<...>>` 并跨 `async` 边界持有，
//! 有很大概率遇到 Send/Sync 相关的编译错误或运行时问题——这类坑我在没有
//! 编译器在场的情况下没法可靠排查。无状态方式牺牲了「大文件多次翻页要
//! 重新解析」的一点性能，换来的是这份代码更大概率能直接跑起来。等你确认
//! 基础版本工作正常，再按你自己项目里已经验证过的并发模式加缓存层。

use mupdf::{Colorspace, Document, Matrix};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PdfInfo {
    pub page_count: i32,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PageSize {
    pub width: f32,
    pub height: f32,
}

pub fn open_and_get_info(path: &str) -> Result<PdfInfo, String> {
    let doc = Document::open(path).map_err(|e| format!("无法打开 PDF：{e:?}"))?;
    let page_count = doc
        .page_count()
        .map_err(|e| format!("无法读取页数：{e:?}"))?;
    // metadata() 在找不到该字段或文档没有该元数据时返回 Err，
    // 这里用 .ok() 把它当作「没有标题」处理，而不是当成错误往外抛。
    let title = doc.metadata("Title").ok();
    Ok(PdfInfo {
        page_count: page_count as i32,
        title,
    })
}

pub fn get_page_size(path: &str, page_index: i32) -> Result<PageSize, String> {
    let doc = Document::open(path).map_err(|e| format!("无法打开 PDF：{e:?}"))?;
    let page = doc
        .load_page(page_index)
        .map_err(|e| format!("无法加载第 {page_index} 页：{e:?}"))?;
    let bounds = page.bounds().map_err(|e| format!("无法获取页面尺寸：{e:?}"))?;
    Ok(PageSize {
        width: bounds.x1 - bounds.x0,
        height: bounds.y1 - bounds.y0,
    })
}

/// 把指定页渲染为 PNG 字节。
/// `dpi`：72 = 100%（PDF 原生单位就是 1/72 英寸），96 ≈ 133%，以此类推。
pub fn render_page_png(path: &str, page_index: i32, dpi: f32) -> Result<Vec<u8>, String> {
    let doc = Document::open(path).map_err(|e| format!("无法打开 PDF：{e:?}"))?;
    let page = doc
        .load_page(page_index)
        .map_err(|e| format!("无法加载第 {page_index} 页：{e:?}"))?;

    let zoom = dpi / 72.0;
    let matrix = Matrix::new_scale(zoom, zoom);

    // [VERIFY] 第三个参数（alpha）在部分版本里是 bool（是否带透明通道），
    // 部分版本可能是别的签名。PDF 页面本身不透明，这里传 false／不带 alpha。
    let pixmap = page
        .to_pixmap(&matrix, &Colorspace::device_rgb(), false, true)
        .map_err(|e| format!("渲染第 {page_index} 页失败：{e:?}"))?;

    // [VERIFY] 这是本文件里最不确定的一行。按已知的 mupdf-rs 用法示例，
    // Pixmap 有直接输出 PNG 字节的便捷方法；如果你的版本没有 `to_png()`，
    // 常见替代方案（任选其一，二选一即可编译通过）：
    //
    //   方案 A（如果 Pixmap 提供 write_to + 内存 Buffer）：
    //     let mut buf = mupdf::Buffer::new();
    //     pixmap.write_to(&mut buf, mupdf::ImageFormat::PNG)?;
    //     Ok(buf.as_slice().to_vec())
    //
    //   方案 B（如果只能拿到原始像素样本，自己用 `image` crate 编码）：
    //     let (w, h) = (pixmap.width(), pixmap.height());
    //     let samples = pixmap.samples(); // 原始 RGB 字节
    //     let img = image::RgbImage::from_raw(w, h, samples.to_vec()).unwrap();
    //     let mut out = Vec::new();
    //     img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)?;
    //     Ok(out)
    pixmap
        .to_png()
        .map_err(|e| format!("PNG 编码失败：{e:?}"))
}
