[English](README.md) · [Français](README.fr.md) · [Español](README.es.md) · [Deutsch](README.de.md) · [Italiano](README.it.md) · [Português (Brasil)](README.pt.md) · [简体中文](README.zh.md) · [日本語](README.ja.md)

# Carnet Rust 0.3.0

Applicazione nativa per appunti Windows di **jeremdevgit**, con interfaccia chiara e scura ispirata a Liquid Glass e otto lingue. Copia, taglia e incolla usano gli appunti del sistema. La connessione è generica per Nextcloud, WebDAV e Joplin Cloud/Server, senza server privati preimpostati. I dati locali sono cifrati con SQLCipher. **La piena parità con Joplin non è ancora disponibile**: consulta [COMPATIBILITY.md](COMPATIBILITY.md).

## Avvio e protezione locale

Estrai lo ZIP per Windows e avvia `CarnetRust.exe`. L'uso locale non richiede un account né privilegi di amministratore. L'eseguibile non è firmato e richiede un driver OpenGL funzionante. Su Windows 11 viene richiesto Mica; altrimenti restano disponibili sfumature e pannelli traslucidi.

Per impostazione predefinita, una chiave casuale di SQLCipher viene custodita in **Gestione credenziali di Windows**. Le note restano accessibili senza rete e senza una seconda password. La protezione dipende dalla sicurezza dell'account Windows; non viene richiesta Windows Hello a ogni apertura. Blocca la sessione Windows quando ti allontani. La password dell'account di sincronizzazione non è la chiave del database.

La **password principale** è facoltativa e deve contenere almeno dodici caratteri. Non esiste un recupero di questa password. Con questa modalità il blocco automatico avviene dopo cinque minuti per impostazione predefinita; con la chiave di sistema è disattivato per impostazione predefinita. Le modalità si possono cambiare nelle impostazioni. I profili 0.2 conservano la password precedente. I profili locali 0.1 non cifrati vengono migrati, ma vecchie copie esterne e blocchi SSD non vengono cifrati retroattivamente. Il profilo ufficiale Joplin non viene modificato.

I backup portatili cifrati richiedono una **password separata per il backup**. Copiare soltanto la cartella del profilo non trasferisce la chiave custodita dal sistema; per un altro PC usa un backup portatile o sincronizza di nuovo. La perdita della credenziale di sistema può rendere inaccessibile la cache locale.

## Funzioni e sincronizzazione

Sono disponibili taccuini e sottotaccuini, note Markdown con anteprima, attività e scadenze, etichette, ricerca, cronologia, cestino, allegati e anteprime PNG/JPEG locali. Le modifiche sono salvate automaticamente. Importazione ed esportazione Markdown e JEX sperimentale sono supportate; i file esportati sono **in chiaro**.

- **Nextcloud:** inserisci l'indirizzo HTTPS del server, il nome utente e una password per l'app. Crea prima una cartella privata `Joplin` nei file di quell'account. L'app costruisce l'URL WebDAV; è accettato anche un URL completo `/remote.php/`.
- **WebDAV:** usa l'URL HTTPS completo di una cartella dedicata alle note, non la radice del server o dell'account.
- **Joplin Cloud/Server:** usa il servizio Cloud preimpostato o l'indirizzo API del tuo server e le credenziali. Accesso interattivo Cloud, MFA e SSO non sono supportati; nessun account Cloud reale è stato provato.

Per i nuovi account la sincronizzazione automatica parte dopo l'apertura e riprova ogni cinque minuti finché il contenitore è sbloccato. Le note locali restano modificabili senza rete o VPN. Le credenziali rimangono nel contenitore cifrato. Test isolati con Nextcloud 35.0.2 e Joplin Server 3.7.1 hanno avuto esito positivo; la compatibilità generale resta sperimentale.

**La cifratura locale SQLCipher non equivale alla cifratura end-to-end (E2EE) di Joplin. Destinazioni E2EE, taccuini condivisi e note HTML vengono rifiutati senza disattivare la cifratura esistente.** Senza E2EE il server può leggere i contenuti sincronizzati.

## Stato e codice sorgente

L'eseguibile Windows è stato compilato, ma non eseguito su un vero PC Windows. Non sono disponibili un confronto attendibile di RAM/CPU con Joplin né una certificazione di sicurezza. Mancano ancora E2EE, estensioni, Web Clipper, editor avanzato, OCR e altre funzioni. Consulta [VALIDATION.md](VALIDATION.md) e [SECURITY.md](SECURITY.md).

Per compilare: Rust 1.95+, strumenti C++ Microsoft e Perl; `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo build --locked --release`. Codice originale GPL-3.0-or-later. Progetto indipendente, non affiliato a Joplin. Licenze delle dipendenze in [THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt). Autore pubblico: [jeremdevgit](https://github.com/jeremdevgit).
