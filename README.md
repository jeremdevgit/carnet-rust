<p align="center"><img src="assets/carnet-de-notes.svg" width="112" alt=""></p>

<h1 align="center">Carnet de notes</h1>

<p align="center"><b>Encrypted notebooks for Windows, with Markdown, to-dos, history and experimental Nextcloud, WebDAV and Joplin sync.</b></p>

<p align="center"><img src="https://img.shields.io/badge/%F0%9F%93%92%20Productivity%20%26%20data-Windows-1a9a6c?style=flat-square" alt="Productivity & data"> <a href="https://github.com/jeremdevgit/carnet-rust/releases"><img src="https://img.shields.io/github/v/release/jeremdevgit/carnet-rust?include_prereleases&style=flat-square&label=version" alt="version"></a> <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0--or--later-555?style=flat-square" alt="GPL-3.0-or-later"></a> <a href="https://github.com/jeremdevgit/carnet-rust/actions/workflows/windows.yml"><img src="https://github.com/jeremdevgit/carnet-rust/actions/workflows/windows.yml/badge.svg" alt="CI"></a></p>

<p align="center"><b>English</b> · <a href="README.fr.md">Français</a> · <a href="README.es.md">Español</a> · <a href="README.de.md">Deutsch</a> · <a href="README.it.md">Italiano</a> · <a href="README.pt.md">Português</a> · <a href="README.zh.md">简体中文</a> · <a href="README.ja.md">日本語</a></p>

<p align="center"><a href="https://github.com/jeremdevgit/carnet-rust/releases/latest"><b>⬇ Download</b></a></p>

![Carnet de notes, dark theme with demo notes](screenshots/en/liquid-glass-dark.png)

Carnet de notes is a native Windows notebook app with a Liquid Glass inspired light/dark interface. **Your notes are always encrypted on disk**, and you can sync them with Nextcloud, WebDAV or Joplin Cloud/Server. Full Joplin parity is not reached yet — see [COMPATIBILITY.md](COMPATIBILITY.md).

<p><img src="screenshots/en/liquid-glass-light.png" alt="Light theme" width="49%"> <img src="screenshots/encrypted-vault.png" alt="Encrypted vault setup" width="49%"></p>

## At a glance

| | |
|---|---|
| **Category** | 📒 Productivity & data |
| **Platforms** | Windows |
| **Interface** | 8 languages (EN, FR, ES, DE, IT, PT, ZH, JA) |
| **Price** | Free, open source, no ads, no telemetry |
| **Status** | Stable |
| **License** | [GPL-3.0-or-later](LICENSE) |

## Features

- **Notebooks** in a tree (rename, move, safe delete), Markdown editing and preview, internal links, duplication.
- **To-dos** with due dates, **tags** (rename, merge, remove), text search, paginated lists.
- **Autosave** every 800 ms, Ctrl+S / Ctrl+N, 100 snapshots per note, trash with recovery.
- Local **image previews** (PNG/JPEG); remote images and unknown URL handlers are blocked.
- **Import/export**: UTF-8 Markdown and experimental JEX (exports are unencrypted, with a warning).
- Bundled CJK font, so Chinese and Japanese display correctly.

## Your data

- **Always encrypted**: notes, attachments, history, drafts, search index, settings and WAL pages are stored with **SQLCipher**.
- **Two protection modes**: by default a random key kept in **Windows Credential Manager** (no extra password, works offline); or an optional **master password** (12+ characters, no backdoor, idle lock after 5 minutes by default).
- **Encrypted backups** with their own password, portable to another PC; restoring keeps an encrypted recovery copy first.
- Profile folder: `%LOCALAPPDATA%\jeremdevgit\CarnetRust\data`. Uninstalling keeps it.

## Sync (experimental)

| Service | What you need |
|---|---|
| **Nextcloud** | HTTPS address, username, an *app password*, and a private `Joplin` folder |
| **WebDAV** | The full HTTPS URL of a dedicated notes folder |
| **Joplin Cloud / Server** | The Cloud preset or your server's API address and credentials (interactive MFA/SSO not supported) |

Sync runs on open and every five minutes while unlocked; notes stay editable offline. **Joplin end-to-end encryption is supported** both ways (verified with Joplin CLI 3.7.1). Conflicts ask for an explicit choice, and a batch deleting 90% of your notes is stopped as a safety measure. HTTP is allowed only on loopback, redirects are refused, TLS verification stays on. Without E2EE, the server can read synced content.

## Install

Download `CarnetDeNotes-<version>-Windows-x64-Setup.exe` from the [latest release](https://github.com/jeremdevgit/carnet-rust/releases/latest) and run it — per user, no administrator rights. It needs an OpenGL desktop driver; Mica is used on Windows 11. The executable is unsigned, so SmartScreen may ask for confirmation. Profiles from earlier versions open as they are.

## Build from source

```powershell
# Rust 1.95+, Microsoft C++ Build Tools, Perl (vendored OpenSSL)
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --release
```

`tools/package.ps1` builds the installer with Inno Setup 6. CI builds, tests, installs and uninstalls it on Windows. Details: [VALIDATION.md](VALIDATION.md).

**Not implemented yet**: plugins, notebook sharing, Web Clipper, rich text, OCR, drawings, ENEX/OneNote import, HTML/PDF export, reminders.

## License

GPL-3.0-or-later — see [LICENSE](LICENSE). An independent implementation with no Joplin code, logo or icon; not affiliated with Joplin. Bundled libraries and font keep their licenses ([THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt)).

---

Found a vulnerability? Please report it privately (see [SECURITY.md](SECURITY.md)) rather than in a public issue.

## More free software by jeremdevgit

All free, open source and ad-free.

**🛠️ System & hardware** — [Boîte à outils](https://github.com/jeremdevgit/boite-a-outils) · [rJb](https://github.com/jeremdevgit/rjb) · [LJ Hub](https://github.com/jeremdevgit/lj-hub) · [Decklight](https://github.com/jeremdevgit/decklight)

**📒 Productivity & data** — [JMeca](https://github.com/jeremdevgit/jmeca) · [Suite Libre](https://jeremdevgit.github.io/suite-libre/)

**🎮 Music & games** — [Pomme](https://github.com/jeremdevgit/pomme) · [Ludothèque](https://github.com/jeremdevgit/ludotheque) · [AutoDex](https://github.com/jeremdevgit/autodex) · [Batailles de France](https://github.com/jeremdevgit/batailles-de-france)
