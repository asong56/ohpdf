//! `encrypt`/`decrypt` — merged into one file since the two commands were
//! previously separate, byte-for-byte identical files apart from which
//! `ohpdf_core::pdf` function they called at the end.

use super::{parse_args, report, ArgSpec};
use std::path::PathBuf;
use anyhow::{anyhow, Result};
use ohpdf_core::pdf;

const OUTPUT: ArgSpec = ArgSpec { names: &["-o", "--output"] };
const PASSWORD: ArgSpec = ArgSpec { names: &["--password"] };

fn run(args: &[String], verb: &str, op: fn(&std::path::Path, &std::path::Path, &str) -> Result<()>) -> Result<()> {
    let (input, values) = parse_args(args, &[OUTPUT, PASSWORD])?;
    let [output, password] = values.try_into().unwrap();

    let input = input.ok_or_else(|| {
        anyhow!("Usage: ohpdf-cli {verb} <in.pdf> -o <out.pdf> --password <pw>")
    })?;
    let output = output.ok_or_else(|| anyhow!("Missing -o/--output <path>"))?;
    let password = password.ok_or_else(|| anyhow!("Missing --password <pw>"))?;

    let input_path = PathBuf::from(&input);
    if !input_path.exists() {
        return Err(anyhow!("File not found: {}", input));
    }

    op(&input_path, &PathBuf::from(&output), &password)?;
    report(&input_path, &PathBuf::from(&output));
    Ok(())
}

pub mod encrypt {
    use super::*;
    pub fn run(args: &[String]) -> Result<()> {
        super::run(args, "encrypt", pdf::encrypt)
    }
}

pub mod decrypt {
    use super::*;
    pub fn run(args: &[String]) -> Result<()> {
        super::run(args, "decrypt", pdf::decrypt)
    }
}
