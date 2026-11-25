# Mistly Launcher — le code

Le launcher réécrit en Rust, avec l'interface en [Slint](https://slint.dev). Il reprend toutes
les fonctions de l'ancienne version C# / WPF et **les mêmes fichiers de données**
(`%LOCALAPPDATA%\Misty` : `data.txt`, converti en `reglages.json` au premier démarrage, `listpseudo.txt`, `instances\…\instance.json`,
`misty-contenu.json`, versions, bibliothèques, Java…). Rien n'est à réinstaller.

## Compiler et lancer

Depuis la racine du repo (workspace Cargo) :

```powershell
cargo run                  # version de développement (avec console pour voir les erreurs)
cargo test                 # tests hors ligne de misty-core et misty-app
cargo test -p misty-core   # la logique seule, sans compiler Slint
cargo build --release      # exe final : target\release\Misty.exe
```

La clé API CurseForge est lue à la compilation depuis `curseforge.key` à la racine du repo
(privée, ignorée par git), ou à l'exécution depuis la variable d'environnement
`MISTY_CURSEFORGE_KEY`.

Tests qui utilisent Internet et votre dossier Misty (ils ne lancent jamais le jeu) :

```powershell
cargo test -- --ignored --nocapture
```

## Organisation

```
misty-core/         la logique, sans interface ni Slint
  build.rs          intègre la clé CurseForge
  lang/             traductions (fr, en, es, it, pt, ru), intégrées à l'exe
  assets/steve.png  skin par défaut (media::skins::STEVE_PNG)
  src/
    auth/           compte Microsoft (account), coffre des jetons (coffre)
    minecraft/      installation et lancement du jeu (remplace CmlLib), profils (instances),
                    Mistly Client (client)
      install.rs    JSON de version, jar, bibliothèques, assets
      java.rs       Java fourni par Mojang
      launch.rs     ligne de commande Java
      loaders/      Forge, NeoForge, Fabric/Quilt, OptiFine, LiteLoader
    contenu/        Modrinth, CurseForge, catalogue, contenu des profils, modpacks, export,
                    partage des serveurs entre profils
    social/         amis, présence, Discord, pseudos hors ligne, Horizon
    systeme/        chemins, réglages, stockage, mises à jour, lanceur fixe, raccourcis,
                    télémétrie, infos système, journal, traductions, HTTP, téléchargements
    media/          skins (skins/ : fichiers, rendu, studio), musique en jeu
misty-ui/           l'interface Slint : à part, pour qu'une modification du code Rust ne
                    recompile pas les ~20 Mo de Rust générés par Slint (4 s au lieu de 20)
  ui/
    theme.slint     couleurs, polices, traductions (Tr.t("Cle"))
    widgets.slint   boutons, listes déroulantes, interrupteurs, curseurs…
    state.slint     état de chaque page, partagé avec Rust (les « globals »)
    main.slint      fenêtre, barre latérale, boîte de dialogue
    home / profiles / instance / browser / settings / changelog / friends / console .slint
misty-setup/        la fenêtre de l'installateur (Mistly-Setup.exe), qui contient l'installateur
                    Inno Setup (installer/Misty.iss) et le lance sans fenêtre
misty-app/          l'application : produit Misty.exe
  build.rs          icône et infos de l'exe
  assets/           polices, drapeaux, icône, curseurs
  src/
    main.rs         démarrage
    ui/             le code derrière chaque page (l'équivalent des .xaml.cs)
```

## Correspondance avec la version C#

| C#                              | Rust                                   |
|---------------------------------|----------------------------------------|
| `async void` + `Dispatcher`     | `ui::lancer(async { … })` (thread de l'interface) |
| `await Task.Run(…)` / HttpClient | `app.bg(async { … })` (runtime tokio)  |
| `Loc.T("Cle")`, `{loc:Tr Cle}`  | `loc::t("Cle")`, `Tr.t("Cle")` en Slint |
| `Dialogs.ConfirmerAsync`        | `dialogs::confirmer(&app, …).await`    |
| `ObservableCollection`          | `VecModel` / `ModelRc`                 |
| CmlLib.Core                     | `misty-core`, `minecraft/`             |
| CmlLib.Core.Auth.Microsoft      | `misty-core`, `auth/account.rs` (crates `oauth2` et `minecraft-msa-auth`) |

## Ce qui a changé

- Le compte Microsoft : le cache de CmlLib n'est pas réutilisable, il faut se reconnecter une fois
  (le compte est ensuite gardé dans `msa.json`, ses jetons dans le Gestionnaire d'identifiants
  Windows).
- La fenêtre est transparente et son contenu est découpé en coins arrondis (identique sur Windows 10 et 11).
- L'éditeur externe (Paint.NET…) : la texture est rechargée dès que son fichier change.
