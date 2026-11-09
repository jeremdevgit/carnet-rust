[English](README.md) · [Français](README.fr.md) · [Español](README.es.md) · [Deutsch](README.de.md) · [Italiano](README.it.md) · [Português (Brasil)](README.pt.md) · [简体中文](README.zh.md) · [日本語](README.ja.md)

[![Windows CI](https://github.com/jeremdevgit/carnet-rust/actions/workflows/windows.yml/badge.svg)](https://github.com/jeremdevgit/carnet-rust/actions/workflows/windows.yml) [![RustSec](https://github.com/jeremdevgit/carnet-rust/actions/workflows/security.yml/badge.svg)](https://github.com/jeremdevgit/carnet-rust/actions/workflows/security.yml)

# Carnet de notes 0.4.1

<p align="center"><img src="assets/carnet-de-notes.svg" alt="Carnet de notes" width="128"></p>

Application native de notes Windows, par **jeremdevgit**, avec un thème inspiré de **Liquid Glass**, en huit langues. Copier/couper/coller du texte utilise maintenant le presse-papiers système. Connexion générique Nextcloud, WebDAV ou Joplin Cloud/Server, sans serveur personnel prédéfini. Le stockage local reste chiffré avec SQLCipher. **La parité complète avec Joplin n'est pas encore atteinte** : [suivi détaillé](COMPATIBILITY.md).

👉 **[Télécharger Carnet de notes pour Windows](https://github.com/jeremdevgit/carnet-rust/releases/latest)** (`CarnetDeNotes-0.4.1-Windows-x64.zip`, application portable)

![Thème sombre de Carnet de notes avec des notes de démonstration](screenshots/en/liquid-glass-dark.png)

<p><img src="screenshots/en/liquid-glass-light.png" alt="Thème clair avec notes de démonstration" width="49%"> <img src="screenshots/encrypted-vault.png" alt="Création du coffre chiffré avec un compte Nextcloud générique" width="49%"></p>

*Captures de la véritable interface avec des données de démonstration, prises dans un environnement de rendu hors écran.*

## Démarrer

Extraire le ZIP Windows, puis ouvrir `CarnetDeNotes.exe`. Aucun compte ni droit administrateur requis pour les fonctions locales. L'exécutable n'est pas signé et nécessite un pilote OpenGL opérationnel. Windows 11 peut fournir le fond Mica ; sinon les dégradés et panneaux translucides restent disponibles.

Les profils 0.3 restent dans le même dossier chiffré et conservent la même entrée du Gestionnaire d’identifiants Windows. Le nouvel exécutable les ouvre sans copie ni conversion du profil.

Au premier lancement, choisir son service et saisir l’adresse et les identifiants, puis **Se connecter et synchroniser**. Le mot de passe d’application Nextcloud est recommandé pour Nextcloud. Pour utiliser uniquement les fonctions locales, décocher **Se connecter à un compte**.

Par défaut, le coffre local utilise une clé aléatoire enregistrée dans le **Gestionnaire d’identifiants Windows** (Trousseau sur macOS). L’application peut donc fonctionner hors connexion sans réclamer un second mot de passe. Cette protection dépend de la sécurité de la session système et ne constitue pas une demande Windows Hello à chaque ouverture. Le mot de passe du compte sert à la connexion réseau ; il n’est pas utilisé comme clé SQLCipher.

L’option **Utiliser un mot de passe maître (optionnel)** permet de choisir une phrase d’au moins 12 caractères. Ce secret reste distinct du compte de synchronisation. **Sans cette phrase, le coffre protégé par mot de passe maître ne peut pas être récupéré.** Un ancien profil 0.2.0 conserve sa phrase actuelle ; dans Paramètres → Changer la phrase de passe, on peut passer à la protection système après l’avoir vérifiée.

Un profil local 0.1.0 en clair est reconnu puis converti en SQLCipher sous le mode choisi, après vérification et remplacement atomique. Cela ne chiffre pas les anciens exports/sauvegardes externes et ne garantit pas l’effacement des anciens blocs SSD ou instantanés système. Le profil officiel de Joplin n’est jamais modifié.

## Fonctions locales

- Français, anglais, espagnol, allemand, italien, portugais du Brésil, chinois simplifié et japonais ; police libre CJK embarquée.
- Carnets et sous-carnets : création, renommage, déplacement avec contrôle des cycles, suppression avec déplacement des notes et conservation des sous-carnets. Les commandes sont accessibles par clic droit.
- Notes Markdown, aperçu, liens internes, duplication et déplacement ; tâches avec état et échéance ; étiquettes avec renommage/fusion/suppression ; recherche et listes paginées.
- Enregistrement après 800 ms sans frappe ; Ctrl+S/Ctrl+N ; 100 instantanés texte par note ; corbeille, restauration et purge confirmée ; nettoyage explicite des pièces jointes inutilisées en protégeant l'historique et les brouillons.
- Pièces jointes dans la base chiffrée. Aperçus PNG/JPEG locaux limités à quatre images par note, source de 4096 × 4096 pixels au plus, miniature de 1024 pixels au plus. Aucune image distante ni ouverture automatique de pièce jointe.
- Import/export Markdown UTF-8 et JEX expérimental. Les exports Markdown/JEX et les fichiers joints exportés sont **en clair**, avec indication dans l'interface.

## Coffre, verrouillage et sauvegardes

La base, les pièces jointes, l'historique, les brouillons de récupération, les réglages et les index de recherche sont chiffrés ; les pages du journal WAL le sont également. SQLCipher 4 emploie ses paramètres standards, avec OpenSSL embarqué. Aucun système cryptographique propriétaire n'a été inventé.

Le bouton **Verrouiller** ferme le profil. En mode système, l’application se rouvre avec la clé du compte Windows ; ce bouton masque le contenu mais ne réclame pas une nouvelle authentification système. Verrouillez votre session Windows pour protéger un ordinateur laissé sans surveillance. Le verrouillage automatique est désactivé par défaut dans ce mode. En mode maître, il est réglé à cinq minutes. Réglage 0–60 minutes, où 0 le désactive. Un brouillon invalide est conservé chiffré avant verrouillage ; si cette écriture échoue aussi, le verrouillage est refusé pour préserver le brouillon. Une requête de synchronisation en cours peut retarder la libération de sa clé.

**Fichiers → Sauvegarder la base** demande désormais un **mot de passe propre à la sauvegarde**, afin de pouvoir la restaurer sur un autre appareil. **Paramètres → Restaurer une sauvegarde chiffrée** demande ce mot de passe, conserve une copie de récupération chiffrée et réimporte sous la clé actuelle du coffre. Les fichiers existants ne sont pas écrasés.

Dans **Paramètres → Changer la phrase de passe**, un coffre en mode système peut recevoir un mot de passe maître. Les anciennes sauvegardes gardent leur propre clé. Avant de retirer la clé système, une copie `recovery-master-*.sqlite3` est créée sous la nouvelle phrase ; les anciens instantanés automatiques sous la clé retirée ne sont plus restaurables avec cette nouvelle phrase. Les copies de récupération ne sont pas automatiquement purgées.

**La clé système n’est pas incluse dans un dossier de profil copié sur un autre PC.** Utilisez la sauvegarde portable avec son mot de passe pour transporter le contenu, ou resynchronisez le compte sur le nouvel appareil. La perte du coffre d’identifiants système peut rendre le cache local inaccessible.

Sous Windows, le profil propre à l'application se trouve dans `%LOCALAPPDATA%\jeremdevgit\CarnetRust\data`. Aucun chemin personnel n'est codé en dur.

## Connexion et synchronisation

- **Nextcloud** : saisir l’adresse HTTPS de son serveur, son nom d’utilisateur et un mot de passe d’application. Créer le dossier privé **Joplin** dans les fichiers de ce compte avant la première connexion. L’application construit l’adresse `/remote.php/dav/files/UTILISATEUR/Joplin/`. Une URL complète contenant `/remote.php/` peut aussi être saisie.
- **WebDAV** : saisir l’URL HTTPS complète du dossier de notes dédié et ses identifiants. Ne pas choisir la racine générale du serveur ou du compte.
- **Joplin Cloud / Joplin Server** : le bouton Joplin Cloud renseigne son API officielle ; pour un autre serveur, saisir son adresse HTTPS et les identifiants. L’authentification interactive Cloud/MFA/SSO reste non prise en charge ; aucun compte Cloud réel n’a été testé.

Les identifiants et sessions restent dans le coffre local chiffré. La synchronisation automatique, activée pour les nouveaux comptes, démarre après ouverture puis réessaie toutes les cinq minutes **tant que le coffre reste ouvert**. Sans réseau/VPN, les notes locales restent consultables et modifiables ; un nouvel essai aura lieu au prochain intervalle. Les identifiants modifiés dans la fenêtre ne sont appliqués qu’après **Enregistrer** ou **Se connecter et synchroniser**. Le traitement réseau utilise un travailleur séparé, suspend l’édition pendant l’échange et peut être annulé. HTTP est réservé à la boucle locale pour les tests ; redirections refusées et certificats TLS vérifiés.

Tests isolés : **Nextcloud 35.0.2** en WebDAV et **Joplin Server 3.7.1/PostgreSQL 17**, deux profils chiffrés, notes/carnets/tâches/tags/pièces jointes, conflits, suppressions et refus E2EE. Les verrous SYNC coexistent avec ceux de Joplin ; un verrou EXCLUSIVE de migration bloque la synchronisation. Les écritures/suppressions WebDAV utilisent les ETag pour refuser l’écrasement d’un fichier changé entre-temps. Le client officiel Joplin CLI 3.7.1 et Carnet de notes ont échangé des carnets et notes Markdown sans E2EE dans les deux sens sur Nextcloud isolé. La compatibilité générale avec toutes les versions et options de serveur reste expérimentale.

Les changements concurrents produisent des conflits à résoudre dans l'interface avant de poursuivre. Une copie de récupération chiffrée est créée avant l'application des changements entrants. Un garde-fou arrête une cible déjà utilisée devenue vide ou un lot qui supprimerait au moins 90 % des enregistrements locaux.

**Le chiffrement local SQLCipher ne remplace pas l'E2EE Joplin. Les cibles E2EE et les carnets partagés ne sont pas pris en charge et sont refusés, sans désactiver leur chiffrement.** Les notes HTML/riches ne sont pas prises en charge par cette synchronisation. Les cibles doivent être au format de synchronisation 3. Le traitement complet est limité à 10 000 enregistrements et 256 Mio de contenu par profil, avec un budget de temps de deux minutes par tentative ; relancer en cas d'interruption. L'interopérabilité générale et les cas limites nécessitent encore des essais.

## Vérifications et limites

Voir [VALIDATION.md](VALIDATION.md) et [SECURITY.md](SECURITY.md). Le binaire Windows a été compilé, mais n'a pas été exécuté sur un Windows réel ici. Il n'y a pas encore de mesure comparable de RAM/CPU contre Joplin, de certification de sécurité ni de garantie d'invulnérabilité.

Restent notamment : E2EE Joplin, extensions, Web Clipper, éditeur riche, OCR, dessin, imports ENEX/OneNote, exports HTML/PDF et notifications. La version 0.4.1 est une étape fonctionnelle ; elle ne doit pas être présentée comme une réécriture complète achevée.

## Compiler

Rust 1.95+ (validation avec 1.99.0), outils C++ Microsoft et Perl pour OpenSSL. Depuis le projet :

```powershell
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --release
```

Le programme est dans `target/release/CarnetDeNotes.exe`. Le workflow Windows construit un artefact sans publier de Release. Le script `tools/package.ps1` prépare le ZIP portable.

## Mes logiciels

[Profil public jeremdevgit](https://github.com/jeremdevgit) : Suite libre, Batailles de France, Pomme, Ludothèque, rJb, Boîte à outils et [JMeca](https://github.com/jeremdevgit/jmeca).

Code original GPL-3.0-or-later. Aucun code source, logo ou icône Joplin incorporé. Projet indépendant, sans affiliation à Joplin. Référence fonctionnelle : [Joplin Desktop v3.7.22](https://github.com/laurent22/joplin/releases/tag/v3.7.22). Police et dépendances : licences dans LICENSE, THIRD-PARTY-NOTICES.txt et assets/.
