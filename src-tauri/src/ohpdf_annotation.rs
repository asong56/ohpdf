//! 标注存储：side-car JSON 方案
//!
//! 设计取舍：本补丁**不会**把标注写回 PDF 内部的注释对象（那需要调用
//! MuPDF 的 PdfAnnotation API，其具体方法签名随 mupdf-rs 版本变化，
//! 在当前无网络/无 Rust 工具链的沙箱中无法编译验证，贸然写出来风险较高）。
//!
//! 取而代之：标注保存在原 PDF 同目录下的 `<原文件名>.ohpdf-annot.json`
//! 里，格式为 { "页码(0起)": [ 标注... ] }。
//!
//! 好处：
//!   - 100% 只用 std::fs + serde，没有任何 MuPDF API 不确定性，编译风险最低
//!   - 不会修改/损坏用户的原始 PDF 文件
//!   - 前端已经按 PDF 点坐标存储/绘制，未来若要接入 MuPDF 的原生注释写入
//!     （让 Adobe Reader 等其他软件也能看到标注），坐标系统无需重新设计，
//!     只需要新增一个「导出为原生 PDF 注释」的命令，把这里的 JSON 转换成
//!     PdfAnnotation 调用即可。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

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

/// 单条标注。`type` 字段做内部 tag，其余字段按变体展开，
/// 因此 JSON 形如 { "type": "highlight", "rect": {...}, "color": "#f2c40c" }。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Annotation {
    Highlight { rect: Rect, color: String },
    Note { point: Point, text: String, color: String },
    Ink { points: Vec<Point>, color: String, width: f32 },
}

/// 页码(字符串形式，如 "0","1"...) -> 该页的标注列表。
/// 用 newtype 包一层 HashMap，但 serde 对 newtype 默认是透明的，
/// 序列化结果就是纯粹的 { "0": [...], "1": [...] }，前端不需要额外解包。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PageAnnotations(pub HashMap<String, Vec<Annotation>>);

fn sidecar_path(pdf_path: &str) -> String {
    format!("{}.ohpdf-annot.json", pdf_path)
}

pub fn load_sidecar(pdf_path: &str) -> Result<PageAnnotations, String> {
    let path = sidecar_path(pdf_path);
    if !Path::new(&path).exists() {
        return Ok(PageAnnotations::default());
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("读取标注文件失败：{e}"))?;
    if content.trim().is_empty() {
        return Ok(PageAnnotations::default());
    }
    serde_json::from_str(&content).map_err(|e| format!("标注文件格式错误：{e}"))
}

pub fn save_sidecar(pdf_path: &str, data: &PageAnnotations) -> Result<(), String> {
    let path = sidecar_path(pdf_path);
    let content = serde_json::to_string_pretty(data)
        .map_err(|e| format!("序列化标注失败：{e}"))?;
    fs::write(&path, content).map_err(|e| format!("写入标注文件失败：{e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_empty() {
        let tmp = std::env::temp_dir().join("ohpdf_test_empty.pdf");
        let tmp_str = tmp.to_string_lossy().to_string();
        // no sidecar yet -> should return default, not error
        let loaded = load_sidecar(&tmp_str).unwrap();
        assert!(loaded.0.is_empty());
    }

    #[test]
    fn roundtrip_with_data() {
        let tmp = std::env::temp_dir().join("ohpdf_test_data.pdf");
        let tmp_str = tmp.to_string_lossy().to_string();
        let sidecar = sidecar_path(&tmp_str);

        let mut map = HashMap::new();
        map.insert(
            "0".to_string(),
            vec![Annotation::Highlight {
                rect: Rect { x: 10.0, y: 20.0, w: 100.0, h: 12.0 },
                color: "#f2c40c".to_string(),
            }],
        );
        let data = PageAnnotations(map);
        save_sidecar(&tmp_str, &data).unwrap();

        let loaded = load_sidecar(&tmp_str).unwrap();
        assert_eq!(loaded.0.get("0").unwrap().len(), 1);

        let _ = fs::remove_file(sidecar);
    }
}
