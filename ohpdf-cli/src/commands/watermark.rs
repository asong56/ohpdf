use super::{parse_args, report, ArgSpec};
use std::path::PathBuf;
use anyhow::{anyhow, Result};
use ohpdf_core::pdf;

const OUTPUT: ArgSpec = ArgSpec { names: &["-o", "--output"] };
const TEXT: ArgSpec = ArgSpec { names: &["--text"] };
const FONT_SIZE: ArgSpec = ArgSpec { names: &["--font-size"] };
const OPACITY: ArgSpec = ArgSpec { names: &["--opacity"] };

pub fn run(args: &[String]) -> Result<()> {
    let (input, values) = parse_args(args, &[OUTPUT, TEXT, FONT_SIZE, OPACITY])?;
    let [output, text, font_size, opacity] = values.try_into().unwrap();

    let input = input.ok_or_else(|| anyhow!("Usage: ohpdf-cli watermark <in.pdf> -o <out.pdf> --text \"DRAFT\""))?;
    let output = output.ok_or_else(|| anyhow!("Missing -o/--output <path>"))?;
    let text = text.unwrap_or_else(|| "WATERMARK".to_string());
    let font_size: f32 = font_size.and_then(|s| s.parse().ok()).unwrap_or(60.0);
    let opacity: f32 = opacity.and_then(|s| s.parse().ok()).unwrap_or(0.15);

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
    report(&input_path, &PathBuf::from(&output));
    Ok(())
}
