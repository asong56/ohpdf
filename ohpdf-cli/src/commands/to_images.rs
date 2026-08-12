use super::{parse_args, ArgSpec};
use std::path::PathBuf;
use anyhow::{anyhow, Result};
use ohpdf_core::pdf;

const OUTPUT: ArgSpec = ArgSpec { names: &["-o", "--output"] };
const DPI: ArgSpec = ArgSpec { names: &["--dpi"] };

pub fn run(args: &[String]) -> Result<()> {
    let (input, values) = parse_args(args, &[OUTPUT, DPI])?;
    let [output_dir, dpi] = values.try_into().unwrap();

    let input = input.ok_or_else(|| anyhow!("Usage: ohpdf-cli to-images <in.pdf> [-o <dir>] [--dpi N]"))?;
    let input_path = PathBuf::from(&input);
    if !input_path.exists() {
        return Err(anyhow!("File not found: {}", input));
    }

    let dpi = dpi.and_then(|s| s.parse().ok()).unwrap_or(150);
    let out_dir = output_dir.map(PathBuf::from);
    let outputs = pdf::pdf_to_images(&input_path, dpi, out_dir.as_deref())?;
    println!("Wrote {} image(s):", outputs.len());
    for p in outputs {
        println!("  {}", p.display());
    }
    Ok(())
}
