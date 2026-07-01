//! Fetches the khPOS corpus into a local, gitignored checkout.
//!
//! khPOS is CC BY-NC-SA 4.0 — evaluation-only, never bundled or committed
//! (see `docs/RESEARCH.md` §3 and `docs/RESEARCH-2.md` §5).

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

const KHPOS_REPO: &str = "https://github.com/ye-kyaw-thu/khPOS";

/// Ensure a local khPOS checkout exists under `data_dir/khpos`, cloning it
/// on first use. Returns the checkout path.
pub fn ensure_khpos(data_dir: &Path) -> io::Result<PathBuf> {
    let repo_dir = data_dir.join("khpos");
    if repo_dir.join("corpus-draft-ver-1.0").is_dir() {
        return Ok(repo_dir);
    }

    std::fs::create_dir_all(data_dir)?;
    let status = Command::new("git")
        .args(["clone", "--depth", "1", KHPOS_REPO])
        .arg(&repo_dir)
        .status()?;

    if !status.success() {
        return Err(io::Error::other(format!(
            "git clone of {KHPOS_REPO} failed ({status})"
        )));
    }

    Ok(repo_dir)
}
