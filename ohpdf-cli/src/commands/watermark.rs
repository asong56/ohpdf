use std::path::PathBuf;
use anyhow::{anyhow, Result};
use ohpdf_core::pdf;

pub fn run(args: &[String]) -> Result<()> {
    let mut input = None;
    let mut output = None;
    let mut text = None;
    let mut font_size = 60.0_f32;
    let mut opacity = 0.15_f32;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => { output = args.get(i + 1).cloned(); i += 2; }
            "--text" => { text = args.get(i + 1).cloned(); i += 2; }
            "--font-size" => { font_size = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(60.0); i += 2; }
            "--opacity" => { opacity = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(0.15); i += 2; }
            arg if !arg.starts_with('-') => { input = Some(arg.to_string()); i += 1; }
            arg => return Err(anyhow!("Unknown option: {}", arg)),
        }
    }

    let input = input.ok_or_else(|| anyhow!("Usage: ohpdf-cli watermark <in.pdf> -o <out.pdf> --text \"DRAFT\""))?;
    let output = output.ok_or_else(|| anyhow!("Missing -o/--output <path>"))?;
    let text = text.unwrap_or_else(|| "WATERMARK".to_string());

    let input_path = PathBuf::from(&input);
    if !input_path.exists() {
        return Err(anyhow!("File not found: {}", input));
    }

    let opts = pdf::WatermarkOptions {
        text: &text,
        font_size,
        opacity,
        color: (0.5, 0.5, 0.5),
    };

    pdf::add_watermark(&input_path, &PathBuf::from(&output), &opts)?;
    println!("Saved: {}", output);
    Ok(())
}
