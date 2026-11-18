<div align="center">

<img src="https://misty-launcher.online/assets/icone.png" alt="Mistly" width="110" />

# Mistly Launcher

**The Minecraft launcher, light as the mist.**

Written in Rust with a [Slint](https://slint.dev) interface: a single `Misty.exe`, no runtime to install.

[![Website](https://img.shields.io/badge/website-misty--launcher.online-8ab4f8?style=flat-square)](https://misty-launcher.online/)
[![Version](https://img.shields.io/github/v/release/Misty-MC/Misty-Launcher?label=version&style=flat-square)](https://github.com/Misty-MC/Misty-Launcher/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/Misty-MC/Misty-Launcher/total?style=flat-square)](https://github.com/Misty-MC/Misty-Launcher/releases)
[![Rust](https://img.shields.io/badge/written%20in-Rust-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![Slint](https://img.shields.io/badge/UI-Slint-2379F4?style=flat-square)](https://slint.dev)
[![Windows](https://img.shields.io/badge/Windows-10%20%7C%2011-0078D6?style=flat-square&logo=windows)](https://misty-launcher.online/)
[![License](https://img.shields.io/badge/license-Misty%20(no%20resale)-blue?style=flat-square)](LICENSE.txt)

[**Download**](https://misty-launcher.online/) · [**Docs**](https://misty-launcher.online/docs) · [**Patch notes**](https://misty-launcher.online/patch-notes) · [**Releases**](https://github.com/Misty-MC/Misty-Launcher/releases) · [**Website source**](https://github.com/Misty-MC/Misty-Web)

<img src="https://raw.githubusercontent.com/Misty-MC/Misty-Web/main/public/assets/screens/en/home.png" alt="Mistly home screen" width="800" />

</div>

## 📥 Installation

Download `Misty-Setup-<version>.exe` from the [latest release](https://github.com/Misty-MC/Misty-Launcher/releases/latest) and run it.

- No admin rights needed: Mistly installs into `%LOCALAPPDATA%\Programs\Misty`.
- Nothing else to install: Java is downloaded automatically when the game needs it.
- **Portable version**: download the zip, extract it and run `Misty-<version>/Misty.exe`.

Updates install themselves afterwards.

## 🗂️ Project structure

```
Cargo.toml             Cargo workspace (version, shared dependencies, build profiles)
CHANGELOG.md           patch notes (embedded in the exe)
curseforge.key         CurseForge API key (private, ignored by git)
installer/             release script (build.ps1) + Inno Setup installer (Misty.iss)
.github/workflows/     CI (format, clippy, tests) and signed releases from a version tag
misty-client/          Mistly Client (submodule → Misty-MC/Misty-Client): the in-game Fabric mod, used by the tests
crates/
├── misty-core/        all the logic, no UI and no Slint (cargo test -p misty-core)
│   ├── build.rs       embeds the CurseForge key
│   ├── lang/          translations (en, fr, es, it, pt, ru), embedded in the exe
│   └── src/
│       ├── auth/      Microsoft accounts, token vault (Windows Credential Manager)
│       ├── minecraft/ game install and launch (versions, Java, loaders), profiles, Mistly Client
│       ├── contenu/   Modrinth, CurseForge, mods, modpacks, texture packs, export, sharing
│       ├── social/    friends, presence, Discord, offline names, Horizon
│       ├── systeme/   paths, settings, storage, updates, logs, translations, HTTP, downloads
│       └── media/     skins, in-game music, texture categories
├── misty-ui/          Slint interface (ui/*.slint), its own crate so Rust edits rebuild fast
└── misty-app/         the app, builds Misty.exe
    ├── build.rs       embeds the icon and the changelog
    ├── assets/        icon, Silkscreen pixel font (OFL), flags, cursors
    └── src/
        ├── main.rs    startup
        ├── ms_login.rs  Microsoft sign-in window (subprocess "Misty.exe --ms-login")
        └── ui/        code behind each page
```

The version comes from the root `Cargo.toml` only (`[workspace.package]`): the app, `installer/build.ps1` and the release workflow all read it there, and the release tag (without `v`) must match it.

To publish a release, push a tag such as `0.4.0`: `.github/workflows/release.yml` builds the zip and the installer, signs the zip with minisign and publishes them. The launcher refuses an update whose signature or SHA-256 checksum doesn't match.

User data lives in `%LOCALAPPDATA%\Misty\` (`reglages.json` settings, `msa.json` Microsoft accounts (names only: the sign-in tokens are kept in the Windows Credential Manager, one `msa-<uuid>.Misty` entry per account), `instances/<id>/` profiles, plus versions, libraries, assets and Java). The old `data.txt` settings file is converted to `reglages.json` on first start and kept as `data.txt.old`. To add a setting, add a field with `#[serde(default)]` to `Reglages` in `crates/misty-core/src/systeme/reglages.rs`.

## 📄 License

The code is released under the [Misty License](LICENSE.txt): free to use, modify, share and contribute to, but it may not be sold. Versions published before this license remain under MIT.
The Silkscreen font is distributed under the SIL Open Font License.

---

<div align="center">
<sub>
Mistly is an independent launcher and is not affiliated with Mojang Studios or Microsoft.<br/>
No Minecraft files are shipped: the game is downloaded from Mojang's official servers on first launch.
</sub>
</div>
