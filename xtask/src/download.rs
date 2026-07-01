//! Fetches third-party repositories into local, gitignored checkouts.
//!
//! khPOS is CC BY-NC-SA 4.0 — evaluation-only, never bundled or committed
//! (see `docs/RESEARCH.md` §3 and `docs/RESEARCH-2.md` §5). chamkho's
//! `khmerdict.txt` is MIT-licensed and *is* the source of the bundled
//! dictionary (see `ATTRIBUTION.md`) — only the derived, cleaned
//! `core/src/dict.txt` is committed, not this raw checkout.

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

const KHPOS_REPO: &str = "https://github.com/ye-kyaw-thu/khPOS";
const CHAMKHO_REPO: &str = "https://github.com/veer66/chamkho";

/// Ensure a local khPOS checkout exists under `data_dir/khpos`, cloning it
/// on first use. Returns the checkout path.
pub fn ensure_khpos(data_dir: &Path) -> io::Result<PathBuf> {
    clone_shallow(CHECK_KHPOS, KHPOS_REPO, data_dir, "khpos")
}

/// Ensure a local chamkho checkout exists under `data_dir/chamkho`, cloning
/// it on first use. Returns the checkout path.
pub fn ensure_chamkho(data_dir: &Path) -> io::Result<PathBuf> {
    clone_shallow(CHECK_CHAMKHO, CHAMKHO_REPO, data_dir, "chamkho")
}

/// A marker path (relative to the checkout root) used to decide whether a
/// clone already exists and is complete.
type PresenceCheck = fn(&Path) -> bool;

const CHECK_KHPOS: PresenceCheck = |dir| dir.join("corpus-draft-ver-1.0").is_dir();
const CHECK_CHAMKHO: PresenceCheck = |dir| dir.join("data/khmerdict.txt").is_file();

fn clone_shallow(
    already_present: PresenceCheck,
    repo_url: &str,
    data_dir: &Path,
    subdir: &str,
) -> io::Result<PathBuf> {
    let repo_dir = data_dir.join(subdir);
    if already_present(&repo_dir) {
        return Ok(repo_dir);
    }

    std::fs::create_dir_all(data_dir)?;
    let status = Command::new("git")
        .args(["clone", "--depth", "1", repo_url])
        .arg(&repo_dir)
        .status()?;

    if !status.success() {
        return Err(io::Error::other(format!(
            "git clone of {repo_url} failed ({status})"
        )));
    }

    Ok(repo_dir)
}
