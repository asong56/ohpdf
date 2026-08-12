use super::{parse_args, report, ArgSpec};
use std::path::PathBuf;
use anyhow::{anyhow, Result};
use ohpdf_core::pdf;

const OUTPUT: ArgSpec = ArgSpec { names: &["-o", "--output"] };
const START: ArgSpec = ArgSpec { names: &["--start"] };
const END: ArgSpec = ArgSpec { names: &["--end"] };

pub fn run(args: &[String]) -> Result<()> {
    let (input, values) = parse_args(args, &[OUTPUT, START, END])?;
    let [output, start, end] = values.try_into().unwrap();

    let input = input.ok_or_else(|| anyhow!("Usage: ohpdf-cli split <in.pdf> --start N --end M -o <out.pdf>"))?;
    let output = output.ok_or_else(|| anyhow!("Missing -o/--output <path>"))?;
    let start: u32 = start.ok_or_else(|| anyhow!("Missing --start <page>"))?
        .parse().map_err(|_| anyhow!("--start must be a page number"))?;
    let end: u32 = end.ok_or_else(|| anyhow!("Missing --end <page>"))?
        .parse().map_err(|_| anyhow!("--end must be a page number"))?;

    let input_path = PathBuf::from(&input);
    if !input_path.exists() {
        return Err(anyhow!("File not found: {}", input));
    }

    pdf::split(&input_path, &[(start, end)], &[PathBuf::from(&output)])?;
    report(&input_path, &PathBuf::from(&output));
    Ok(())
}
