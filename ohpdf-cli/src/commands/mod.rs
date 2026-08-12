pub mod batch;
pub mod compress;
pub mod crypt;
pub mod info;
pub mod merge;
pub mod rotate;
pub mod split;
pub mod to_images;
pub mod watermark;

/// Shared -o/--output flag parsing used by `merge` (the one subcommand
/// with a variable-length list of positional inputs, rather than exactly
/// one, so it can't use `ArgSpec`/`parse_args` below).
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

/// Every subcommand except `merge`/`info`/`batch` takes exactly one
/// positional `<in.pdf>` plus a handful of `--flag value` pairs, and had
/// been hand-parsing that with its own copy of the same
/// `while i < args.len() { match ... }` loop. `parse_args` is that loop,
/// written once: it walks `args`, resolves each token against `names` in
/// `flags` (in order — first match wins, so give aliases as extra
/// `ArgSpec` entries if ever needed) to fill in `values` at the same
/// index, and returns the one non-flag token as the positional input.
///
/// A flag with no following token, or a token that matches no known flag
/// and doesn't parse as the positional input slot (already filled, or
/// starts with `-`), is reported as an error the same way every command
/// already did by hand: "Unknown option: X" / "Missing value for X".
pub(crate) struct ArgSpec {
    pub names: &'static [&'static str],
}

pub(crate) fn parse_args(
    args: &[String],
    flags: &[ArgSpec],
) -> Result<(Option<String>, Vec<Option<String>>), anyhow::Error> {
    use anyhow::anyhow;

    let mut input = None;
    let mut values = vec![None; flags.len()];
    let mut i = 0;

    while i < args.len() {
        let arg = args[i].as_str();

        if let Some((slot, _)) = flags
            .iter()
            .enumerate()
            .find(|(_, spec)| spec.names.contains(&arg))
        {
            let value = args
                .get(i + 1)
                .ok_or_else(|| anyhow!("Missing value for {}", arg))?;
            values[slot] = Some(value.clone());
            i += 2;
        } else if !arg.starts_with('-') && input.is_none() {
            input = Some(arg.to_string());
            i += 1;
        } else {
            return Err(anyhow!("Unknown option: {}", arg));
        }
    }

    Ok((input, values))
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
