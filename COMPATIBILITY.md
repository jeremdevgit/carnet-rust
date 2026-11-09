# Suivi du portage Joplin → Rust

Référence : Joplin Desktop **v3.7.22** (version stable du 28 octobre 2026), inventaire au 7 novembre 2026. Le périmètre vise l'application Windows et ses interactions avec les services/extensions ; aucune application mobile n'est livrée. Ce tableau ne constitue pas une déclaration de compatibilité totale.

| Fonction | État 0.4.1 | Détail / travail restant |
| --- | --- | --- |
| Interface native Rust | Fonctionnel | egui/eframe + OpenGL, sans Electron |
| Thème Liquid Glass | Fonctionnel | Clair/sombre, panneaux translucides et reflets statiques ; demande Mica Windows 11, repli dessiné ; pas de réfraction optique dynamique |
| Huit langues | Fonctionnel | fr, en, es, de, it, pt-BR, zh-Hans, ja ; repli anglais, police CJK libre embarquée |
| Carnets et sous-carnets | Partiel | Création, renommage, déplacement sans cycle et suppression avec transfert des notes ; réorganisation manuelle et glisser-déposer à ajouter |
| Notes Markdown | Fonctionnel | Création, édition, sauvegarde, duplication, déplacement entre carnets ; copier/couper/coller texte système et annuler |
| Aperçu Markdown | Partiel | Titres, listes, tableaux, code, emphase ; images locales PNG/JPEG bornées ; images distantes et liens externes désactivés ; formules/diagrammes/HTML avancé non pris en charge |
| Éditeur riche WYSIWYG | À porter | Édition riche et aller-retour Markdown |
| Tâches | Partiel | État et échéance enregistrés ; notifications, fuseaux et tri par échéance à ajouter |
| Étiquettes | Partiel | Ajout/modification/filtre, renommage global, fusion et suppression ; saisie des noms contenant une virgule à améliorer |
| Recherche | Partiel | Sous-chaîne littérale en interface, FTS5 dans le moteur ; Unicode case-fold complet, opérateurs Joplin, classement et filtres avancés à porter |
| Liens internes | Partiel | Navigation :/id et remappage lors d'import ; liens joplin:// et ouverture externe à porter |
| Pièces jointes | Partiel | Ajout, stockage chiffré, export explicite en clair, aperçu PNG/JPEG et nettoyage des ressources orphelines ; lecteurs audio/vidéo/PDF à porter |
| Historique des notes | Partiel | 100 instantanés de titre/texte par note, restauration ; rétention configurable et format de révisions Joplin à porter |
| Corbeille | Partiel | Mise à la corbeille, restauration et purge définitive confirmée ; expiration automatique à ajouter |
| Import/export Markdown | Partiel | Fichier UTF-8 ; dossiers, front matter, export complet de carnets et ressources à ajouter |
| Import/export JEX | Expérimental | Types note/carnet/ressource/étiquette/relation ; remappage des identifiants et liens ; transaction intégrale ; autres types refusés |
| Fidélité JEX | À vérifier dans Joplin | Tests d'archives construites ; pas d'aller-retour validé avec le programme Joplin ; attributs inconnus des notes conservés ; timestamps de carnets et métadonnées d'étiquettes non conservés, horodatage exact de complétion des tâches non préservé |
| Sauvegarde locale | Fonctionnel | Sauvegarde SQLCipher chiffrée, intégrité vérifiée, restauration assistée avec copie de récupération chiffrée ; rétention automatique à ajouter |
| Import Evernote ENEX | À porter | HTML→Markdown, ressources et métadonnées |
| Import OneNote / Obsidian / HTML | À porter | Compatibilité des convertisseurs et ressources |
| Export HTML / PDF / RAW | À porter | Mise en page, impression et ressources |
| Synchronisation | Expérimental | Client format 3 Nextcloud/WebDAV et Joplin Server/Cloud, connexion générique ; essais sur Nextcloud 35.0.2 et Server 3.7.1 + PostgreSQL. Le client officiel Joplin CLI 3.7.1 et Carnet de notes ont échangé des carnets et notes Markdown non E2EE dans les deux sens sur Nextcloud isolé. Automatique à l’ouverture/puis toutes les 5 min tant que le coffre reste ouvert. Compte Cloud réel, MFA/SSO non vérifiés ; autres cibles à porter. Instantanés complets limités à 10 000 objets/256 Mio |
| Conflits de synchronisation | Partiel | Détection locale/distante, arrêt et choix de la version locale ou serveur ; verrous SYNC partagés et respect du verrou EXCLUSIVE de migration, ETag DAV et protection contre effacement massif ; fusion et aperçu détaillé des versions à ajouter |
| Chiffrement E2EE Joplin | À porter | Protocoles/algorithmes existants, clés multiples, migration, partage, tests connus ; aucune crypto maison |
| Chiffrement au repos | Fonctionnel | SQLCipher obligatoire ; clé système aléatoire par défaut, mot de passe maître optionnel, conversion entre modes, sauvegardes portables avec mot de passe d’export ; limites de session système précisées dans SECURITY.md |
| Plugins Joplin et API | À porter | API JS, sandbox, permissions et compatibilité à concevoir ; contrainte de coût mémoire à mesurer |
| Web Clipper / Data API | À porter | API authentifiée, validation d'origine et compatibilité extension |
| Partage, publication, Teams | À porter | Dépend des services et de la synchronisation |
| Profils multiples et instances | À porter | Un profil isolé, verrou OS exclusif ; pas de sélecteur de profils |
| Éditeur externe | À porter | Surveillance de fichier et conflits |
| OCR, dessin, tableau blanc, transcription | À porter | Moteurs, formats, accessibilité et ressources |
| IA, recherche sémantique, MCP | À inventorier/porter | Fournisseurs facultatifs, consentement, coûts et confidentialité à définir |
| Accessibilité | Partiel | AccessKit actif, libellés vérifiés dans les tests ; navigation clavier et lecteur d'écran Windows à vérifier |
| Icône, barre système, raccourcis configurables | Partiel | Logo propre dans l’application et l’exécutable ; zone de notification, palette et personnalisation à porter |
| Installateur / mises à jour | À porter | ZIP portable compilé ; MSI/EXE signé, mises à jour et désinstallation à ajouter |
| Compatibilité Windows réelle | À vérifier | Compilation et tests CI natifs x86_64 MSVC réussis ; tests UI hors écran et moteur sur hôte/container, pas encore d'essai manuel sur PC Windows |
| Optimisation mémoire/CPU | À mesurer | Rendu à la demande et listes paginées ; pas de comparaison chiffrée avec Joplin à ce stade |

## Étapes de validation du portage complet

1. Stabiliser le noyau et vérifier manuellement le binaire sur Windows 10/11 ; mesures identiques de démarrage, repos, frappe/recherche et pièces jointes contre la version de référence.
2. Compléter et vérifier les formats avec des jeux de données anonymes produits par Joplin, notamment métadonnées, liens, caractères internationaux et ressources.
3. Étendre les essais Server/Cloud et les cas de défaillance, puis porter l’E2EE avec les vecteurs officiels ; les cibles E2EE sont actuellement refusées.
4. Porter les éditeurs, convertisseurs et intégrations restantes ; décider du moteur compatible plugins et en mesurer le coût.
5. Audit externe, tests de défaillance/charge/accessibilité Windows, installateur signé et publication explicitement autorisée.

Aucun identifiant de service ni secret d'un projet existant n'est requis pour ce noyau hors ligne. Les versions de développement sont publiées sur GitHub avec leur périmètre et leurs limites.
