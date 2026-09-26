use anyhow::{Context, Result, bail};
use std::{io::Write, path::Path, process::Command};
use tempfile::NamedTempFile;

#[derive(Clone, Debug)]
pub struct SigningKey {
    pub fingerprint: String,
    pub label: String,
}

pub fn list_secret_keys() -> Result<Vec<SigningKey>> {
    ensure_available()?;
    let result = Command::new("gpg")
        .args(["--batch", "--with-colons", "--list-secret-keys"])
        .output()
        .context("could not list GPG secret keys")?;
    if !result.status.success() {
        bail!(
            "could not list GPG keys: {}",
            String::from_utf8_lossy(&result.stderr).trim()
        );
    }

    let listing = String::from_utf8_lossy(&result.stdout);
    let mut keys = Vec::new();
    let mut fingerprint = String::new();
    let mut user_id = String::new();
    let mut in_primary_key = false;
    for line in listing.lines() {
        let fields: Vec<&str> = line.split(':').collect();
        match fields.first().copied() {
            Some("sec") => {
                push_key(&mut keys, &mut fingerprint, &mut user_id);
                in_primary_key = true;
            }
            Some("ssb") => in_primary_key = false,
            Some("fpr") if in_primary_key && fingerprint.is_empty() => {
                fingerprint = fields.get(9).copied().unwrap_or_default().to_string();
            }
            Some("uid") if !fingerprint.is_empty() && user_id.is_empty() => {
                user_id = decode_field(fields.get(9).copied().unwrap_or_default());
            }
            _ => {}
        }
    }
    push_key(&mut keys, &mut fingerprint, &mut user_id);
    Ok(keys)
}

pub fn sign_detached(data: &[u8], output: &Path, key: Option<&str>) -> Result<()> {
    ensure_available()?;
    let mut input = NamedTempFile::new().context("could not create signing buffer")?;
    input
        .write_all(data)
        .context("could not prepare document for signing")?;
    let mut command = Command::new("gpg");
    command.args(["--yes", "--armor", "--detach-sign"]);
    if let Some(fingerprint) = key {
        command.arg("--local-user").arg(fingerprint);
    }
    let result = command
        .arg("--output")
        .arg(output)
        .arg(input.path())
        .output()
        .context("could not launch gpg")?;
    if !result.status.success() {
        bail!(
            "gpg signing failed: {}",
            String::from_utf8_lossy(&result.stderr).trim()
        );
    }
    Ok(())
}

pub fn verify(signature: &Path, document: &Path) -> Result<String> {
    ensure_available()?;
    let result = Command::new("gpg")
        .arg("--verify")
        .arg(signature)
        .arg(document)
        .output()
        .context("could not launch gpg")?;
    let details = String::from_utf8_lossy(&result.stderr);
    if !result.status.success() {
        bail!("signature invalid: {}", compact(&details));
    }
    Ok(format!("signature valid — {}", compact(&details)))
}

fn ensure_available() -> Result<()> {
    let status = Command::new("gpg")
        .arg("--version")
        .output()
        .context("gpg is not installed or not on PATH")?;
    if !status.status.success() {
        bail!("gpg is unavailable");
    }
    Ok(())
}

fn push_key(keys: &mut Vec<SigningKey>, fingerprint: &mut String, user_id: &mut String) {
    if fingerprint.is_empty() {
        return;
    }
    let short = fingerprint[fingerprint.len().saturating_sub(16)..].to_string();
    let owner = if user_id.is_empty() {
        "Unnamed key".to_string()
    } else {
        user_id.clone()
    };
    keys.push(SigningKey {
        fingerprint: std::mem::take(fingerprint),
        label: format!("{owner} — {short}"),
    });
    user_id.clear();
}

fn decode_field(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'\\' && index + 3 < bytes.len() && bytes[index + 1] == b'x' {
            if let Ok(byte) = u8::from_str_radix(&value[index + 2..index + 4], 16) {
                result.push(byte);
                index += 4;
                continue;
            }
        }
        result.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&result).into_owned()
}

fn compact(text: &str) -> String {
    text.lines()
        .find(|line| line.contains("Good signature"))
        .or_else(|| text.lines().next())
        .unwrap_or("verified")
        .trim()
        .to_string()
}
