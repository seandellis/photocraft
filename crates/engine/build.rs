//! Bakes optional build provenance into `photocraft-engine` when the environment did not already
//! set it (packaging/CI). Uses git from the workspace root; skipped on wasm targets and without `.git`.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    for key in ["PHOTOCRAFT_BUILD_SHA", "PHOTOCRAFT_BUILD_DATE", "PHOTOCRAFT_BUILD_BRANCH", "PHOTOCRAFT_BUILD_UNIX"] {
        println!("cargo:rerun-if-env-changed={key}");
    }

    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        return;
    }

    let root = workspace_root();
    let git_dir = root.join(".git");
    if !git_dir.exists() {
        return;
    }

    register_git_rerun(&root, &git_dir);

    set_if_unset("PHOTOCRAFT_BUILD_SHA", git(&root, &["rev-parse", "HEAD"]));
    set_if_unset("PHOTOCRAFT_BUILD_DATE", utc_date_from_now());
    let branch = git(&root, &["rev-parse", "--abbrev-ref", "HEAD"]);
    if branch.as_deref() != Some("HEAD") {
        set_if_unset("PHOTOCRAFT_BUILD_BRANCH", branch);
    }
    set_if_unset("PHOTOCRAFT_BUILD_UNIX", build_unix_secs());
}

/// Workspace root: `crates/engine` → two levels up.
fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default());
    let root = manifest.join("../..");
    root.canonicalize().unwrap_or(root)
}

fn register_git_rerun(root: &Path, git_dir: &Path) {
    let head = git_dir.join("HEAD");
    println!("cargo:rerun-if-changed={}", head.display());
    if let Ok(raw) = std::fs::read_to_string(&head) {
        let raw = raw.trim();
        if let Some(ref_path) = raw.strip_prefix("ref: ") {
            let ref_file = root.join(".git").join(ref_path.trim());
            if ref_file.exists() {
                println!("cargo:rerun-if-changed={}", ref_file.display());
            }
        }
    }
}

fn set_if_unset(key: &str, value: Option<String>) {
    if std::env::var_os(key).is_some() {
        return;
    }
    if let Some(v) = value.filter(|s| !s.is_empty()) {
        // Values are trusted from our own git/date commands (no embedded quotes).
        println!("cargo:rustc-env={key}={v}");
    }
}

fn git(root: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git").arg("-C").arg(root).args(args).output();
    match out {
        Ok(o) if o.status.success() => {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() { None } else { Some(s) }
        }
        _ => None,
    }
}

/// UTC `YYYY-MM-DD` without shelling out (works on Windows dev builds too).
fn utc_date_from_now() -> Option<String> {
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    let (y, m, d) = utc_ymd_from_unix_secs(secs);
    Some(format!("{y:04}-{m:02}-{d:02}"))
}

/// Civil calendar date in UTC from Unix seconds (no external deps).
fn utc_ymd_from_unix_secs(secs: u64) -> (i32, u32, u32) {
    civil_from_days((secs / 86_400) as i64)
}

/// Days since 1970-01-01 → (year, month, day); Howard Hinnant's algorithm.
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i32 + era as i32 * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mp < 10 { y } else { y + 1 };
    (y, m, d)
}

fn build_unix_secs() -> Option<String> {
    SystemTime::now().duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs().to_string())
}
