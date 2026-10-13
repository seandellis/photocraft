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

/// The workspace version, such as `0.1.0` or `0.2.0-rc.1`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The full git commit this build was made from, when the build environment recorded it.
pub fn git_sha() -> Option<&'static str> {
    option_env!("PHOTOCRAFT_BUILD_SHA").map(str::trim).filter(|s| !s.is_empty())
}

/// The build date (`YYYY-MM-DD`), when the build environment recorded it.
pub fn build_date() -> Option<&'static str> {
    option_env!("PHOTOCRAFT_BUILD_DATE").map(str::trim).filter(|s| !s.is_empty())
}

/// Git branch at compile time, when recorded and not detached (`HEAD`).
pub fn git_branch() -> Option<&'static str> {
    option_env!("PHOTOCRAFT_BUILD_BRANCH").map(str::trim).filter(|s| !s.is_empty())
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
fn display_version(version: &str) -> String {
    if version.starts_with('v') || version.starts_with('V') {
        version.to_string()
    } else {
        format!("v{version}")
    }
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
    let primary = release_core(version, short_sha);
    let branch = display_branch(branch);
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

fn release_core(version: &str, short_sha: Option<&str>) -> String {
    let v = display_version(version);
    match short_sha {
        Some(s) => format!("{v} ({s})"),
        None => format!("{v} (dev build)"),
    }
}

/// Branch name for display; detached checkout (`HEAD`) is treated as absent.
pub(crate) fn display_branch(branch: Option<&str>) -> Option<&str> {
    branch.filter(|b| !b.is_empty() && *b != "HEAD")
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
    fn display_branch_skips_head() {
        assert_eq!(display_branch(Some("HEAD")), None);
        assert_eq!(display_branch(Some("main")), Some("main"));
    }

    #[test]
    fn home_screen_clipboard_matches_display_lines() {
        let v = format_home_screen_version("0.8.2", Some("a1b2c3d4e"), Some("feature/x"), Some("10/12/2026, 3:48:44 PM"));
        assert_eq!(v.clipboard_text(), "v0.8.2 (a1b2c3d4e)\nfeature/x • 10/12/2026, 3:48:44 PM");
        let v = format_home_screen_version("0.8.2", Some("abc"), Some("HEAD"), Some("10/12/2026, 3:48:44 PM"));
        assert_eq!(v.clipboard_text(), "v0.8.2 (abc) · 10/12/2026, 3:48:44 PM");
    }
}
