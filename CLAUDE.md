# Misty Launcher

Launcher Minecraft pour Windows, écrit en Rust avec une interface Slint. Repo : Misty-MC/Misty-Launcher.
Le mod en jeu (Mistly Client) vient du repo Misty-MC/Misty-Client (il a son propre `CLAUDE.md`). Le launcher le télécharge depuis les releases GitHub de ce repo (`crates/misty-core/src/minecraft/client.rs`) ; le sous-module `misty-client/` ne sert plus qu'aux tests (textures de cape partagées).

## Structure

Workspace Cargo à la racine du repo (détail dans `crates/README.md`) :
- `crates/misty-core` : toute la logique, **sans Slint ni interface** (`auth/`, `minecraft/`, `contenu/`, `social/`, `systeme/`, `media/`). `cargo test -p misty-core` doit compiler sans Slint : ne jamais y ajouter de dépendance à `slint`, `misty-ui` ou une fenêtre.
- `crates/misty-ui` : les fichiers `.slint` (`ui/`), compilés à part.
- Dossier de données, client HTTP et runtime : `misty_core::systeme::contexte` (créé et installé dans `main`, lu par `paths::data_dir()` et `http::client()`). Un test qui touche au dossier de données prend `let _garde = contexte::pour_test(&dossier_temporaire);` : jamais les vraies données du joueur, sans variable d'environnement.
- Erreurs de `misty-core` : une enum `thiserror` par domaine (`AuthErreur`, `InstallErreur`, `ContenuErreur`, `MajErreur`) avec `cle()` (clé de traduction) ; `misty-app` garde `anyhow`.
- `crates/misty-app` : `main.rs`, `ms_login.rs`, `ui/*.rs` (le code des pages), `assets/` ; produit `Misty.exe`.

## Cloner et compiler

```powershell
git clone --recursive https://github.com/Misty-MC/Misty-Launcher.git   # ou : git submodule update --init
cargo check --all-targets   # vérification rapide (depuis la racine du repo)
cargo test                  # tests hors ligne (doivent passer avant chaque commit)
cargo test -- --ignored --nocapture   # tests qui utilisent Internet et le dossier Misty
cargo run                   # lancer le launcher
```

- Le mod n'est pas dans l'exe. Pour essayer un mod compilé en local : `MISTY_CLIENT_DIR=<dossier du mod>` au lancement (c'est ce que fait « Lancer Misty.cmd » à la racine de l'espace de travail), sinon le launcher prend la dernière release de Misty-Client.
- Windows uniquement : WebView2, registre, raccourcis. La fenêtre de connexion Microsoft (`crates/misty-app/src/ms_login.rs`, winit + wry) n'existe que sous `cfg(windows)`.
- CI (`.github/workflows/ci.yml`) à chaque push et PR : `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`. Un avertissement clippy fait échouer la CI.
- Release : pousser un tag `x.y.z` (sans `v`, égal à la version de `Cargo.toml` ; `x.y.z-beta.n` pour une pré-release). `.github/workflows/release.yml` lance `installer\build.ps1`, signe le zip (`.sha256` + `.minisig` avec le secret `MINISIGN_KEY`) et publie la release. Toujours demander avant de pousser un tag : la release part aussitôt chez les joueurs. `installer\build.ps1` reste utilisable en local, mais un zip sans `.sha256` ni `.minisig` est refusé par la mise à jour automatique.

## Conventions du code

- **Identifiants et commentaires en français** (`executer`, `fenetre`, `dossier`…), comme le reste du code. Garder ce style.
- Textes de l'interface : jamais en dur. Ajouter la clé dans **les 6 fichiers** `crates/misty-core/lang/*.json` (en, fr, es, it, pt, ru), puis l'utiliser avec `loc::t("Cle")`.
- Nouveau réglage : ajouter un champ à `Reglages` (`crates/misty-core/src/systeme/reglages.rs`) avec `#[serde(default)]` (ou `#[serde(default = "…")]`), puis `reglages::lire().champ` / `reglages::modifier(|r| r.champ = …)`. Pas de version à incrémenter : un ancien `reglages.json` reçoit la valeur par défaut. Ne jamais renommer ni changer le type d'un champ existant sans migration (sinon le joueur perd ce réglage).
- Ne pas modifier `GITHUB_REPO` (`app_info.rs`) ni l'`AppId` de `installer/Misty.iss` : les mises à jour automatiques et l'installateur en dépendent.
- Le nom affiché est « Mistly Launcher » (et « Mistly Client » pour le mod ; l'app s'appelait Misty), mais l'exe (`Misty.exe`) et les dossiers de données (`%LOCALAPPDATA%\Misty`, `%LOCALAPPDATA%\Programs\Misty`) gardent le nom « Misty » : les renommer casserait les installations existantes. Les raccourcis (menu Démarrer, bureau) et les infos de l'exe (`build.rs`) affichent « Mistly ». Gardent « Misty » : l'`AppId`, l'AUMID, le domaine `misty-launcher.online`, les repos `Misty-MC/*` et le nom de la licence (« Misty License 1.0 »).
- `curseforge.key` (racine du repo, lu par `crates/misty-core/build.rs`) est privé et ignoré par git : ne jamais le commiter.

## Commits et branches

- Branches : `main` (publiée), `EnderMythex` et `xtheox` (branches perso). Travailler sur sa branche, PR vers `main`. Demander avant tout push ou merge sur `main`.
- Message : sujet en anglais à l'impératif, sans point final (`Add …`, `Fix …`, `Keep …`), corps optionnel qui explique pourquoi. Terminer par `Co-Authored-By: Claude <noreply@anthropic.com>` quand Claude a écrit le changement.
- Tout changement visible par l'utilisateur va dans `CHANGELOG.md`, en anglais, sous la version en cours (`## <version>` puis `### Launcher` ou `### Mistly Client (in game)`). Ce fichier est intégré dans l'exe et affiché dans « Patch notes ».
- Mise à jour du mod pour les joueurs : publier une release de Misty-Client (son workflow y attache les jars et `misty-client.json`). Le sous-module se met à jour à part (`git submodule update --remote misty-client`), seulement pour les tests.
- Version : la seule source est `version` dans `[workspace.package]` du `Cargo.toml` racine (`app_info::VERSION` la lit avec `env!("CARGO_PKG_VERSION")`, `installer/build.ps1` la lit dans `Cargo.toml`). Le tag GitHub (sans `v`) doit être identique, sinon le launcher propose la mise à jour en boucle.

## Dépendances

Dependabot ouvre des PR chaque semaine (cargo + sous-module). Pour les traiter : les merger ensemble dans une branche locale, puis `cargo check --all-targets` et `cargo test`, et pousser seulement si tout passe. Un bump 0.x → 0.y peut casser l'API.
