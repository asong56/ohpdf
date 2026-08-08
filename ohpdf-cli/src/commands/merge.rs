use std::path::PathBuf;
use anyhow::{anyhow, Result};
use ohpdf_core::pdf;
use super::take_flag_value;

pub fn run(args: &[String]) -> Result<()> {
    if args.is_empty() {
        return Err(anyhow!("Usage: ohpdf-cli merge <in1.pdf> <in2.pdf> ... -o <out.pdf>"));
    }

    let (output, inputs) = take_flag_value(args, &["-o", "--output"]);
    let output = output.ok_or_else(|| anyhow!("Missing -o/--output <path>"))?;

    if inputs.is_empty() {
        return Err(anyhow!("No input PDFs given."));
    }

    let input_paths: Vec<PathBuf> = inputs.iter().map(PathBuf::from).collect();
    for p in &input_paths {
        if !p.exists() {
            return Err(anyhow!("File not found: {}", p.display()));
        }
    }

    println!("Merging {} PDF(s)...", input_paths.len());
    pdf::merge(&input_paths, &PathBuf::from(&output))?;
    println!("Saved: {}", output);
    Ok(())
}
