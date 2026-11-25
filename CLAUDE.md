# Mistly Launcher

Launcher Minecraft pour Windows et Linux, écrit en Rust avec une interface Slint. Repo : Mistly-MC/Mistly-Launcher. Site : https://mistly.cc.
Le mod en jeu (Mistly Client) vient du repo Mistly-MC/Mistly-Client (il a son propre `CLAUDE.md`). Le launcher le télécharge depuis les releases GitHub de ce repo (`crates/misty-core/src/minecraft/client.rs`) ; le sous-module `misty-client/` ne sert plus qu'aux tests (textures de cape partagées).

## Structure

Workspace Cargo à la racine du repo (détail dans `crates/README.md`) :
- `crates/misty-core` : toute la logique, **sans Slint ni interface** (`auth/`, `minecraft/`, `contenu/`, `social/`, `systeme/`, `media/`). `cargo test -p misty-core` doit compiler sans Slint : ne jamais y ajouter de dépendance à `slint`, `misty-ui` ou une fenêtre.
- `crates/misty-ui` : les fichiers `.slint` (`ui/`), compilés à part.
- Connexion Microsoft (`misty-core/src/auth/account.rs`) : code d'appareil saisi sur microsoft.com/link (crate `oauth2`), puis Xbox Live → Minecraft (crate `minecraft-msa-auth`, avec la chaîne Xbox du launcher en secours pour les erreurs exactes). Même chose sous Windows et Linux, sans WebView ; la fenêtre du code est dans `misty-ui/ui/accueil.slint`.
- Dossier de données, client HTTP et runtime : `misty_core::systeme::contexte` (créé et installé dans `main`, lu par `paths::data_dir()` et `http::client()`). Un test qui touche au dossier de données prend `let _garde = contexte::pour_test(&dossier_temporaire);` : jamais les vraies données du joueur, sans variable d'environnement.
- Erreurs de `misty-core` : une enum `thiserror` par domaine (`AuthErreur`, `InstallErreur`, `ContenuErreur`, `MajErreur`) avec `cle()` (clé de traduction) ; `misty-app` garde `anyhow`.
- `crates/misty-app` : `main.rs`, `ui/*.rs` (le code des pages), `assets/` ; produit `Misty.exe`.

## Cloner et compiler

```powershell
git clone --recursive https://github.com/Mistly-MC/Mistly-Launcher.git   # ou : git submodule update --init
cargo check --all-targets   # vérification rapide (depuis la racine du repo)
cargo test                  # tests hors ligne (doivent passer avant chaque commit)
cargo test -- --ignored --nocapture   # tests qui utilisent Internet et le dossier Misty
cargo run                   # lancer le launcher
```

- Le mod n'est pas dans l'exe. Pour essayer un mod compilé en local : `MISTY_CLIENT_DIR=<dossier du mod>` au lancement (c'est ce que fait « Lancer Misty.cmd » à la racine de l'espace de travail), sinon le launcher prend la dernière release de Misty-Client.
- Windows et Linux (x86_64). Ce qui dépend du système est derrière `cfg(windows)` / `cfg(not(windows))` : coffre des jetons, raccourcis (.lnk / .desktop), lanceur fixe, Java et natives de Mojang (`minecraft/mod.rs` : `OS_MOJANG`…). Nom de l'exe : `paths::NOM_EXE` (`Misty.exe` / `Misty`), à ne pas changer.
- Linux en local : `installer/build-linux.sh` (paquets nécessaires en tête du script). Une release contient une seule archive en `.zip` (Windows) : les launchers d'avant 0.4 prennent le premier `.zip` venu.
- CI (`.github/workflows/ci.yml`) à chaque push et PR : `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`. Un avertissement clippy fait échouer la CI.
- Release : pousser un tag `x.y.z` (sans `v`, égal à la version de `Cargo.toml` ; `x.y.z-beta.n` pour une pré-release). Sans pouvoir pousser de tag (session Claude Code dans le cloud, par exemple) : bouton « Run workflow » du workflow Release sur `main`, avec la version (il refuse une autre branche, un tag déjà existant ou une version différente de `Cargo.toml`). `.github/workflows/release.yml` compile sous Windows (`installer\build.ps1`) et sous Linux (`installer/build-linux.sh`), signe les deux archives (`.sha256` + `.minisig` avec le secret `MINISIGN_KEY`) et publie une seule release, avec le secret `RELEASE_TOKEN` (jeton d'un administrateur : le ruleset des tags bloque le jeton de GitHub Actions). Les deux secrets sont dans l'environnement GitHub `release`, qui demande une validation manuelle (« Review deployments ») et n'accepte que `main` et les tags `*.*.*`. Toujours demander avant de pousser un tag : la release part aussitôt chez les joueurs. `installer\build.ps1` reste utilisable en local, mais un zip sans `.sha256` ni `.minisig` est refusé par la mise à jour automatique.

## Conventions du code

- **Identifiants et commentaires en français** (`executer`, `fenetre`, `dossier`…), comme le reste du code. Garder ce style.
- Textes de l'interface : jamais en dur. Ajouter la clé dans **les 6 fichiers** `crates/misty-core/lang/*.json` (en, fr, es, it, pt, ru), puis l'utiliser avec `loc::t("Cle")`.
- Nouveau réglage : ajouter un champ à `Reglages` (`crates/misty-core/src/systeme/reglages.rs`) avec `#[serde(default)]` (ou `#[serde(default = "…")]`), puis `reglages::lire().champ` / `reglages::modifier(|r| r.champ = …)`. Pas de version à incrémenter : un ancien `reglages.json` reçoit la valeur par défaut. Ne jamais renommer ni changer le type d'un champ existant sans migration (sinon le joueur perd ce réglage).
- Ne pas modifier `GITHUB_REPO` (`app_info.rs`, `Mistly-MC/Mistly-Launcher`) ni l'`AppId` de `installer/Misty.iss` : les mises à jour automatiques et l'installateur en dépendent.
- L'organisation GitHub s'appelait `Misty-MC` (et les repos `Misty`, `Misty-Launcher`, `Misty-Client`) : GitHub redirige ces anciens noms vers `Mistly-MC/*`, et c'est par cette redirection que les launchers déjà installés trouvent leurs mises à jour. **Ne jamais créer d'organisation ni de repo sous ces anciens noms** : la redirection s'arrêterait.
- Le nom affiché est « Mistly Launcher » (et « Mistly Client » pour le mod ; l'app s'appelait Misty), mais l'exe (`Misty.exe`) et les dossiers de données (`%LOCALAPPDATA%\Misty`, `%LOCALAPPDATA%\Programs\Misty`) gardent le nom « Misty » : les renommer casserait les installations existantes. Les raccourcis (menu Démarrer, bureau) et les infos de l'exe (`build.rs`) affichent « Mistly ». Gardent « Misty » : l'`AppId`, l'AUMID et le nom de la licence (« Misty License 1.0 »). Le site est https://mistly.cc (l'ancien domaine `misty-launcher.online` sert encore l'API et les téléchargements).
- `curseforge.key` (racine du repo, lu par `crates/misty-core/build.rs`) est privé et ignoré par git : ne jamais le commiter.

## Commits et branches

- Branches : seule `main` existe (publiée). Pour un changement : créer une branche temporaire, ouvrir une PR vers `main`, puis supprimer la branche après le merge. Toujours demander avant de pousser sur `main`.
- Message : sujet en anglais à l'impératif, sans point final (`Add …`, `Fix …`, `Keep …`), corps optionnel qui explique pourquoi. Terminer par `Co-Authored-By: Claude <noreply@anthropic.com>` quand Claude a écrit le changement.
- Tout changement visible par l'utilisateur va dans `CHANGELOG.md`, en anglais, sous la version en cours (`## <version>` puis `### Launcher` ou `### Mistly Client (in game)`). Ce fichier est intégré dans l'exe et affiché dans « Patch notes ».
- Mise à jour du mod pour les joueurs : publier une release de Mistly-Client (son workflow y attache les jars et `misty-client.json`). Le sous-module se met à jour à part (`git submodule update --remote misty-client`), seulement pour les tests.
- Version : la seule source est `version` dans `[workspace.package]` du `Cargo.toml` racine (`app_info::VERSION` la lit avec `env!("CARGO_PKG_VERSION")`, `installer/build.ps1` la lit dans `Cargo.toml`). Le tag GitHub (sans `v`) doit être identique, sinon le launcher propose la mise à jour en boucle.

## Dépendances

Dependabot ouvre des PR chaque semaine (cargo + sous-module). Pour les traiter : les merger ensemble dans une branche locale, puis `cargo check --all-targets` et `cargo test`, et pousser seulement si tout passe. Un bump 0.x → 0.y peut casser l'API.
