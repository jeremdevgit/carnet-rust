[English](README.md) · [Français](README.fr.md) · [Español](README.es.md) · [Deutsch](README.de.md) · [Italiano](README.it.md) · [Português (Brasil)](README.pt.md) · [简体中文](README.zh.md) · [日本語](README.ja.md)

[![Windows CI](https://github.com/jeremdevgit/carnet-rust/actions/workflows/windows.yml/badge.svg)](https://github.com/jeremdevgit/carnet-rust/actions/workflows/windows.yml) [![RustSec](https://github.com/jeremdevgit/carnet-rust/actions/workflows/security.yml/badge.svg)](https://github.com/jeremdevgit/carnet-rust/actions/workflows/security.yml)

# Carnet de notes 0.4.1

<p align="center"><img src="assets/carnet-de-notes.svg" alt="Carnet de notes" width="128"></p>

Native Windows notebooks by **jeremdevgit**, with a Liquid Glass inspired light/dark interface and eight languages. System text clipboard support is enabled. Generic account connections support Nextcloud, WebDAV and Joplin Cloud/Server. Local data remains SQLCipher encrypted. **Full Joplin parity is not complete**; see [COMPATIBILITY.md](COMPATIBILITY.md).

👉 **[Download Carnet de notes for Windows](https://github.com/jeremdevgit/carnet-rust/releases/latest)** (`CarnetDeNotes-0.4.1-Windows-x64.zip`, portable application)

![Carnet de notes dark theme with demonstration notes](screenshots/en/liquid-glass-dark.png)

<p><img src="screenshots/en/liquid-glass-light.png" alt="Light theme with demonstration notes" width="49%"> <img src="screenshots/encrypted-vault.png" alt="Encrypted vault setup with a generic Nextcloud account" width="49%"></p>

*Screenshots show the real interface with demonstration data; they were captured in an offscreen test environment.*

## Run and unlock

Extract the Windows ZIP and run `CarnetDeNotes.exe`. No administrator rights or account are required for local use. The executable is unsigned and requires an OpenGL desktop driver. Mica is requested on Windows 11; translucent panels and gradients provide a fallback.

Existing 0.3 profiles remain in the same encrypted data directory and keep the same Windows Credential Manager entry. The new executable opens them without copying or converting the profile.

At first launch, choose a service, server address and credentials, then Connect and sync. Nextcloud uses an app password. Uncheck Connect an account for local-only use. No private server or organisation is preconfigured.

By default, SQLCipher uses a random key held in **Windows Credential Manager** (macOS Keychain on macOS), so no second password is required and offline use remains possible. Your sync password is not used as the database key. This mode relies on the security of your OS account; the app does not demand Windows Hello authentication each time. Manual locking hides and closes the vault but can be reopened by the same OS user. Idle locking defaults to off in system mode; lock your OS session when away.

**Use a master password (optional)** instead requires a passphrase of at least 12 characters, with no recovery/backdoor. Idle locking defaults to five minutes; 0–60 minutes can be configured (0 disables it). Existing 0.2 profiles keep their master password, with a verified switch to system protection in Settings → Change passphrase. Legacy 0.1 plaintext profiles are converted under the selected mode. The official Joplin profile is never modified. Old exports, external backups, SSD blocks and OS snapshots are not retroactively encrypted or securely erased.

Notes, attachments, history, recovery drafts, search indexes, settings and WAL pages remain encrypted. Invalid drafts are saved before locking; if that also fails, the app stays open to preserve the draft. Locking during sync may leave the worker key in memory until an in-flight request finishes or times out.

## Available functions

- French, English, Spanish, German, Italian, Brazilian Portuguese, simplified Chinese and Japanese, with a bundled OFL CJK font.
- Notebook trees, rename/move/cycle prevention and deletion preserving notes/sub-notebooks; Markdown editing/preview, internal links, duplication and notebook reassignment.
- To-dos, due dates, tags including rename/merge/removal, literal text search and paginated lists.
- 800 ms transactional autosave, Ctrl+S/Ctrl+N, 100 text snapshots per note, trash/recovery/confirmed purge, explicit unused-attachment cleanup protecting history and drafts.
- Local PNG/JPEG previews: at most four images per note, source dimensions up to 4096×4096, thumbnails up to 1024 pixels. Remote images and arbitrary URL handlers are blocked.
- UTF-8 Markdown and experimental JEX import/export. Markdown/JEX exports and exported attachment files are **unencrypted**, with UI notices. There is no general rich-text editor, plugin runtime or OCR.

## Encrypted backups

File → Back up database now asks for a **separate backup password**, making the backup portable to another device. Restore asks for that password and imports under the current vault key after preserving an encrypted recovery copy. Existing files are protected. Copying a system-managed profile folder alone does not copy its OS credential; use portable backups or resync. Losing the OS credential store can make the local cache inaccessible.

Settings → Change passphrase can switch between verified master-password protection and system protection. Before retiring a device key, a `recovery-master-*.sqlite3` copy is encrypted with the new master password. Older backups retain their previous key; older automatic device-key snapshots cannot be opened with the new master password. Recovery snapshots are not automatically pruned. Profile: `%LOCALAPPDATA%\jeremdevgit\CarnetRust\data`.

## Connect and sync

- **Nextcloud:** enter your HTTPS server URL, username and app password. First create a private `Joplin` folder in that account. The DAV URL is built for that username; an explicit `/remote.php/` DAV URL is also accepted.
- **WebDAV:** use the full HTTPS URL of a dedicated notes directory, not the server/account root.
- **Joplin Cloud / Server:** use the official Cloud preset or your server API address and account credentials. Interactive Cloud/MFA/SSO remains unsupported; no real Cloud account was tested.

Credentials/sessions stay encrypted locally. New accounts default to sync after opening and every five minutes **while unlocked**. Offline notes stay editable and a later interval retries when the network/VPN is available. Editing account fields does not apply them until Save or Connect and sync. The worker temporarily suspends editing during sync and is cancellable. HTTP is loopback-only, redirects are refused and TLS verification stays enabled.

Isolated integration scenarios cover Nextcloud 35.0.2 WebDAV and Joplin Server 3.7.1/PostgreSQL 17, two encrypted profiles, notebook/note/task/tag/attachment round trips, conflicts, trash/deletion and E2EE refusal. Shared SYNC locks coexist with Joplin clients; EXCLUSIVE migration locks block sync. DAV ETag conditions reject concurrent overwrites/deletions. Official Joplin CLI 3.7.1 exchanged unencrypted Markdown notebooks and notes with Carnet de notes in both directions on isolated Nextcloud. Compatibility remains experimental.

**Local SQLCipher is separate from Joplin E2EE. E2EE targets, shared notebooks and HTML notes remain unsupported and are refused without disabling encryption.** The server can read synced content on an E2EE-disabled target. Format 3 is required; complete snapshots are capped at 10,000 records/256 MiB and two minutes per attempt. Incoming changes get encrypted recovery copies. Conflicts require an explicit choice. Disappearing metadata or a batch deleting at least 90% of local records triggers a fail-safe stop.

## Validation and remaining work

See [VALIDATION.md](VALIDATION.md) and [SECURITY.md](SECURITY.md). Windows CI compiles and runs the Rust tests natively; the desktop GUI has not been manually verified on a Windows PC. No comparative Windows RAM/CPU measurement or security certification is claimed.

Joplin E2EE, full plugin compatibility, Web Clipper, rich text, OCR, drawings, ENEX/OneNote import, HTML/PDF export, reminders, profile selection and several other features remain unimplemented. This is a functional development milestone, not a completed full rewrite.

## Build

Rust 1.95+ (verified with 1.99.0), Microsoft C++ build tools and Perl for vendored OpenSSL:

```powershell
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --release
```

`tools/package.ps1` builds the portable ZIP. The Windows workflow only builds/tests and uploads an artifact; no release is published automatically.

## My software and license

[Public profile: jeremdevgit](https://github.com/jeremdevgit): Suite libre, Batailles de France, Pomme, Ludothèque, rJb, Boîte à outils and [JMeca](https://github.com/jeremdevgit/jmeca).

Original code and logo: GPL-3.0-or-later. Independent implementation; no Joplin source, logo or icon incorporated. Not affiliated with Joplin. Reference: [Joplin Desktop v3.7.22](https://github.com/laurent22/joplin/releases/tag/v3.7.22). Bundled libraries and font retain their own licenses; see LICENSE, THIRD-PARTY-NOTICES.txt and assets/.
