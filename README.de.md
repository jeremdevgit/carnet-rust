<p align="center"><img src="assets/carnet-de-notes.svg" width="112" alt=""></p>

<h1 align="center">Carnet de notes</h1>

<p align="center"><b>Verschlüsselte Notizbücher für Windows mit Markdown, Aufgaben, Verlauf und experimenteller Nextcloud-, WebDAV- und Joplin-Synchronisierung.</b></p>

<p align="center"><img src="https://img.shields.io/badge/%F0%9F%93%92%20Productivity%20%26%20data-Windows-1a9a6c?style=flat-square" alt="Produktivität & Daten"> <a href="https://github.com/jeremdevgit/carnet-rust/releases"><img src="https://img.shields.io/github/v/release/jeremdevgit/carnet-rust?include_prereleases&style=flat-square&label=version" alt="version"></a> <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0--or--later-555?style=flat-square" alt="GPL-3.0-or-later"></a> <a href="https://github.com/jeremdevgit/carnet-rust/actions/workflows/windows.yml"><img src="https://github.com/jeremdevgit/carnet-rust/actions/workflows/windows.yml/badge.svg" alt="CI"></a></p>

<p align="center"><a href="README.md">English</a> · <a href="README.fr.md">Français</a> · <a href="README.es.md">Español</a> · <b>Deutsch</b> · <a href="README.it.md">Italiano</a> · <a href="README.pt.md">Português</a> · <a href="README.zh.md">简体中文</a> · <a href="README.ja.md">日本語</a></p>

<p align="center"><a href="https://github.com/jeremdevgit/carnet-rust/releases/latest"><b>⬇ Herunterladen</b></a></p>

![Carnet de notes, dunkles Design mit Demo-Notizen](screenshots/en/liquid-glass-dark.png)

Carnet de notes ist eine native Notizbuch-App für Windows mit heller/dunkler Oberfläche im Stil von Liquid Glass. **Deine Notizen liegen immer verschlüsselt auf der Festplatte**, und du kannst sie mit Nextcloud, WebDAV oder Joplin Cloud/Server synchronisieren. Volle Joplin-Parität ist noch nicht erreicht – siehe [COMPATIBILITY.md](COMPATIBILITY.md).

<p><img src="screenshots/en/liquid-glass-light.png" alt="Helles Design" width="49%"> <img src="screenshots/encrypted-vault.png" alt="Einrichtung des verschlüsselten Tresors" width="49%"></p>

## Auf einen Blick

| | |
|---|---|
| **Kategorie** | 📒 Produktivität & Daten |
| **Plattformen** | Windows |
| **Oberfläche** | 8 Sprachen (EN, FR, ES, DE, IT, PT, ZH, JA) |
| **Preis** | Kostenlos, quelloffen, ohne Werbung und Telemetrie |
| **Status** | Stabil |
| **Lizenz** | [GPL-3.0-or-later](LICENSE) |

## Funktionen

- **Notizbücher** als Baum (umbenennen, verschieben, sicheres Löschen), Markdown-Bearbeitung mit Vorschau, interne Links, Duplizieren.
- **Aufgaben** mit Fälligkeitsdatum, **Schlagwörter** (umbenennen, zusammenführen, entfernen), Textsuche, seitenweise Listen.
- **Automatisches Speichern** alle 800 ms, Strg+S / Strg+N, 100 Versionen pro Notiz, Papierkorb mit Wiederherstellung.
- Lokale **Bildvorschau** (PNG/JPEG); entfernte Bilder und unbekannte Link-Handler sind blockiert.
- **Import/Export**: UTF-8-Markdown und experimentell JEX (Exporte unverschlüsselt, mit Warnung).
- Mitgelieferte CJK-Schrift für korrektes Chinesisch und Japanisch.

## Deine Daten

- **Immer verschlüsselt**: Notizen, Anhänge, Verlauf, Entwürfe, Suchindex, Einstellungen und WAL-Seiten werden mit **SQLCipher** gespeichert.
- **Zwei Schutzmodi**: standardmäßig ein Zufallsschlüssel in der **Windows-Anmeldeinformationsverwaltung** (kein zusätzliches Passwort, offline nutzbar) oder optional ein **Master-Passwort** (mindestens 12 Zeichen, keine Hintertür, Sperre nach standardmäßig 5 Minuten Inaktivität).
- **Verschlüsselte Sicherungen** mit eigenem Passwort, übertragbar auf einen anderen PC; beim Wiederherstellen wird zuerst eine verschlüsselte Notfallkopie angelegt.
- Profilordner: `%LOCALAPPDATA%\jeremdevgit\CarnetRust\data`. Beim Deinstallieren bleibt er erhalten.

## Synchronisierung (experimentell)

| Dienst | Was du brauchst |
|---|---|
| **Nextcloud** | HTTPS-Adresse, Benutzername, ein *App-Passwort* und einen privaten Ordner `Joplin` |
| **WebDAV** | Die vollständige HTTPS-URL eines eigenen Notizordners |
| **Joplin Cloud / Server** | Die Cloud-Voreinstellung oder die API-Adresse deines Servers und Zugangsdaten (interaktives MFA/SSO nicht unterstützt) |

Die Synchronisierung läuft beim Öffnen und alle fünf Minuten, solange entsperrt; offline bleiben Notizen bearbeitbar. **Joplins Ende-zu-Ende-Verschlüsselung wird unterstützt**, in beide Richtungen (geprüft mit Joplin CLI 3.7.1). Konflikte verlangen eine bewusste Auswahl, und ein Stapel, der 90 % deiner Notizen löschen würde, wird sicherheitshalber gestoppt. HTTP nur über Loopback, Weiterleitungen werden abgelehnt, TLS-Prüfung bleibt aktiv. Ohne E2EE kann der Server die synchronisierten Inhalte lesen.

## Installation

Lade `CarnetDeNotes-<Version>-Windows-x64-Setup.exe` aus der [neuesten Version](https://github.com/jeremdevgit/carnet-rust/releases/latest) und starte es – pro Benutzer, ohne Administratorrechte. Benötigt wird ein OpenGL-Treiber; unter Windows 11 wird Mica genutzt. Die Datei ist nicht signiert, SmartScreen kann nachfragen. Profile früherer Versionen öffnen sich unverändert.

## Selbst bauen

```powershell
# Rust 1.95+, Microsoft C++ Build Tools, Perl (mitgeliefertes OpenSSL)
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --release
```

`tools/package.ps1` erstellt den Installer mit Inno Setup 6. Die CI baut, testet, installiert und deinstalliert ihn unter Windows. Details: [VALIDATION.md](VALIDATION.md).

**Noch nicht umgesetzt**: Plugins, Teilen von Notizbüchern, Web Clipper, Rich Text, OCR, Zeichnungen, ENEX/OneNote-Import, HTML/PDF-Export, Erinnerungen.

## Lizenz

GPL-3.0-or-later – siehe [LICENSE](LICENSE). Eigenständige Implementierung ohne Code, Logo oder Symbol von Joplin; nicht mit Joplin verbunden. Enthaltene Bibliotheken und die Schrift behalten ihre Lizenzen ([THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt)).

---

Eine Sicherheitslücke gefunden? Bitte privat melden (siehe [SECURITY.md](SECURITY.md)), nicht in einem öffentlichen Issue.

## Weitere freie Software von jeremdevgit

Alle kostenlos, quelloffen und werbefrei.

**🛠️ System & Hardware** — [Boîte à outils](https://github.com/jeremdevgit/boite-a-outils) · [rJb](https://github.com/jeremdevgit/rjb) · [LJ Hub](https://github.com/jeremdevgit/lj-hub) · [Decklight](https://github.com/jeremdevgit/decklight)

**📒 Produktivität & Daten** — [JMeca](https://github.com/jeremdevgit/jmeca) · [Suite Libre](https://jeremdevgit.github.io/suite-libre/)

**🎮 Musik & Spiele** — [Pomme](https://github.com/jeremdevgit/pomme) · [Ludothèque](https://github.com/jeremdevgit/ludotheque) · [AutoDex](https://github.com/jeremdevgit/autodex) · [Batailles de France](https://github.com/jeremdevgit/batailles-de-france)
