use std::path::PathBuf;
use anyhow::{anyhow, Result};
use ohpdf_core::pdf;

pub fn run(args: &[String]) -> Result<()> {
    let input = args.first().ok_or_else(|| anyhow!("Usage: ohpdf-cli info <in.pdf>"))?;
    let input_path = PathBuf::from(input);
    if !input_path.exists() {
        return Err(anyhow!("File not found: {}", input));
    }

    let pages = pdf::page_count(&input_path)?;
    println!("File:  {}", input_path.display());
    println!("Pages: {}", pages);
    Ok(())
}
