//! Version and build provenance shared by every frontend (About dialog, `--version`, home screen).
//!
//! The version is the workspace version (`[workspace.package] version` in the root
//! `Cargo.toml`, managed by `cargo xtask version`). Optional fields come from environment
//! variables at compile time, set by CI, packaging scripts, or `build.rs` when git is available
//! locally:
//!
//! - `PHOTOCRAFT_BUILD_SHA`: the git commit (any length; shown shortened to 9 characters)
//! - `PHOTOCRAFT_BUILD_DATE`: the build date, conventionally `YYYY-MM-DD` (UTC)
//! - `PHOTOCRAFT_BUILD_BRANCH`: branch name; omitted for detached `HEAD`
//! - `PHOTOCRAFT_BUILD_UNIX`: Unix seconds when the binary was built (home screen formats this in the UI locale)
//!
//! Without sha/date (plain tarball build) the build reports itself as a dev build. Runtime code
//! does not shell out to git.

use std::sync::OnceLock;

/// The workspace version, such as `0.1.0` or `0.2.0-rc.1`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The full git commit this build was made from, when the build environment recorded it.
pub fn git_sha() -> Option<&'static str> {
    option_env!("PHOTOCRAFT_BUILD_SHA").map(str::trim).filter(|s| !s.is_empty())
}

/// The build date (`YYYY-MM-DD`), when the build environment recorded it.
///
/// When only `PHOTOCRAFT_BUILD_UNIX` is set, derives the UTC calendar date from that instant.
pub fn build_date() -> Option<&'static str> {
    if let Some(d) = option_env!("PHOTOCRAFT_BUILD_DATE").map(str::trim).filter(|s| !s.is_empty()) {
        return Some(d);
    }
    build_unix().map(|secs| ISO_DATE_FROM_UNIX.get_or_init(|| iso_date_from_unix_secs(secs)).as_str())
}

/// Git branch at compile time when recorded and not detached (`HEAD`).
pub fn git_branch() -> Option<&'static str> {
    named_branch(option_env!("PHOTOCRAFT_BUILD_BRANCH").map(str::trim))
}

/// When this binary was built, as Unix seconds (UTC instant).
pub fn build_unix() -> Option<i64> {
    option_env!("PHOTOCRAFT_BUILD_UNIX").map(str::trim).filter(|s| !s.is_empty()).and_then(|s| s.parse().ok())
}

/// The commit shortened for display.
pub fn short_sha() -> Option<&'static str> {
    git_sha().map(|s| &s[..s.len().min(9)])
}

/// `v0.1.0 (3f2a9c1d0, 2026-10-01)`, or `v0.1.0 (dev build)` without provenance.
pub fn long_version() -> String {
    format_long(VERSION, short_sha(), build_date())
}

fn format_long(version: &str, sha: Option<&str>, date: Option<&str>) -> String {
    let v = display_version(version);
    match (sha, date) {
        (Some(s), Some(d)) => format!("{v} ({s}, {d})"),
        (Some(s), None) => format!("{v} ({s})"),
        (None, Some(d)) => format!("{v} ({d})"),
        (None, None) => format!("{v} (dev build)"),
    }
}

/// User-facing version label (`v0.8.2`); workspace `VERSION` stays bare semver.
pub fn display_version(version: &str) -> String {
    if version.starts_with('v') || version.starts_with('V') {
        version.to_string()
    } else {
        format!("v{version}")
    }
}

/// `v0.8.2 (abc)` or `v0.8.2 (dev build)` when sha is missing.
fn version_with_sha(version: &str, sha: Option<&str>) -> String {
    format_long(version, sha, None)
}

/// Home-screen version lines: two when a named branch is baked, one collapsed line otherwise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeScreenVersion {
    pub primary: String,
    pub secondary: Option<String>,
}

impl HomeScreenVersion {
    /// Clipboard text matching what the home screen shows (one or two lines).
    pub fn clipboard_text(&self) -> String {
        match &self.secondary {
            Some(second) => format!("{}\n{}", self.primary, second),
            None => self.primary.clone(),
        }
    }
}

/// Formats home-screen provenance (`when_built` is already localized: `{date}, {time}`).
pub fn format_home_screen_version(version: &str, short_sha: Option<&str>, branch: Option<&str>, when_built: Option<&str>) -> HomeScreenVersion {
    let primary = version_with_sha(version, short_sha);
    let branch = named_branch(branch);
    if let Some(name) = branch {
        let secondary = match when_built.filter(|s| !s.is_empty()) {
            Some(w) => format!("{name} • {w}"),
            None => name.to_string(),
        };
        HomeScreenVersion { primary, secondary: Some(secondary) }
    } else {
        let primary = match when_built.filter(|s| !s.is_empty()) {
            Some(w) => format!("{primary} · {w}"),
            None => primary,
        };
        HomeScreenVersion { primary, secondary: None }
    }
}

/// `v0.8.2 (sha, YYYY-MM-DD, time) [branch]` — ISO date, UI-local time, branch when named.
pub fn format_home_screen_detail_line(version: &str, short_sha: &str, date_iso: &str, time_local: &str, branch: Option<&str>) -> String {
    let v_label = display_version(version);
    let mut line = format!("{v_label} ({short_sha}, {date_iso}, {time_local})");
    if let Some(name) = named_branch(branch) {
        line.push_str(&format!(" [{name}]"));
    }
    line
}

/// Branch name for display; detached checkout (`HEAD`) is treated as absent.
pub(crate) fn named_branch(branch: Option<&str>) -> Option<&str> {
    branch.filter(|b| !b.is_empty() && *b != "HEAD")
}

static ISO_DATE_FROM_UNIX: OnceLock<String> = OnceLock::new();

/// UTC `YYYY-MM-DD` from Unix seconds (matches `build.rs` civil calendar).
fn iso_date_from_unix_secs(unix: i64) -> String {
    let secs = unix.max(0) as u64;
    let (y, m, d) = utc_ymd_from_unix_secs(secs);
    format!("{y:04}-{m:02}-{d:02}")
}

fn utc_ymd_from_unix_secs(secs: u64) -> (i32, u32, u32) {
    civil_from_days((secs / 86_400) as i64)
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_version_formats() {
        assert_eq!(format_long("1.2.3", Some("abc"), Some("2026-10-01")), "v1.2.3 (abc, 2026-10-01)");
        assert_eq!(format_long("1.2.3", Some("abc"), None), "v1.2.3 (abc)");
        assert_eq!(format_long("1.2.3", None, Some("2026-10-01")), "v1.2.3 (2026-10-01)");
        assert_eq!(format_long("1.2.3-rc.1", None, None), "v1.2.3-rc.1 (dev build)");
    }

    #[test]
    fn version_with_sha_matches_release_core() {
        assert_eq!(version_with_sha("0.8.2", Some("abc")), "v0.8.2 (abc)");
        assert_eq!(version_with_sha("0.8.2", None), "v0.8.2 (dev build)");
    }

    #[test]
    fn version_is_the_workspace_version() {
        assert!(!VERSION.is_empty());
        assert!(long_version().starts_with('v') && long_version().contains(VERSION));
        if let Some(s) = short_sha() {
            assert!(s.len() <= 9);
        }
    }

    #[test]
    fn home_screen_named_branch_two_lines() {
        let v = format_home_screen_version("0.8.2", Some("a1b2c3d4e"), Some("feature/x"), Some("10/12/2026, 3:48:44 PM"));
        assert_eq!(v.primary, "v0.8.2 (a1b2c3d4e)");
        assert_eq!(v.secondary.as_deref(), Some("feature/x • 10/12/2026, 3:48:44 PM"));
    }

    #[test]
    fn home_screen_detached_collapsed() {
        let v = format_home_screen_version("0.8.2", Some("abc"), Some("HEAD"), Some("10/12/2026, 3:48:44 PM"));
        assert_eq!(v.primary, "v0.8.2 (abc) · 10/12/2026, 3:48:44 PM");
        assert!(v.secondary.is_none());
    }

    #[test]
    fn named_branch_skips_head() {
        assert_eq!(named_branch(Some("HEAD")), None);
        assert_eq!(named_branch(Some("main")), Some("main"));
    }

    #[test]
    fn home_screen_clipboard_matches_display_lines() {
        let v = format_home_screen_version("0.8.2", Some("a1b2c3d4e"), Some("feature/x"), Some("10/12/2026, 3:48:44 PM"));
        assert_eq!(v.clipboard_text(), "v0.8.2 (a1b2c3d4e)\nfeature/x • 10/12/2026, 3:48:44 PM");
        let v = format_home_screen_version("0.8.2", Some("abc"), Some("HEAD"), Some("10/12/2026, 3:48:44 PM"));
        assert_eq!(v.clipboard_text(), "v0.8.2 (abc) · 10/12/2026, 3:48:44 PM");
    }

    #[test]
    fn detail_line_format() {
        let s = format_home_screen_detail_line("0.8.2", "e6b18d0a8", "2026-10-13", "10:48:36 PM", Some("feat/x"));
        assert_eq!(s, "v0.8.2 (e6b18d0a8, 2026-10-13, 10:48:36 PM) [feat/x]");
        let no_branch = format_home_screen_detail_line("0.8.2", "abc", "2026-10-13", "22:00:00", Some("HEAD"));
        assert_eq!(no_branch, "v0.8.2 (abc, 2026-10-13, 22:00:00)");
    }

    #[test]
    fn iso_date_from_unix_secs_utc() {
        assert_eq!(iso_date_from_unix_secs(1_760_000_000), "2025-10-09");
    }
}
