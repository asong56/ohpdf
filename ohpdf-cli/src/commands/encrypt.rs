use std::path::PathBuf;
use anyhow::{anyhow, Result};
use ohpdf_core::pdf;

pub fn run(args: &[String]) -> Result<()> {
    let mut input = None;
    let mut output = None;
    let mut password = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => { output = args.get(i + 1).cloned(); i += 2; }
            "--password" => { password = args.get(i + 1).cloned(); i += 2; }
            arg if !arg.starts_with('-') => { input = Some(arg.to_string()); i += 1; }
            arg => return Err(anyhow!("Unknown option: {}", arg)),
        }
    }

    let input = input.ok_or_else(|| anyhow!("Usage: ohpdf-cli encrypt <in.pdf> -o <out.pdf> --password <pw>"))?;
    let output = output.ok_or_else(|| anyhow!("Missing -o/--output <path>"))?;
    let password = password.ok_or_else(|| anyhow!("Missing --password <pw>"))?;

    let input_path = PathBuf::from(&input);
    if !input_path.exists() {
        return Err(anyhow!("File not found: {}", input));
    }

    pdf::encrypt(&input_path, &PathBuf::from(&output), &password)?;
    println!("Saved: {}", output);
    Ok(())
}
