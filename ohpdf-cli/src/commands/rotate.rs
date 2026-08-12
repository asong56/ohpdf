use super::{parse_args, report, ArgSpec};
use std::path::PathBuf;
use anyhow::{anyhow, Result};
use ohpdf_core::pdf;

const OUTPUT: ArgSpec = ArgSpec { names: &["-o", "--output"] };
const PAGES: ArgSpec = ArgSpec { names: &["--pages"] };
const DEGREES: ArgSpec = ArgSpec { names: &["--degrees"] };

pub fn run(args: &[String]) -> Result<()> {
    let (input, values) = parse_args(args, &[OUTPUT, PAGES, DEGREES])?;
    let [output, pages, degrees] = values.try_into().unwrap();

    let input = input.ok_or_else(|| anyhow!(
        "Usage: ohpdf-cli rotate <in.pdf> -o <out.pdf> [--pages 1,3-5] [--degrees 90|180|270]"
    ))?;
    let output = output.ok_or_else(|| anyhow!("Missing -o/--output <path>"))?;
    let pages = pages.map(|s| parse_page_list(&s)).transpose()?.unwrap_or_default();
    let degrees: i32 = match degrees {
        Some(s) => s.parse().map_err(|_| anyhow!("--degrees must be 90, 180, or 270"))?,
        None => 90,
    };

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
