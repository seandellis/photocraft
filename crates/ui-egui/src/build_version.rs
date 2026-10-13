//! Home-screen build provenance: formats compile-time unix in the app UI language (date, then time).

use std::sync::OnceLock;

use photocraft_engine::build_info::{self, HomeScreenVersion};
use time::{format_description::StaticFormatDescription, OffsetDateTime, UtcOffset};

use crate::i18n::Lang;

/// Home-screen version lines for the current build and UI language.
pub fn home_screen_version(lang: Lang) -> HomeScreenVersion {
    let when = when_built_display(lang);
    build_info::format_home_screen_version(build_info::VERSION, build_info::short_sha(), build_info::git_branch(), when.as_deref())
}

/// Alternate home-screen line after a blank gap: `v0.8.2 (e6b18d0a8, 2026-10-13, 10:48:36 PM) [branch]`.
pub fn home_screen_detail_line(lang: Lang) -> Option<String> {
    let sha = build_info::short_sha()?;
    let date_iso = build_info::build_date()?.to_string();
    let unix = build_info::build_unix()?;
    let utc = OffsetDateTime::from_unix_timestamp(unix).ok()?;
    let local = utc.to_offset(local_offset().unwrap_or(UtcOffset::UTC));
    let time = format_time(local, lang);
    Some(build_info::format_home_screen_detail_line(build_info::VERSION, sha, &date_iso, &time, build_info::git_branch()))
}

/// Clipboard text for the full home-screen version block (primary lines, blank line, optional detail).
pub fn home_screen_clipboard_text(version: &HomeScreenVersion, detail: Option<&str>) -> String {
    let base = version.clipboard_text();
    match detail.filter(|s| !s.is_empty()) {
        Some(d) => format!("{base}\n\n{d}"),
        None => base,
    }
}

fn when_built_display(lang: Lang) -> Option<String> {
    build_info::build_unix().and_then(|unix| format_build_when(unix, lang))
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
    let fmt = if matches!(code, "ja" | "ko" | "zh-hans" | "zh-hant") {
        date_fmt_ymd_slash()
    } else if matches!(code, "de" | "el" | "hu" | "pl" | "cs" | "ru" | "uk" | "tr") {
        date_fmt_dmy_dot()
    } else {
        date_fmt_mdy_slash()
    };
    dt.format(fmt).unwrap_or_default()
}

fn format_time(dt: OffsetDateTime, lang: Lang) -> String {
    let fmt = if lang.code() == "en" { time_fmt_en_12h() } else { time_fmt_24h() };
    dt.format(fmt).unwrap_or_default()
}

fn date_fmt_ymd_slash() -> &'static StaticFormatDescription {
    static FMT: OnceLock<StaticFormatDescription> = OnceLock::new();
    FMT.get_or_init(|| parse_fmt_literal("[year]/[month padding:zero]/[day padding:zero]"))
}

fn date_fmt_dmy_dot() -> &'static StaticFormatDescription {
    static FMT: OnceLock<StaticFormatDescription> = OnceLock::new();
    FMT.get_or_init(|| parse_fmt_literal("[day padding:zero].[month padding:zero].[year]"))
}

fn date_fmt_mdy_slash() -> &'static StaticFormatDescription {
    static FMT: OnceLock<StaticFormatDescription> = OnceLock::new();
    FMT.get_or_init(|| parse_fmt_literal("[month padding:zero]/[day padding:zero]/[year]"))
}

fn time_fmt_en_12h() -> &'static StaticFormatDescription {
    static FMT: OnceLock<StaticFormatDescription> = OnceLock::new();
    FMT.get_or_init(|| parse_fmt_literal("[hour padding:zero repr:12]:[minute padding:zero]:[second padding:zero] [period]"))
}

fn time_fmt_24h() -> &'static StaticFormatDescription {
    static FMT: OnceLock<StaticFormatDescription> = OnceLock::new();
    FMT.get_or_init(|| parse_fmt_literal("[hour padding:zero]:[minute padding:zero]:[second padding:zero]"))
}

/// Fixed `time` format strings used at startup; patterns are compile-time constants.
#[allow(clippy::expect_used, deprecated)]
fn parse_fmt_literal(pattern: &'static str) -> StaticFormatDescription {
    let items = time::format_description::parse(pattern).expect("fixed home-screen locale format literal");
    Box::leak(items.into_boxed_slice())
}

#[cfg(test)]
mod tests {
    use super::*;

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
