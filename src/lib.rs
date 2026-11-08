// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 jeremdevgit
#![forbid(unsafe_code)]

pub mod device_key;
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
