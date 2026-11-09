# Changelog / Historique

## 0.4.1 — 2026-11-09 (development / développement)

Français : les synchronisations WebDAV et Joplin Server prennent désormais un verrou SYNC partagé, comme le client officiel, et respectent les verrous EXCLUSIVE réservés aux migrations. Sur WebDAV, la durée de vie du verrou est contrôlée avec la date de modification du serveur et un verrou perdu ou expiré arrête la synchronisation. Essais isolés avec Nextcloud 35.0.2, Joplin Server 3.7.1 et le client officiel Joplin CLI 3.7.1 dans les deux sens pour les carnets et notes Markdown sans E2EE. Le chiffrement E2EE Joplin, le partage et la parité totale restent à porter.

English: WebDAV and Joplin Server now acquire shared SYNC locks and respect EXCLUSIVE migration locks. DAV lock expiry is checked using the server's file modification time; a lost or expired lock stops synchronisation. Isolated interoperability checks used Nextcloud 35.0.2, Joplin Server 3.7.1 and official Joplin CLI 3.7.1 in both directions for unencrypted Markdown notebooks and notes. Joplin E2EE, sharing and full parity remain unsupported.

## 0.4.0 — 2026-11-08 (development / développement)

Français : le nom public devient **Carnet de notes**. Nouveau logo original au format vectoriel, intégré à la fenêtre et aux ressources de l’exécutable Windows, avec métadonnées éditeur `jeremdevgit`. Le nom de l’exécutable et de l’archive change ; le chemin du profil chiffré et l’identifiant du coffre système restent identiques pour retrouver les données existantes.

English: the public name is now **Carnet de notes**. An original vector logo is embedded in the window and Windows executable resources, with `jeremdevgit` publisher metadata. The executable and archive names change; the encrypted profile path and OS credential identifier remain stable so existing data stays accessible.

## 0.3.0 — 2026-11-08 (development / développement)

Français : presse-papiers système pour copier/couper/coller du texte ; connexion générique Nextcloud/WebDAV et Joplin Cloud/Server, sans adresse privée prédéfinie ; mode système par défaut avec clé locale SQLCipher aléatoire conservée dans les identifiants OS, mot de passe maître optionnel et conversion vérifiée ; sauvegarde portable avec mot de passe propre ; synchronisation après ouverture et toutes les cinq minutes tant que le coffre reste ouvert. Écritures WebDAV conditionnées par ETag, chemins/listes XML bornés, refus E2EE préservé. Tests Nextcloud 35.0.2 et Joplin Server 3.7.1 isolés.

English: native system text clipboard, generic Nextcloud/WebDAV and Joplin Cloud/Server account setup, default OS-protected local encryption key with optional master password, verified mode transitions, password-protected portable backups and five-minute sync retries while unlocked. DAV conditional ETags and bounded XML/path validation; E2EE targets remain refused. No full Joplin parity or native Windows certification is claimed.

## 0.2.0 — 2026-11-07 (development / développement)

English: mandatory SQLCipher vault, first-launch setup/unlock, legacy-profile migration, passphrase changes, manual/idle lock with encrypted recovery drafts, encrypted backup/restore, notebook move/delete preserving content, tag rename/merge/removal, trash purge, protected attachment cleanup, bounded local PNG/JPEG previews. Experimental Joplin Server sync with TLS, encrypted credential/session storage, conflict choices, transactional inbound changes and deletion fail-safes. Existing Joplin E2EE targets are refused; full Joplin parity remains unfinished.

Français : coffre SQLCipher obligatoire, création/déverrouillage au lancement, migration de l'ancien profil, changement de phrase de passe, verrouillage manuel/automatique avec brouillons de récupération chiffrés, sauvegarde/restauration chiffrée, déplacement/suppression des carnets en conservant leur contenu, renommage/fusion/suppression d'étiquettes, purge de corbeille, nettoyage protégé des pièces jointes, aperçus locaux PNG/JPEG bornés. Synchronisation Joplin Server expérimentale avec TLS, identifiants/sessions dans le coffre, choix des conflits, modifications entrantes transactionnelles et garde-fous contre les suppressions. Les cibles Joplin E2EE sont refusées ; la parité complète reste inachevée.

## 0.1.0 — 2026-11-07 (development / développement)

English: first native Rust notebook core, Liquid Glass inspired light/dark themes, eight interface languages, transactional local notes, notebooks, tags, to-dos, search, history, trash, attachments, experimental JEX and Markdown exchange, SQLite backup. Full Joplin parity, synchronization and E2EE are not implemented.

Français : premier noyau de notes Rust natif, thème inspiré de Liquid Glass clair/sombre, interface en huit langues, notes locales transactionnelles, carnets, étiquettes, tâches, recherche, historique, corbeille, pièces jointes, échanges JEX expérimentaux et Markdown, sauvegarde SQLite. La parité complète avec Joplin, la synchronisation et l'E2EE ne sont pas réalisés.
