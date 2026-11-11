<p align="center"><img src="assets/carnet-de-notes.svg" alt="Carnet de notes" width="128"></p>

# Carnet de notes

[![Windows CI](https://github.com/jeremdevgit/carnet-rust/actions/workflows/windows.yml/badge.svg)](https://github.com/jeremdevgit/carnet-rust/actions/workflows/windows.yml) [![RustSec](https://github.com/jeremdevgit/carnet-rust/actions/workflows/security.yml/badge.svg)](https://github.com/jeremdevgit/carnet-rust/actions/workflows/security.yml)

[English](README.md) · [Français](README.fr.md) · [Español](README.es.md) · [Deutsch](README.de.md) · [Italiano](README.it.md) · [Português (Brasil)](README.pt.md) · [简体中文](README.zh.md) · [日本語](README.ja.md)


Native Notizbuch-App für Windows von **jeremdevgit**, mit einer von Liquid Glass inspirierten hellen und dunklen Oberfläche in acht Sprachen. Die App unterstützt Kopieren, Ausschneiden und Einfügen über die Zwischenablage des Systems. Sie kann sich mit Nextcloud, WebDAV und Joplin Cloud/Server verbinden. Es ist kein privater Server voreingestellt. Die lokalen Daten werden mit SQLCipher verschlüsselt. **Die vollständige Funktionsgleichheit mit Joplin ist noch nicht erreicht**; Einzelheiten stehen in [COMPATIBILITY.md](COMPATIBILITY.md).

👉 **[Carnet de notes für Windows herunterladen](https://github.com/jeremdevgit/carnet-rust/releases/latest)** (`CarnetDeNotes-0.4.1-Windows-x64.zip`, portable App)

![Dunkles Design mit Beispielnotizen](screenshots/en/liquid-glass-dark.png)

<p><img src="screenshots/en/liquid-glass-light.png" alt="Helles Design mit Beispielnotizen" width="49%"> <img src="screenshots/encrypted-vault.png" alt="Einrichtung des verschlüsselten Tresors" width="49%"></p>

*Die Aufnahmen zeigen die echte Oberfläche mit Beispieldaten aus einer bildschirmlosen Testumgebung.*

## Installation und lokaler Tresor

Das Windows-ZIP entpacken und `CarnetDeNotes.exe` starten. Für die lokale Nutzung sind weder ein Konto noch Administratorrechte erforderlich. Die ausführbare Datei ist nicht signiert und benötigt einen funktionierenden OpenGL-Treiber. Unter Windows 11 wird Mica angefordert; andernfalls bleiben Farbverläufe und durchscheinende Bereiche verfügbar.

Bestehende 0.3-Profile bleiben im selben verschlüsselten Datenordner und behalten denselben Eintrag im Windows-Anmeldeinformationsmanager. Die neue Programmdatei öffnet sie ohne Kopie oder Profilkonvertierung.

Standardmäßig wird ein zufälliger SQLCipher-Schlüssel im **Windows-Anmeldeinformationsmanager** gespeichert. So sind Notizen auch ohne Netzwerk und ohne zweites Passwort verfügbar. Der Schutz hängt von der Sicherheit des Windows-Kontos ab; Windows Hello wird nicht bei jedem Start angefordert. Beim Verlassen des Computers die Windows-Sitzung sperren. Das Passwort des Synchronisierungskontos dient nicht als Datenbankschlüssel.

Optional kann ein **Master-Passwort** mit mindestens zwölf Zeichen verwendet werden. Dafür gibt es keine Wiederherstellungsmöglichkeit. Bei diesem Modus wird der Tresor standardmäßig nach fünf Minuten Inaktivität gesperrt; im Systemschlüssel-Modus ist die automatische Sperre standardmäßig ausgeschaltet. Beide Modi können in den Einstellungen gewechselt werden. Profile aus Version 0.2 behalten ihr bisheriges Master-Passwort. Lokale Klartextprofile aus 0.1 werden verschlüsselt migriert; alte externe Kopien oder SSD-Blöcke werden dadurch nicht nachträglich verschlüsselt. Das offizielle Joplin-Profil bleibt unberührt.

Verschlüsselte, portable Sicherungen verwenden ein **eigenes Sicherungspasswort**. Wer nur den Profilordner kopiert, überträgt den Schlüssel aus dem Windows-Anmeldeinformationsmanager nicht. Für einen anderen PC eine portable Sicherung verwenden oder erneut synchronisieren. Geht der Systemschlüssel verloren, kann der lokale Cache unzugänglich werden.

## Funktionen und Synchronisierung

Notizbücher und Unterordner, Markdown-Notizen und Vorschau, Aufgaben mit Fälligkeit, Schlagwörter, Suche, Verlauf, Papierkorb, Anhänge und lokale PNG/JPEG-Vorschau sind vorhanden. Änderungen werden automatisch gespeichert. Markdown- und experimentelle JEX-Importe/-Exporte sind möglich; exportierte Dateien sind **unverschlüsselt**.

- **Nextcloud:** HTTPS-Serveradresse, Benutzername und App-Passwort eingeben. Zuvor im privaten Dateibereich des Kontos einen Ordner `Joplin` anlegen. Die App bildet daraus die WebDAV-Adresse; eine vollständige `/remote.php/`-Adresse kann ebenfalls eingegeben werden.
- **WebDAV:** die vollständige HTTPS-Adresse eines eigenen Notizenordners verwenden, nicht die Wurzel des Servers oder Kontos.
- **Joplin Cloud/Server:** Cloud-Voreinstellung oder eigene Server-API-Adresse und Kontodaten verwenden. Interaktive Cloud-Anmeldung, MFA und SSO werden nicht unterstützt; ein echtes Cloud-Konto wurde nicht getestet.

Nach dem Öffnen synchronisieren neue Konten automatisch und versuchen es alle fünf Minuten erneut, solange der Tresor geöffnet ist. Ohne Netzwerk oder VPN lassen sich lokale Notizen weiter bearbeiten. Zugangsdaten bleiben im verschlüsselten Tresor. Die Kompatibilität wurde mit isolierten Instanzen von Nextcloud 35.0.2 und Joplin Server 3.7.1 getestet; sie ist weiterhin experimentell.

**Lokale SQLCipher-Verschlüsselung ist keine Joplin-Ende-zu-Ende-Verschlüsselung (E2EE). E2EE-Ziele, geteilte Notizbücher und HTML-Notizen werden abgelehnt, ohne bestehende Verschlüsselung abzuschalten.** Auf einem Ziel ohne E2EE kann der Server synchronisierte Inhalte lesen.

## Status und Quellcode

Der Windows-Build wurde kompiliert, aber noch nicht auf einem echten Windows-PC ausgeführt. Es gibt keinen belastbaren RAM-/CPU-Vergleich mit Joplin und keine Sicherheitszertifizierung. E2EE, Plugins, Web Clipper, Rich-Text-Editor, OCR und weitere Funktionen fehlen noch. Siehe [VALIDATION.md](VALIDATION.md) und [SECURITY.md](SECURITY.md).

Zum Bauen: Rust 1.95+, Microsoft C++ Build Tools und Perl; `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo build --locked --release`. Der ursprüngliche Code steht unter GPL-3.0-or-later. Unabhängiges Projekt ohne Zugehörigkeit zu Joplin; Lizenzen der Abhängigkeiten stehen in [THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt). Öffentlicher Autor: [jeremdevgit](https://github.com/jeremdevgit).
