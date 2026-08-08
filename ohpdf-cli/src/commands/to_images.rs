use std::path::PathBuf;
use anyhow::{anyhow, Result};
use ohpdf_core::pdf;

pub fn run(args: &[String]) -> Result<()> {
    let mut input = None;
    let mut output_dir = None;
    let mut dpi = 150_u32;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => { output_dir = args.get(i + 1).cloned(); i += 2; }
            "--dpi" => { dpi = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(150); i += 2; }
            arg if !arg.starts_with('-') => { input = Some(arg.to_string()); i += 1; }
            arg => return Err(anyhow!("Unknown option: {}", arg)),
        }
    }

    let input = input.ok_or_else(|| anyhow!("Usage: ohpdf-cli to-images <in.pdf> [-o <dir>] [--dpi N]"))?;
    let input_path = PathBuf::from(&input);
    if !input_path.exists() {
        return Err(anyhow!("File not found: {}", input));
    }

    let out_dir = output_dir.map(PathBuf::from);
    let outputs = pdf::pdf_to_images(&input_path, dpi, out_dir.as_deref())?;
    println!("Wrote {} image(s):", outputs.len());
    for p in outputs {
        println!("  {}", p.display());
    }
    Ok(())
}
