use std::path::Path;
use anyhow::{Context, Result};
use mupdf::pdf::{Encryption, PdfDocument, PdfWriteOptions, Permission};

use super::pdf_ops::compact_write_options;

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
        .set_permissions(Permission::all());

    // `set_owner_password`/`set_user_password` write into a fixed-size
    // buffer inside mupdf-rs and will panic if the password is too long for
    // it, so check that ourselves first and turn it into a normal error
    // instead of letting it crash the whole app silently with no message.
    anyhow::ensure!(
        password.len() < 32,
        "Password is too long (max 31 bytes; yours is {} bytes).",
        password.len()
    );
    opts.set_owner_password(password)
        .set_user_password(password);

    doc.save_with_options(
        output.to_str().context("Output path contains invalid characters.")?,
        opts,
    )
    .with_context(|| {
        format!(
            "Failed to save encrypted file to {} (source: {}).",
            output.display(),
            input.display()
        )
    })?;

    log::info!("Encrypted {} → {}", input.display(), output.display());
    Ok(())
}

/// Remove password protection from a PDF.
pub fn decrypt(input: &Path, output: &Path, password: &str) -> Result<()> {
    let mut doc = PdfDocument::open(input.to_str().context("Path contains invalid characters.")?)
        .with_context(|| format!("Failed to open file: {}", input.display()))?;

    let needs_pw = doc
        .needs_password()
        .context("Failed to check document encryption status.")?;

    if needs_pw {
        let ok = doc
            .authenticate(password)
            .context("Password authentication failed (I/O or corrupt file).")?;
        anyhow::ensure!(ok, "Incorrect password — could not decrypt the file.");
    } else {
        log::info!(
            "{} does not appear to be password-protected; saving an unencrypted copy anyway.",
            input.display()
        );
    }

    let mut opts = compact_write_options();
    opts.set_encryption(Encryption::None);

    doc.save_with_options(
        output.to_str().context("Output path contains invalid characters.")?,
        opts,
    )
    .with_context(|| {
        format!(
            "Failed to save decrypted file to {} (source: {}).",
            output.display(),
            input.display()
        )
    })?;

    log::info!("Decrypted {} → {}", input.display(), output.display());
    Ok(())
}
