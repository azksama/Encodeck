# Encodeck

Application de bureau FFmpeg en **Rust + Tauri 2**, avec une interface **React + daisyUI 5**.

## Développement

Prérequis : Node.js 22+, Rust stable et les [prérequis Tauri](https://v2.tauri.app/start/prerequisites/) de la plateforme.

```sh
npm ci
npm run desktop
```

`npm run dev` ouvre uniquement l’aperçu web ; les opérations natives sont désactivées dans cet aperçu.

## Fonctionnalités

- Générateur complet des options du site de référence : conteneurs, extraits, codecs, débits, CRF, une/deux passes, profils, redimensionnement, filtres vidéo et audio, options avancées.
- 34 presets intégrés ; presets personnels nommés, mise à jour, import/export JSON.
- Commandes transmises à Rust sous forme de tableaux d’arguments, sans shell ; éditeur JSON avancé.
- File d’attente séquentielle, annulation, progression, logs et historique conservés sur disque. Les tâches interrompues ne redémarrent pas sans demande.
- Analyse ffprobe et preset automatique basé sur le média, sans agrandissement automatique.
- Glisser-déposer des médias et sous-titres ; vidéos supplémentaires proposées en lot ; sous-titres intégrés en piste MP4/MKV.
- Palette d’actions `Ctrl+Shift+P` (`Cmd+Shift+P` fonctionne également), navigation avec flèches/Entrée/Échap.
- Barre de fenêtre intégrée, scrollbar personnalisée confinée au contenu, menus déroulants sous les champs.
- Onboarding en six étapes ; paramètres centralisés : apparence, FFmpeg, mises à jour et langues.
- Logo SVG fourni par le propriétaire, texte adapté au thème, icônes Windows/macOS/Linux dérivées du PNG fourni.

## FFmpeg et versions

Windows x64 : builds Gyan récupérés depuis GitHub, SHA-256 comparé à l’empreinte publiée.

macOS Intel/Apple Silicon et Linux x64/ARM64 : binaires FFmpeg/ffprobe de Descript ; comparaison de l’empreinte GitHub lorsqu’elle existe, sinon téléchargement HTTPS avec empreinte locale enregistrée (ce dernier cas n’est pas une vérification indépendante du fournisseur).

Une installation locale peut également être importée. `ffprobe` doit être situé à côté de `ffmpeg`.
Chaque tâche conserve le chemin du moteur choisi lors de son ajout.
Une version utilisée ne peut pas être retirée ; les installations retirées sont déplacées dans le sous-dossier `trash` des données applicatives.

Les fichiers existants sont protégés par défaut (`-n`). L’option d’écrasement doit être cochée explicitement.
Un encodage annulé peut laisser un fichier partiel ; il n’est pas supprimé automatiquement.

Les installations FFmpeg restent séparées de l’application et conservent leur propre licence.
Les capacités d’encodage matériel dépendent de la machine et du build FFmpeg choisi.

## Traductions

Les catalogues se trouvent dans `src/locales`.
Ils sont générés et intégrés à l’application pour fonctionner hors ligne ; aucun média ni chemin utilisateur n’est envoyé pour les traductions.
La traduction automatique n’équivaut pas à une validation linguistique humaine.
La liste de langues vise une couverture internationale large ; un pourcentage exact de population n’est pas déduit par addition des locuteurs multilingues.
Les journaux FFmpeg et les commandes conservent leur texte technique d’origine.

## Builds

```sh
npm test
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri -- build
```

Le workflow GitHub compile et teste Windows x64, macOS Apple Silicon, macOS Intel et Linux x64.
Il fournit des installateurs `.exe`, `.dmg`, `.deb` et `.AppImage` dans les artefacts GitHub Actions.
Les packages ne sont pas signés avec des certificats commerciaux ou notariés Apple.

Pour le test d’intégration avec téléchargement réel de FFmpeg dans un dossier isolé :

```sh
cargo test --manifest-path src-tauri/Cargo.toml native_end_to_end -- --ignored --nocapture
```

Le test vérifie installation, analyse, encodage, deux passes, annulation, refus d’écrasement, échec FFmpeg et persistance de l’historique.

## Stockage et mises à jour

Les versions, presets et historiques sont dans le dossier de données Tauri `com.encodeck.desktop`.
Les préférences d’affichage et le brouillon du générateur utilisent le stockage local du webview.

Le bouton « Rechercher les mises à jour » consulte les releases de [azksama/Encodeck](https://github.com/azksama/Encodeck).
Les releases doivent être publiées pour que l’API `releases/latest` les expose.
Un dépôt privé nécessite une authentification ; aucun token du développeur n’est embarqué dans l’application.

## Origine et licence

Le modèle de formulaire, les presets et les contrôles techniques sont adaptés de [alfg/ffmpeg-commander](https://github.com/alfg/ffmpeg-commander), commit `2d9e3500e3905c4f827d0e26e7ce028bc85b50e2`, sous licence MIT (voir `LICENSE`).
La nouvelle interface et le moteur natif constituent Encodeck, une application distincte.
