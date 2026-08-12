use super::{parse_args, report, ArgSpec};
use std::path::PathBuf;
use anyhow::{anyhow, Result};
use ohpdf_core::pdf;

const OUTPUT: ArgSpec = ArgSpec { names: &["-o", "--output"] };
const QUALITY: ArgSpec = ArgSpec { names: &["--quality"] };

pub fn run(args: &[String]) -> Result<()> {
    let (input, values) = parse_args(args, &[OUTPUT, QUALITY])?;
    let [output, quality] = values.try_into().unwrap();

    let input = input.ok_or_else(|| anyhow!("Usage: ohpdf-cli compress <in.pdf> -o <out.pdf> [--quality N]"))?;
    let output = output.ok_or_else(|| anyhow!("Missing -o/--output <path>"))?;
    let quality = quality.and_then(|s| s.parse::<u8>().ok());

    let input_path = PathBuf::from(&input);
    if !input_path.exists() {
        return Err(anyhow!("File not found: {}", input));
    }

    pdf::compress(&input_path, &PathBuf::from(&output), quality)?;
    report(&input_path, &PathBuf::from(&output));
    Ok(())
}
