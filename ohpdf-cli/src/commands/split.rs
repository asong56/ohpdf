use std::path::PathBuf;
use anyhow::{anyhow, Result};
use ohpdf_core::pdf;

pub fn run(args: &[String]) -> Result<()> {
    let mut input = None;
    let mut output = None;
    let mut start = None;
    let mut end = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => { output = args.get(i + 1).cloned(); i += 2; }
            "--start" => { start = args.get(i + 1).and_then(|s| s.parse::<u32>().ok()); i += 2; }
            "--end" => { end = args.get(i + 1).and_then(|s| s.parse::<u32>().ok()); i += 2; }
            arg if !arg.starts_with('-') => { input = Some(arg.to_string()); i += 1; }
            arg => return Err(anyhow!("Unknown option: {}", arg)),
        }
    }

    let input = input.ok_or_else(|| anyhow!("Usage: ohpdf-cli split <in.pdf> --start N --end M -o <out.pdf>"))?;
    let output = output.ok_or_else(|| anyhow!("Missing -o/--output <path>"))?;
    let start = start.ok_or_else(|| anyhow!("Missing --start <page>"))?;
    let end = end.ok_or_else(|| anyhow!("Missing --end <page>"))?;

    let input_path = PathBuf::from(&input);
    if !input_path.exists() {
        return Err(anyhow!("File not found: {}", input));
    }

    pdf::split(&input_path, &[(start, end)], &[PathBuf::from(&output)])?;
    println!("Saved: {}", output);
    Ok(())
}
