use std::path::Path;
use anyhow::{Context, Result};
use mupdf::pdf::{Encryption, PdfDocument, PdfWriteOptions, Permission};

/// Add password protection to a PDF.
///
/// NOTE: the old code referenced `WriteOptions`/`Permissions` (plural) and a
/// chained `.encrypt(true)` builder — none of that exists in mupdf-rs. The
/// real types are `PdfWriteOptions` (setters like `set_encryption`,
/// `set_permissions`, `set_owner_password`, `set_user_password`),
/// `Encryption` (an enum: `None`, `Keep`, `Rc4_40`, `Rc4_128`, `Aes128`,
/// `Aes256`, ...) and `Permission` (a bitflags type, not `Permissions`).
pub fn encrypt(input: &Path, output: &Path, password: &str) -> Result<()> {
    anyhow::ensure!(!password.is_empty(), "Password cannot be empty.");

    let doc = PdfDocument::open(input.to_str().context("Path contains invalid characters.")?)
        .with_context(|| format!("Failed to open file: {}", input.display()))?;

    let mut opts = PdfWriteOptions::default();
    opts.set_encryption(Encryption::Aes256)
        .set_permissions(Permission::all())
        .set_owner_password(password)
        .set_user_password(password);

    doc.save_with_options(output.to_str().context("Output path contains invalid characters.")?, opts)
        .context("Failed to save encrypted file.")?;

    log::info!("Encrypted {} → {}", input.display(), output.display());
    Ok(())
}

/// Remove password protection from a PDF.
pub fn decrypt(input: &Path, output: &Path, password: &str) -> Result<()> {
    // `Document::open_with_password` never existed either. The real flow is:
    // open the (possibly encrypted) document, check `needs_password()`, and
    // if so call `authenticate(password)` before touching any page content.
    let mut doc = PdfDocument::open(input.to_str().context("Path contains invalid characters.")?)
        .with_context(|| format!("Failed to open file: {}", input.display()))?;

    if doc.needs_password().context("Failed to check document encryption status.")? {
        let ok = doc
            .authenticate(password)
            .context("Password authentication failed.")?;
        anyhow::ensure!(ok, "Incorrect password — could not decrypt the file.");
    }

    let mut opts = PdfWriteOptions::default();
    opts.set_encryption(Encryption::None).set_garbage(true);

    doc.save_with_options(output.to_str().context("Output path contains invalid characters.")?, opts)
        .context("Failed to save decrypted file.")?;

    log::info!("Decrypted {} → {}", input.display(), output.display());
    Ok(())
}
