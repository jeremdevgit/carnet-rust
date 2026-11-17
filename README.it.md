<p align="center"><img src="assets/carnet-de-notes.svg" width="112" alt=""></p>

<h1 align="center">Carnet de notes</h1>

<p align="center"><b>Taccuini cifrati per Windows, con Markdown, attività, cronologia e sincronizzazione sperimentale con Nextcloud, WebDAV e Joplin.</b></p>

<p align="center"><img src="https://img.shields.io/badge/%F0%9F%93%92%20Productivity%20%26%20data-Windows-1a9a6c?style=flat-square" alt="Produttività e dati"> <a href="https://github.com/jeremdevgit/carnet-rust/releases"><img src="https://img.shields.io/github/v/release/jeremdevgit/carnet-rust?include_prereleases&style=flat-square&label=version" alt="version"></a> <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0--or--later-555?style=flat-square" alt="GPL-3.0-or-later"></a> <a href="https://github.com/jeremdevgit/carnet-rust/actions/workflows/windows.yml"><img src="https://github.com/jeremdevgit/carnet-rust/actions/workflows/windows.yml/badge.svg" alt="CI"></a></p>

<p align="center"><a href="README.md">English</a> · <a href="README.fr.md">Français</a> · <a href="README.es.md">Español</a> · <a href="README.de.md">Deutsch</a> · <b>Italiano</b> · <a href="README.pt.md">Português</a> · <a href="README.zh.md">简体中文</a> · <a href="README.ja.md">日本語</a></p>

<p align="center"><a href="https://github.com/jeremdevgit/carnet-rust/releases/latest"><b>⬇ Scarica</b></a></p>

![Carnet de notes, tema scuro con note dimostrative](screenshots/en/liquid-glass-dark.png)

Carnet de notes è un'app di taccuini nativa per Windows, con interfaccia chiara/scura ispirata a Liquid Glass. **Le tue note sono sempre cifrate sul disco** e puoi sincronizzarle con Nextcloud, WebDAV o Joplin Cloud/Server. La parità completa con Joplin non è ancora raggiunta: vedi [COMPATIBILITY.md](COMPATIBILITY.md).

<p><img src="screenshots/en/liquid-glass-light.png" alt="Tema chiaro" width="49%"> <img src="screenshots/encrypted-vault.png" alt="Configurazione della cassaforte cifrata" width="49%"></p>

## In breve

| | |
|---|---|
| **Categoria** | 📒 Produttività e dati |
| **Piattaforme** | Windows |
| **Interfaccia** | 8 lingue (EN, FR, ES, DE, IT, PT, ZH, JA) |
| **Prezzo** | Gratuito, open source, senza pubblicità né telemetria |
| **Stato** | Stabile |
| **Licenza** | [GPL-3.0-or-later](LICENSE) |

## Funzioni

- **Taccuini** ad albero (rinomina, sposta, eliminazione sicura), modifica e anteprima Markdown, link interni, duplicazione.
- **Attività** con scadenze, **etichette** (rinomina, unisci, rimuovi), ricerca testuale, elenchi paginati.
- **Salvataggio automatico** ogni 800 ms, Ctrl+S / Ctrl+N, 100 versioni per nota, cestino con recupero.
- **Anteprime di immagini** locali (PNG/JPEG); immagini remote e gestori di link sconosciuti bloccati.
- **Importazione/esportazione**: Markdown UTF-8 e JEX sperimentale (esportazioni non cifrate, con avviso).
- Font CJK incluso per mostrare correttamente cinese e giapponese.

## I tuoi dati

- **Sempre cifrati**: note, allegati, cronologia, bozze, indice di ricerca, impostazioni e pagine WAL sono salvati con **SQLCipher**.
- **Due modalità di protezione**: per impostazione predefinita una chiave casuale conservata in **Gestione credenziali di Windows** (nessuna password in più, funziona offline); oppure una **password principale** facoltativa (almeno 12 caratteri, nessuna backdoor, blocco dopo 5 minuti di inattività per impostazione predefinita).
- **Backup cifrati** con una propria password, trasferibili su un altro PC; il ripristino conserva prima una copia di recupero cifrata.
- Cartella del profilo: `%LOCALAPPDATA%\jeremdevgit\CarnetRust\data`. La disinstallazione la conserva.

## Sincronizzazione (sperimentale)

| Servizio | Cosa serve |
|---|---|
| **Nextcloud** | Indirizzo HTTPS, nome utente, una *password per app* e una cartella privata `Joplin` |
| **WebDAV** | L'URL HTTPS completo di una cartella dedicata alle note |
| **Joplin Cloud / Server** | La preimpostazione Cloud o l'indirizzo API del tuo server e le credenziali (MFA/SSO interattivi non supportati) |

La sincronizzazione parte all'apertura e ogni cinque minuti finché è sbloccato; le note restano modificabili offline. **La crittografia end-to-end di Joplin è supportata** in entrambe le direzioni (verificata con Joplin CLI 3.7.1). I conflitti richiedono una scelta esplicita e un lotto che eliminerebbe il 90% delle note viene fermato per sicurezza. HTTP è consentito solo in locale, i reindirizzamenti sono rifiutati, la verifica TLS resta attiva. Senza E2EE il server può leggere i contenuti sincronizzati.

## Installazione

Scarica `CarnetDeNotes-<versione>-Windows-x64-Setup.exe` dalla [versione più recente](https://github.com/jeremdevgit/carnet-rust/releases/latest) ed eseguilo: per utente, senza diritti di amministratore. Serve un driver OpenGL; su Windows 11 viene usato Mica. L'eseguibile non è firmato, SmartScreen potrebbe chiedere conferma. I profili delle versioni precedenti si aprono così come sono.

## Compilare

```powershell
# Rust 1.95+, Microsoft C++ Build Tools, Perl (OpenSSL incluso)
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --release
```

`tools/package.ps1` crea l'installer con Inno Setup 6. La CI lo compila, testa, installa e disinstalla su Windows. Dettagli: [VALIDATION.md](VALIDATION.md).

**Non ancora implementato**: plugin, condivisione dei taccuini, Web Clipper, testo formattato, OCR, disegni, importazione ENEX/OneNote, esportazione HTML/PDF, promemoria.

## Licenza

GPL-3.0-or-later — vedi [LICENSE](LICENSE). Implementazione indipendente senza codice, logo o icona di Joplin; non affiliata a Joplin. Le librerie e il font inclusi mantengono le proprie licenze ([THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt)).

---

Hai trovato una vulnerabilità? Segnalala in privato (vedi [SECURITY.md](SECURITY.md)) e non in una issue pubblica.

## Altro software libero di jeremdevgit

Tutti gratuiti, open source e senza pubblicità.

**🛠️ Sistema e hardware** — [Boîte à outils](https://github.com/jeremdevgit/boite-a-outils) · [rJb](https://github.com/jeremdevgit/rjb) · [LJ Hub](https://github.com/jeremdevgit/lj-hub) · [Decklight](https://github.com/jeremdevgit/decklight)

**📒 Produttività e dati** — [JMeca](https://github.com/jeremdevgit/jmeca) · [Suite Libre](https://jeremdevgit.github.io/suite-libre/)

**🎮 Musica e giochi** — [Pomme](https://github.com/jeremdevgit/pomme) · [Ludothèque](https://github.com/jeremdevgit/ludotheque) · [AutoDex](https://github.com/jeremdevgit/autodex) · [Batailles de France](https://github.com/jeremdevgit/batailles-de-france)
