use anyhow::Result;

mod commands;

fn main() -> Result<()> {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_help();
        return Ok(());
    }

    let result = match args[1].as_str() {
        "merge" => commands::merge::run(&args[2..]),
        "split" => commands::split::run(&args[2..]),
        "compress" => commands::compress::run(&args[2..]),
        "encrypt" => commands::encrypt::run(&args[2..]),
        "decrypt" => commands::decrypt::run(&args[2..]),
        "watermark" => commands::watermark::run(&args[2..]),
        "to-images" => commands::to_images::run(&args[2..]),
        "info" => commands::info::run(&args[2..]),
        "-h" | "--help" => {
            print_help();
            Ok(())
        }
        "-v" | "--version" => {
            println!("ohpdf-cli {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
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

COMMANDS:
    merge            Merge multiple PDFs into one
    split            Split a PDF page range into a new file
    compress         Compress a PDF
    encrypt          Encrypt a PDF with a password
    decrypt          Decrypt a password-protected PDF
    watermark        Add a text watermark to every page
    to-images        Render every page as a PNG
    info             Show page count and basic info

EXAMPLES:
    ohpdf-cli merge a.pdf b.pdf -o merged.pdf
    ohpdf-cli split in.pdf --start 1 --end 3 -o out.pdf
    ohpdf-cli compress in.pdf -o out.pdf
    ohpdf-cli watermark in.pdf -o out.pdf --text "DRAFT"
    ohpdf-cli info in.pdf
"#
    );
}
