// Exit codes:
//   0 — success
//   1 — usage error or operation failed
//   2 — batch completed but some files failed

use anyhow::Result;

mod commands;
mod pipeline;

fn main() -> Result<()> {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_help();
        return Ok(());
    }

    // Pipeline mode: ohpdf input.pdf | rotate 90 | compress | encrypt password
    // Detected when stdin is not a tty (data is being piped in) and the first
    // argument looks like a pipeline stage (not a file path).
    if pipeline::is_pipeline_stage(&args[1]) {
        return pipeline::run_stage(&args[1..]);
    }

    let result = match args[1].as_str() {
        "merge"     => commands::merge::run(&args[2..]),
        "split"     => commands::split::run(&args[2..]),
        "compress"  => commands::compress::run(&args[2..]),
        "encrypt"   => commands::crypt::encrypt::run(&args[2..]),
        "decrypt"   => commands::crypt::decrypt::run(&args[2..]),
        "watermark" => commands::watermark::run(&args[2..]),
        "rotate"    => commands::rotate::run(&args[2..]),
        "to-images" => commands::to_images::run(&args[2..]),
        "info"      => commands::info::run(&args[2..]),
        "batch"     => commands::batch::run(&args[2..]),
        "-h" | "--help" => { print_help(); Ok(()) }
        "-v" | "--version" => { println!("ohpdf-cli {}", env!("CARGO_PKG_VERSION")); Ok(()) }
        cmd => {
            eprintln!("Unknown command: '{}'\n", cmd);
            print_help();
            std::process::exit(1);
        }
    };

    if let Err(e) = &result {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }

    result
}

fn print_help() {
    println!(
        r#"ohpdf-cli - a PDF tool that lives on your computer

USAGE:
    ohpdf-cli <COMMAND> [OPTIONS]
    ohpdf-cli <in.pdf> | <stage> [args] | <stage> [args] | ...

COMMANDS:
    merge            Merge multiple PDFs into one
    split            Split a PDF page range into a new file
    compress         Compress a PDF
    encrypt          Encrypt a PDF with a password
    decrypt          Decrypt a password-protected PDF
    watermark        Add a text watermark to every page
    rotate           Rotate pages in a PDF
    to-images        Render every page as a PNG
    info             Show page count
    batch            Run an operation on every PDF in a directory

PIPELINE STAGES (read from stdin, write to stdout):
    rotate <degrees>         Rotate all pages (90, 180, 270)
    compress [quality]       Compress (quality 0-100, default 75)
    encrypt <password>       Encrypt with password
    decrypt <password>       Decrypt with password
    watermark <text>         Add text watermark

EXIT CODES:
    0   Success
    1   Error (bad arguments or operation failed)
    2   Batch completed with some failures

EXAMPLES:
    ohpdf-cli merge a.pdf b.pdf -o merged.pdf
    ohpdf-cli split in.pdf --start 1 --end 3 -o out.pdf
    ohpdf-cli compress in.pdf -o out.pdf
    ohpdf-cli rotate in.pdf -o out.pdf --degrees 90 --pages 1,3-5
    ohpdf-cli watermark in.pdf -o out.pdf --text "DRAFT"
    ohpdf-cli batch compress ./papers -o ./output
    ohpdf-cli batch merge-all ./papers -o ./output --name combined.pdf

    # Pipeline:
    cat in.pdf | ohpdf-cli rotate 90 | ohpdf-cli compress | ohpdf-cli encrypt secret > out.pdf
"#
    );
}
