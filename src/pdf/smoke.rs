// 把这个文件存成 src/pdf/smoke.rs，
// 然后在 src/pdf/mod.rs 里加一行: #[cfg(test)] mod smoke;
//
// 运行: cargo test --package ohpdf smoke -- --nocapture
//
// 把 TEST_PDF 换成你自己上传到 Codespace 里的那个 myvocabs.pdf 的真实路径
// (在 Codespace 左侧文件树里把文件拖进去，或者用 VS Code 里 Codespace 网页版
// 自带的 "上传文件" 功能，放进仓库根目录下就行，比如 ./myvocabs.pdf)

#[cfg(test)]
mod tests {
    use super::super::*;
    use std::path::PathBuf;

    const TEST_PDF: &str = "/workspaces/ohpdf/myvocabs.pdf";

    fn out(name: &str) -> PathBuf {
        PathBuf::from(format!("/tmp/smoke_{}", name))
    }

    #[test]
    fn t_merge() {
        let input = PathBuf::from(TEST_PDF);
        let output = out("merged.pdf");
        let r = merge::merge(&[input.clone(), input], &output);
        println!("merge -> {:?}", r);
        assert!(r.is_ok(), "{:#?}", r);
        assert!(output.metadata().unwrap().len() > 0);
    }

    #[test]
    fn t_split() {
        let input = PathBuf::from(TEST_PDF);
        let n = pages::page_count(&input).unwrap();
        println!("page_count = {}", n);
        let mid = (n / 2).max(1);
        let outputs = vec![out("split_a.pdf"), out("split_b.pdf")];
        let r = split::split(&input, &[(1, mid), (mid + 1, n)], &outputs);
        println!("split -> {:?}", r);
        assert!(r.is_ok(), "{:#?}", r);
    }

    #[test]
    fn t_compress() {
        let input = PathBuf::from(TEST_PDF);
        let output = out("compressed.pdf");
        let r = compress::compress(&input, &output, Some(75));
        println!("compress -> {:?}", r);
        assert!(r.is_ok(), "{:#?}", r);
        let before = input.metadata().unwrap().len();
        let after = output.metadata().unwrap().len();
        println!("size: {} -> {}", before, after);
    }

    #[test]
    fn t_extract_pages() {
        let input = PathBuf::from(TEST_PDF);
        let output = out("extracted.pdf");
        let r = pages::extract_pages(&input, &output, &[1]);
        println!("extract_pages -> {:?}", r);
        assert!(r.is_ok(), "{:#?}", r);
    }

    #[test]
    fn t_reorder_pages() {
        let input = PathBuf::from(TEST_PDF);
        let n = pages::page_count(&input).unwrap();
        let output = out("reordered.pdf");
        // 简单地把顺序整个反过来
        let order: Vec<u32> = (1..=n).rev().collect();
        let r = pages::reorder_pages(&input, &output, &order);
        println!("reorder_pages -> {:?}", r);
        assert!(r.is_ok(), "{:#?}", r);
    }

    #[test]
    fn t_watermark() {
        let input = PathBuf::from(TEST_PDF);
        let output = out("watermarked.pdf");
        let opts = watermark::WatermarkOptions::default();
        let r = watermark::add_watermark(&input, &output, &opts);
        println!("add_watermark -> {:?}", r);
        assert!(r.is_ok(), "{:#?}", r);
        // 光返回Ok不能证明水印真的画出来了，还得肉眼看——
        // 见下面"看输出文件"那步。
    }
}