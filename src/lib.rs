// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 jeremdevgit
#![forbid(unsafe_code)]

pub mod device_key;
pub mod e2ee;
pub mod exchange;
pub mod i18n;
mod management;
pub mod media;
pub mod store;
pub mod sync;
pub mod vault;
mod webdav;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("database: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("file: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid data: {0}")]
    Invalid(&'static str),
    #[error("the note was changed in another window; your draft is preserved")]
    Conflict,
    #[error("incorrect password or damaged encrypted database")]
    Unlock,
    #[error("use a passphrase of at least 12 characters (maximum 1024 bytes)")]
    PasswordPolicy,
    #[error("an encrypted persistent profile is required")]
    EncryptionRequired,
    #[error("Synchronisation returned HTTP {0}")]
    SyncStatus(u16),
}
pub type Result<T> = std::result::Result<T, Error>;

pub fn id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}
pub fn now() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Native preview never hands image URIs to a loader (local files included).
/// Parser source ranges also cover reference-style images and HTML is inert text.
pub fn preview_markdown(body: &str) -> String {
    let mut ranges = Vec::new();
    for (event, range) in
        pulldown_cmark::Parser::new_ext(body, pulldown_cmark::Options::all()).into_offset_iter()
    {
        if matches!(
            event,
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::Image { .. })
        ) && ranges
            .last()
            .is_none_or(|r: &std::ops::Range<usize>| r.end <= range.start)
        {
            ranges.push(range);
        }
    }
    let mut out = body.to_string();
    for range in ranges.into_iter().rev() {
        out.replace_range(range, "[—]");
    }
    out
}

/// Joplin HTML notes (`markup_language` 2) are shown as inert text: tags are dropped, block
/// elements become line breaks, entities are decoded and script/style content is discarded.
pub fn html_to_text(html: &str) -> String {
    fn entity(name: &str) -> Option<char> {
        Some(match name {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" | "#39" => '\'',
            "nbsp" => '\u{a0}',
            _ => {
                let n = name.strip_prefix('#')?;
                let code = match n.strip_prefix(['x', 'X']) {
                    Some(h) => u32::from_str_radix(h, 16).ok()?,
                    None => n.parse().ok()?,
                };
                char::from_u32(code).filter(|c| !c.is_control() || *c == '\n')?
            }
        })
    }
    const BLOCKS: [&str; 16] = [
        "p",
        "div",
        "br",
        "li",
        "tr",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "blockquote",
        "pre",
        "hr",
        "table",
        "ul",
    ];
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    let mut skip_until: Option<&str> = None;
    while !rest.is_empty() {
        if let Some(end) = skip_until {
            match rest.to_ascii_lowercase().find(end) {
                Some(i) => {
                    rest = &rest[i + end.len()..];
                    skip_until = None;
                }
                None => break,
            }
            continue;
        }
        let c = rest.chars().next().unwrap_or_default();
        if c == '<' {
            let Some(close) = rest.find('>') else { break };
            let tag = rest[1..close].trim_start_matches('/').to_ascii_lowercase();
            let name: String = tag
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric())
                .collect();
            if !rest[1..close].starts_with('/') && (name == "script" || name == "style") {
                skip_until = Some(if name == "script" {
                    "</script>"
                } else {
                    "</style>"
                });
            } else if BLOCKS.contains(&name.as_str()) && !out.ends_with('\n') && !out.is_empty() {
                out.push('\n');
            }
            if name == "li" && !rest[1..close].starts_with('/') {
                out.push_str("• ");
            }
            rest = &rest[close + 1..];
        } else if c == '&' {
            match rest[1..]
                .find(';')
                .filter(|i| *i <= 10)
                .and_then(|i| entity(&rest[1..=i]).map(|ch| (ch, i)))
            {
                Some((ch, i)) => {
                    out.push(ch);
                    rest = &rest[i + 2..];
                }
                None => {
                    out.push('&');
                    rest = &rest[1..];
                }
            }
        } else if c == '\r' || c == '\n' || c == '\t' {
            if !out.ends_with([' ', '\n']) {
                out.push(' ');
            }
            rest = &rest[c.len_utf8()..];
        } else {
            out.push(c);
            rest = &rest[c.len_utf8()..];
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod html_tests {
    use super::html_to_text;

    #[test]
    fn html_notes_become_inert_text() {
        let html = "<h1>Titre</h1><p>Un &amp; deux &lt;b&gt; &#233;&#x6F22;</p><script>alert(1)</script><ul><li>a</li><li>b</li></ul><img src=x onerror=alert(1)>fin";
        assert_eq!(
            html_to_text(html),
            "Titre\nUn & deux <b> é漢\n• a\n• b\nfin"
        );
        assert_eq!(html_to_text("<style>p{}</style>x &bogus; y"), "x &bogus; y");
    }
}
