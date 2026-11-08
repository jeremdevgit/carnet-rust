# Vérification — 8 novembre 2026

Version : **0.3.0**, développement. La parité Joplin n’est pas atteinte ; voir COMPATIBILITY.md.

| Contrôle | Résultat | Limite |
| --- | --- | --- |
| Tests Rust ordinaires | **52 réussis** : 20 bibliothèque, 9 interface, 23 intégrité/échanges | Hôte macOS ; aucun essai natif Windows |
| Chiffrement | Mode système avec backend de test, clé manquante sans remplacement de données, transitions maître/système/maître, sauvegarde portable restaurée sur un autre profil ; mauvaise phrase, corruption authentifiée, contenu/WAL, migration atomique, schéma malformé, changement de clé, sauvegarde/restauration, identifiants de synchronisation chiffrés | Tests ciblés ; pas d’audit externe |
| Intégrité | Notes, tâches, tags, sous-carnets, cycles, historique, corbeille/purge, ressources orphelines, brouillons, versions optimistes, import JEX transactionnel et entrées hostiles | JEX : archives construites ; fidélité complète avec Joplin Desktop non certifiée |
| Interface | Copier/couper/coller Unicode et annuler simulés ; connexion générique, conservation des brouillons, verrouillage et première entrée après inactivité, 8 langues, absence de glyphes manquants, images locales bornées, blocage des images distantes | Accessibilité et comportement réels sous Windows à vérifier |
| Nextcloud WebDAV | **1 test supplémentaire réussi**, Nextcloud 35.0.2 isolé, deux profils SQLCipher, aller-retour/conflits/purge/refus E2EE et rejet d’écrasement concurrent par ETag HTTP 412 | Aucun carnet réel utilisé ; autres versions/options de déploiement à tester |
| Joplin Server | **1 test d’intégration supplémentaire réussi**, Server 3.7.1 et PostgreSQL 17.11, deux profils SQLCipher | Serveur et comptes de démonstration isolés, aucun compte Cloud réel |
| Scénario serveur | Aller-retour notes/carnets/tâches/tags/pièces jointes, conflit et choix, corbeille/suppression, refus E2EE et protection contre cible effacée | Synchronisation expérimentale, instantanés bornés ; partage et E2EE non pris en charge |
| Rendu | **2 tests supplémentaires réussis**, Linux ARM64 Vulkan/lavapipe ; 5 captures inspectées : clair/sombre FR/EN et création du coffre | Données fictives ; Mica réel non vérifié |
| Analyse | Clippy tous targets avec `-D warnings` et formatage réussis | Ne prouve pas l’absence de failles |
| Windows | Compilation release x86_64-pc-windows-gnu, Rust 1.99.0 et MinGW ; SQLCipher/OpenSSL intégrés | Exécution native Windows 10/11, pont presse-papiers et Gestionnaire d’identifiants Windows non vérifiés en situation réelle ; binaire non signé |
| Distribution | En-tête PE et DLL importées inspectés, archives contrôlées et SHA-256 fournis | Pilote OpenGL nécessaire ; pas d’installateur signé |
| Confidentialité | Aucun carnet personnel ni profil réel inclus ; contrôles ciblés de chemins privés et formats courants de secrets | Ne constitue pas une garantie absolue |
| Publication | Dépôt public `jeremdevgit/carnet-rust` ; workflows Windows et RustSec ; CodeQL, Dependabot et signalement privé activés | Les résultats des vérifications sont consultables dans GitHub Actions et Security ; une coche verte n'est pas une certification de sécurité |

Total : **56 tests réussis** sur les quatre exécutions (52 ordinaires + 1 Nextcloud + 1 Joplin Server + 2 rendu). Les quatre tests dépendant d’un serveur ou GPU restent volontairement ignorés dans la commande ordinaire.

## Dépendances

`cargo-audit 0.22.2`, base RustSec commit `309392b18f6b17022f82c701f300433126901068`, mise à jour du **6 novembre 2026** : **aucune vulnérabilité connue signalée** dans Cargo.lock. Rapport : dependency-audit.json.

Trois avertissements de maintenance : zbus_names 4.3.5 (RUSTSEC-2026-0371), zvariant 5.15.1 (RUSTSEC-2026-0370), zvariant_derive 5.15.1 (RUSTSEC-2026-0372). Ces dépendances Linux sont absentes du graphe normal/build Windows vérifié. Elles restent à suivre pour un support Linux.

## Limites restantes

Aucune mesure comparative RAM, CPU, énergie ou démarrage contre Joplin n’est disponible. Mesurer sur le même PC Windows avant de revendiquer une économie. Les imports/exports volumineux restent synchrones ; la synchronisation utilise des instantanés complets. Les copies de récupération chiffrées ne sont pas automatiquement purgées.

Le chiffrement local ne constitue pas l’E2EE de synchronisation. Les cibles E2EE, les notes HTML et les carnets partagés sont refusés. Le mode système utilise une clé du compte OS, sans nouvelle authentification à chaque ouverture ; le verrouillage d’application ne remplace pas celui de Windows. Les sauvegardes portables requièrent un mot de passe d’export. Les exports explicitement demandés (Markdown, JEX et pièces jointes) sont en clair. Le verrouillage ne garantit pas l’effacement de toute copie mémoire/swap ; un appel réseau en cours peut retarder la libération de la clé du worker. Voir SECURITY.md. Aucun essai n’a utilisé les notes personnelles de l’utilisateur.
