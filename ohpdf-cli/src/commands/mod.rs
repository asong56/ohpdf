pub mod batch;
pub mod compress;
pub mod decrypt;
pub mod encrypt;
pub mod info;
pub mod merge;
pub mod rotate;
pub mod split;
pub mod to_images;
pub mod watermark;

/// Shared -o/--output flag parsing used by every subcommand.
pub(crate) fn take_flag_value(args: &[String], names: &[&str]) -> (Option<String>, Vec<String>) {
    let mut value = None;
    let mut rest = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if names.contains(&args[i].as_str()) {
            if i + 1 < args.len() {
                value = Some(args[i + 1].clone());
                i += 2;
                continue;
            }
        }
        rest.push(args[i].clone());
        i += 1;
    }
    (value, rest)
}

/// Print a short conversion report: input size → output size.
pub(crate) fn report(input: &std::path::Path, output: &std::path::Path) {
    let size = |p: &std::path::Path| {
        std::fs::metadata(p).map(|m| m.len()).unwrap_or(0)
    };
    let (a, b) = (size(input), size(output));
    let arrow = if b <= a { "↓" } else { "↑" };
    println!(
        "Saved: {}  ({} → {}  {}{:.0}%)",
        output.display(),
        fmt_size(a), fmt_size(b),
        arrow,
        if a > 0 { (b as f64 / a as f64 * 100.0 - 100.0).abs() } else { 0.0 },
    );
}

fn fmt_size(bytes: u64) -> String {
    if bytes >= 1_048_576 { format!("{:.1}MB", bytes as f64 / 1_048_576.0) }
    else if bytes >= 1_024 { format!("{:.0}KB", bytes as f64 / 1_024.0) }
    else                   { format!("{}B", bytes) }
}
