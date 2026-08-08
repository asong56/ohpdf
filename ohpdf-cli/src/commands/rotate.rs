use super::report;
use std::path::PathBuf;
use anyhow::{anyhow, Result};
use ohpdf_core::pdf;

pub fn run(args: &[String]) -> Result<()> {
    let mut input = None;
    let mut output = None;
    let mut pages: Vec<u32> = vec![];
    let mut degrees: i32 = 90;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => { output = args.get(i + 1).cloned(); i += 2; }
            "--pages" => {
                if let Some(s) = args.get(i + 1) {
                    pages = parse_page_list(s)?;
                    i += 2;
                } else {
                    return Err(anyhow!("Missing value for --pages"));
                }
            }
            "--degrees" => {
                degrees = args.get(i + 1)
                    .and_then(|s| s.parse().ok())
                    .ok_or_else(|| anyhow!("--degrees must be 90, 180, or 270"))?;
                i += 2;
            }
            arg if !arg.starts_with('-') => { input = Some(arg.to_string()); i += 1; }
            arg => return Err(anyhow!("Unknown option: {}", arg)),
        }
    }

    let input = input.ok_or_else(|| anyhow!(
        "Usage: ohpdf-cli rotate <in.pdf> -o <out.pdf> [--pages 1,3-5] [--degrees 90|180|270]"
    ))?;
    let output = output.ok_or_else(|| anyhow!("Missing -o/--output <path>"))?;

    let input_path = PathBuf::from(&input);
    if !input_path.exists() {
        return Err(anyhow!("File not found: {}", input));
    }

    pdf::rotate_pages(&input_path, &PathBuf::from(&output), &pages, degrees)?;
    report(&input_path, &PathBuf::from(&output));
    Ok(())
}

/// Parse "1,3-5,7" → [1, 3, 4, 5, 7]. Empty → all pages (empty vec).
fn parse_page_list(s: &str) -> Result<Vec<u32>> {
    if s.is_empty() { return Ok(vec![]); }
    let mut pages = vec![];
    for part in s.split(',') {
        let part = part.trim();
        if let Some((a, b)) = part.split_once('-') {
            let start: u32 = a.trim().parse().map_err(|_| anyhow!("Invalid page number: {}", a))?;
            let end:   u32 = b.trim().parse().map_err(|_| anyhow!("Invalid page number: {}", b))?;
            if start > end { return Err(anyhow!("Page range {}-{} is backwards", start, end)); }
            pages.extend(start..=end);
        } else {
            let n: u32 = part.parse().map_err(|_| anyhow!("Invalid page number: {}", part))?;
            pages.push(n);
        }
    }
    pages.sort_unstable();
    pages.dedup();
    Ok(pages)
}
