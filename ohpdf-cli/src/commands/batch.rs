use std::path::PathBuf;
use anyhow::{anyhow, Result};
use ohpdf_core::batch::{run_batch, BatchOp};

pub fn run(args: &[String]) -> Result<()> {
    if args.len() < 2 {
        return Err(anyhow!(
            "Usage: ohpdf-cli batch <op> <input_dir> -o <output_dir> [options]\n\
             Ops: compress, encrypt --password <pw>, decrypt --password <pw>, \
             watermark --text <text>, merge-all --name <out.pdf>"
        ));
    }

    let op_name = &args[0];
    let rest = &args[1..];

    let mut input_dir = None;
    let mut output_dir = None;
    let mut password = None;
    let mut text = None;
    let mut name = None;
    let mut quality = None;

    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "-o" | "--output"   => { output_dir  = rest.get(i+1).cloned(); i += 2; }
            "--password"        => { password    = rest.get(i+1).cloned(); i += 2; }
            "--text"            => { text        = rest.get(i+1).cloned(); i += 2; }
            "--name"            => { name        = rest.get(i+1).cloned(); i += 2; }
            "--quality"         => { quality     = rest.get(i+1).and_then(|s| s.parse().ok()); i += 2; }
            arg if !arg.starts_with('-') => { input_dir = Some(arg.to_string()); i += 1; }
            arg => return Err(anyhow!("Unknown option: {}", arg)),
        }
    }

    let input_dir  = PathBuf::from(input_dir .ok_or_else(|| anyhow!("Missing <input_dir>"))?);
    let output_dir = PathBuf::from(output_dir.ok_or_else(|| anyhow!("Missing -o <output_dir>"))?);

    let op = match op_name.as_str() {
        "compress"  => BatchOp::Compress { quality },
        "encrypt"   => BatchOp::Encrypt  { password: password.ok_or_else(|| anyhow!("Missing --password"))? },
        "decrypt"   => BatchOp::Decrypt  { password: password.ok_or_else(|| anyhow!("Missing --password"))? },
        "watermark" => BatchOp::Watermark {
            text:      text.ok_or_else(|| anyhow!("Missing --text"))?,
            font_size: None,
            opacity:   None,
        },
        "merge-all" => BatchOp::MergeAll { output_name: name.ok_or_else(|| anyhow!("Missing --name"))? },
        _ => return Err(anyhow!("Unknown batch op '{}'. Choose: compress, encrypt, decrypt, watermark, merge-all", op_name)),
    };

    println!("Batch {op_name}: {} → {}", input_dir.display(), output_dir.display());

    let results = run_batch(&input_dir, &output_dir, op, |r| {
        match &r.error {
            None    => println!("  ✓  {}", r.input.file_name().unwrap_or_default().to_string_lossy()),
            Some(e) => eprintln!("  ✗  {} — {e}", r.input.file_name().unwrap_or_default().to_string_lossy()),
        }
    })?;

    let ok  = results.iter().filter(|r| r.error.is_none()).count();
    let err = results.len() - ok;
    println!("{ok} succeeded, {err} failed.");
    if err > 0 { std::process::exit(2); }
    Ok(())
}
