//! Home-screen build provenance: formats compile-time unix in the app UI language (date, then time).

use photocraft_engine::build_info::{self, HomeScreenVersion};
use time::{OffsetDateTime, UtcOffset};

use crate::i18n::Lang;

/// Home-screen version lines for the current build and UI language.
pub fn home_screen_version(lang: Lang) -> HomeScreenVersion {
    let when = when_built_display(lang);
    build_info::format_home_screen_version(build_info::VERSION, build_info::short_sha(), build_info::git_branch(), when.as_deref())
}

/// Alternate home-screen line after a blank gap: `v0.8.2 (e6b18d0a8, 2026-10-13, 10:48:36 PM) [branch]`.
pub fn home_screen_detail_line(lang: Lang) -> Option<String> {
    let sha = build_info::short_sha()?;
    let date_iso: String = build_info::build_date()
        .map(str::to_string)
        .or_else(|| build_info::build_unix().and_then(iso_date_from_unix))?;
    let unix = build_info::build_unix()?;
    let utc = OffsetDateTime::from_unix_timestamp(unix).ok()?;
    let local = utc.to_offset(local_offset().unwrap_or(UtcOffset::UTC));
    let time = format_time(local, lang);
    Some(format_home_screen_detail_line(build_info::VERSION, sha, &date_iso, &time, build_info::git_branch()))
}

/// Clipboard text for the full home-screen version block (primary lines, blank line, optional detail).
pub fn home_screen_clipboard_text(version: &HomeScreenVersion, detail: Option<&str>) -> String {
    let base = version.clipboard_text();
    match detail.filter(|s| !s.is_empty()) {
        Some(d) => format!("{base}\n\n{d}"),
        None => base,
    }
}

/// `v0.8.2 (sha, YYYY-MM-DD, time) [branch]` — ISO date, UI-local time, branch when named.
pub fn format_home_screen_detail_line(version: &str, short_sha: &str, date_iso: &str, time_local: &str, branch: Option<&str>) -> String {
    let v_label = if version.starts_with('v') || version.starts_with('V') {
        version.to_string()
    } else {
        format!("v{version}")
    };
    let mut line = format!("{v_label} ({short_sha}, {date_iso}, {time_local})");
    if let Some(name) = branch.filter(|b| !b.is_empty() && *b != "HEAD") {
        line.push_str(&format!(" [{name}]"));
    }
    line
}

fn iso_date_from_unix(unix: i64) -> Option<String> {
    let utc = OffsetDateTime::from_unix_timestamp(unix).ok()?;
    let fmt = time::format_description::parse_borrowed::<1>("[year]-[month padding:zero]-[day padding:zero]").ok()?;
    utc.format(&fmt).ok()
}

fn when_built_display(lang: Lang) -> Option<String> {
    if let Some(unix) = build_info::build_unix() {
        return format_build_when(unix, lang);
    }
    // UTC calendar date only when packaging set DATE but not UNIX (rare).
    build_info::build_date().and_then(|iso| date_from_iso(iso).map(|dt| format_date(dt, lang)))
}

fn date_from_iso(iso: &str) -> Option<OffsetDateTime> {
    let mut it = iso.split('-');
    let y: i32 = it.next()?.parse().ok()?;
    let m: u32 = it.next()?.parse().ok()?;
    let d: u32 = it.next()?.parse().ok()?;
    let month = time::Month::try_from(m as u8).ok()?;
    let day = u8::try_from(d).ok()?;
    let date = time::Date::from_calendar_date(y, month, day).ok()?;
    Some(date.midnight().assume_utc())
}

/// Local wall-clock `{date}, {time}` for the app's language (time at the end).
fn format_build_when(unix: i64, lang: Lang) -> Option<String> {
    let utc = OffsetDateTime::from_unix_timestamp(unix).ok()?;
    let local = utc.to_offset(local_offset().unwrap_or(UtcOffset::UTC));
    let date = format_date(local, lang);
    let time = format_time(local, lang);
    Some(format!("{date}, {time}"))
}

fn local_offset() -> Option<UtcOffset> {
    #[cfg(target_arch = "wasm32")]
    {
        None
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        time::UtcOffset::current_local_offset().ok()
    }
}

fn format_date(dt: OffsetDateTime, lang: Lang) -> String {
    let code = lang.code();
    let pattern = if matches!(code, "ja" | "ko" | "zh-hans" | "zh-hant") {
        "[year]/[month padding:zero]/[day padding:zero]"
    } else if matches!(code, "de" | "el" | "hu" | "pl" | "cs" | "ru" | "uk" | "tr") {
        "[day padding:zero].[month padding:zero].[year]"
    } else {
        "[month padding:zero]/[day padding:zero]/[year]"
    };
    let Ok(fmt) = time::format_description::parse_borrowed::<1>(pattern) else {
        return String::new();
    };
    dt.format(&fmt).unwrap_or_default()
}

fn format_time(dt: OffsetDateTime, lang: Lang) -> String {
    let code = lang.code();
    let pattern = if code == "en" {
        "[hour padding:zero repr:12]:[minute padding:zero]:[second padding:zero] [period]"
    } else {
        "[hour padding:zero]:[minute padding:zero]:[second padding:zero]"
    };
    let Ok(fmt) = time::format_description::parse_borrowed::<1>(pattern) else {
        return String::new();
    };
    dt.format(&fmt).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detail_line_matches_home_screen_alternate_format() {
        let s = format_home_screen_detail_line("0.8.2", "e6b18d0a8", "2026-10-13", "10:48:36 PM", Some("feat/version-build-info"));
        assert_eq!(s, "v0.8.2 (e6b18d0a8, 2026-10-13, 10:48:36 PM) [feat/version-build-info]");
        let no_branch = format_home_screen_detail_line("0.8.2", "abc", "2026-10-13", "22:00:00", Some("HEAD"));
        assert_eq!(no_branch, "v0.8.2 (abc, 2026-10-13, 22:00:00)");
    }

    #[test]
    fn format_build_when_puts_time_last() {
        // 2026-10-12 15:48:44 UTC — exact local string varies; must contain comma between date and time.
        let s = format_build_when(1_760_000_000, Lang::EN).expect("formatted");
        assert!(s.contains(','), "{s}");
        let (date, time) = s.split_once(", ").expect("date, time");
        assert!(!date.is_empty() && !time.is_empty());
        assert!(time.contains(':'));
    }
}
