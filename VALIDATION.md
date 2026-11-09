# Vérification — 9 novembre 2026

Version : **0.4.1**, développement. La parité Joplin n’est pas atteinte ; voir COMPATIBILITY.md.

| Contrôle | Résultat | Limite |
| --- | --- | --- |
| Tests Rust ordinaires | **53 réussis** : 20 bibliothèque, 10 interface, 23 intégrité/échanges ; exécutés sur macOS et par GitHub Actions sur Windows | Interface graphique Windows non testée manuellement |
| Chiffrement | Mode système avec backend de test, clé manquante sans remplacement de données, transitions maître/système/maître, sauvegarde portable restaurée sur un autre profil ; mauvaise phrase, corruption authentifiée, contenu/WAL, migration atomique, schéma malformé, changement de clé, sauvegarde/restauration, identifiants de synchronisation chiffrés | Tests ciblés ; pas d’audit externe |
| Intégrité | Notes, tâches, tags, sous-carnets, cycles, historique, corbeille/purge, ressources orphelines, brouillons, versions optimistes, import JEX transactionnel et entrées hostiles | JEX : archives construites ; fidélité complète avec Joplin Desktop non certifiée |
| Interface | Copier/couper/coller Unicode et annuler simulés ; connexion générique, conservation des brouillons, verrouillage et première entrée après inactivité, 8 langues, absence de glyphes manquants, images locales bornées, blocage des images distantes ; décodage du nouveau logo | Accessibilité et comportement réels sous Windows à vérifier |
| Nextcloud WebDAV | **3 tests d’intégration supplémentaires réussis**, Nextcloud 35.0.2 isolé : URL WebDAV complète, URL simple de compte et écriture pour le client Joplin officiel ; deux profils SQLCipher, aller-retour/conflits/purge/refus E2EE, verrou SYNC partagé, verrou EXCLUSIVE bloquant, rejet d’écrasement concurrent par ETag HTTP 412 | Aucun carnet réel utilisé ; autres versions/options de déploiement à tester |
| Joplin Server | **1 test d’intégration supplémentaire réussi**, Server 3.7.1 et PostgreSQL 17, deux profils SQLCipher, coexistence des verrous SYNC et blocage EXCLUSIVE | Serveur et comptes de démonstration isolés, aucun compte Cloud réel |
| Client Joplin officiel | Joplin CLI 3.7.1 a envoyé un carnet et une note Markdown que Carnet de notes a importés avec le contenu exact ; Joplin CLI a ensuite lu un carnet et une note créés par Carnet de notes sur une autre cible Nextcloud isolée | Scénario manuel de compatibilité sans E2EE ; ressources, partage et Cloud réel non validés entre clients |
| Scénario serveur | Aller-retour notes/carnets/tâches/tags/pièces jointes, conflit et choix, corbeille/suppression, refus E2EE et protection contre cible effacée | Synchronisation expérimentale, instantanés bornés ; partage et E2EE non pris en charge |
| Rendu | **2 tests supplémentaires réussis**, Linux ARM64 Vulkan/lavapipe ; 5 captures régénérées et inspectées : clair/sombre FR/EN et création du coffre avec le nouveau nom et logo | Données fictives ; Mica réel non vérifié |
| Analyse | Clippy tous targets avec `-D warnings` et formatage réussis | Ne prouve pas l’absence de failles |
| Windows | Compilation release native x86_64-pc-windows-msvc sur GitHub Actions, Rust 1.99.0 ; SQLCipher/OpenSSL intégrés | Interface graphique Windows 10/11, pont presse-papiers et Gestionnaire d’identifiants Windows non vérifiés en situation réelle ; binaire non signé |
| Distribution | Exécutable PE Windows GUI contrôlé ; sept tailles d’icône, groupe ICO et métadonnées `Carnet de notes` / `jeremdevgit` présents ; ZIP et SHA-256 du workflow vérifiés | Pilote OpenGL nécessaire ; les métadonnées éditeur ne sont pas une signature numérique |
| Confidentialité | Aucun carnet personnel ni profil réel inclus ; les mots de passe des serveurs de test sont fournis par variables d’environnement et les phrases des coffres de test sont aléatoires ; contrôles ciblés de chemins privés et formats courants de secrets | Ne constitue pas une garantie absolue |
| Publication | Dépôt public `jeremdevgit/carnet-rust` ; workflows Windows et RustSec ; CodeQL étendu, Dependabot, analyse IA des PR, protection de `main` et signalement privé activés | Aucun scan IA complet facturable lancé ; une coche verte n'est pas une certification de sécurité |

Total : **59 tests réussis** sur les exécutions ciblées (53 ordinaires + 3 Nextcloud/WebDAV + 1 Joplin Server + 2 rendu), auxquels s’ajoute la vérification manuelle dans Joplin CLI. Les six tests dépendant d’un serveur ou GPU restent volontairement ignorés dans la commande ordinaire.

## Dépendances

`cargo-audit 0.22.2`, base RustSec commit `309392b18f6b17022f82c701f300433126901068`, mise à jour du **6 novembre 2026** : **aucune vulnérabilité connue signalée** dans Cargo.lock. Rapport : dependency-audit.json.

Trois avertissements de maintenance : zbus_names 4.3.5 (RUSTSEC-2026-0371), zvariant 5.15.1 (RUSTSEC-2026-0370), zvariant_derive 5.15.1 (RUSTSEC-2026-0372). Ces dépendances Linux sont absentes du graphe normal/build Windows vérifié. Elles restent à suivre pour un support Linux.

## Limites restantes

Aucune mesure comparative RAM, CPU, énergie ou démarrage contre Joplin n’est disponible. Mesurer sur le même PC Windows avant de revendiquer une économie. Les imports/exports volumineux restent synchrones ; la synchronisation utilise des instantanés complets. Les copies de récupération chiffrées ne sont pas automatiquement purgées.

Le chiffrement local ne constitue pas l’E2EE de synchronisation. Les cibles E2EE, les notes HTML et les carnets partagés sont refusés. Le mode système utilise une clé du compte OS, sans nouvelle authentification à chaque ouverture ; le verrouillage d’application ne remplace pas celui de Windows. Les sauvegardes portables requièrent un mot de passe d’export. Les exports explicitement demandés (Markdown, JEX et pièces jointes) sont en clair. Le verrouillage ne garantit pas l’effacement de toute copie mémoire/swap ; un appel réseau en cours peut retarder la libération de la clé du worker. Voir SECURITY.md. Aucun essai n’a utilisé les notes personnelles de l’utilisateur.
