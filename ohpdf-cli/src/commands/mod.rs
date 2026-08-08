pub mod merge;
pub mod split;
pub mod compress;
pub mod encrypt;
pub mod decrypt;
pub mod watermark;
pub mod to_images;
pub mod info;

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
