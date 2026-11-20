<div align="center">

<img src="https://mistly.cc/assets/icone.png" alt="Mistly" width="110" />

# Mistly Launcher

**The Minecraft launcher, light as the mist.**

Written in Rust with a [Slint](https://slint.dev) interface, for Windows and Linux: a single program (`Misty.exe` / `Misty`), no runtime to install.

[![Website](https://img.shields.io/badge/website-mistly.cc-8ab4f8?style=flat-square)](https://mistly.cc/)
[![Version](https://img.shields.io/github/v/release/Mistly-MC/Mistly-Launcher?label=version&style=flat-square)](https://github.com/Mistly-MC/Mistly-Launcher/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/Mistly-MC/Mistly-Launcher/total?style=flat-square)](https://github.com/Mistly-MC/Mistly-Launcher/releases)
[![Rust](https://img.shields.io/badge/written%20in-Rust-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![Slint](https://img.shields.io/badge/UI-Slint-2379F4?style=flat-square)](https://slint.dev)
[![Windows](https://img.shields.io/badge/Windows-10%20%7C%2011-0078D6?style=flat-square&logo=windows)](https://mistly.cc/#download)
[![Linux](https://img.shields.io/badge/Linux-x86__64-FCC624?style=flat-square&logo=linux&logoColor=black)](https://mistly.cc/#download)
[![Discord](https://img.shields.io/badge/Discord-join-5865F2?style=flat-square&logo=discord&logoColor=white)](https://discord.gg/rdQJUgvUh3)
[![License](https://img.shields.io/badge/license-Misty%20(no%20resale)-blue?style=flat-square)](LICENSE.txt)

[**Download**](https://mistly.cc/#download) · [**Docs**](https://mistly.cc/docs) · [**Patch notes**](https://mistly.cc/patch-notes) · [**Releases**](https://github.com/Mistly-MC/Mistly-Launcher/releases) · [**Discord**](https://discord.gg/rdQJUgvUh3) · [**Website source**](https://github.com/Mistly-MC/Mistly-Web)

<img src="https://raw.githubusercontent.com/Mistly-MC/Mistly-Web/main/public/assets/screens/en/home.png" alt="Mistly home screen" width="800" />

</div>

## 📥 Installation

Download `Misty-Setup-<version>.exe` from the [latest release](https://github.com/Mistly-MC/Mistly-Launcher/releases/latest) and run it.

- No admin rights needed: Mistly installs into `%LOCALAPPDATA%\Programs\Misty`.
- Nothing else to install: Java is downloaded automatically when the game needs it.
- **Portable version**: download the zip, extract it and run `Misty-<version>/Misty.exe`.

On Linux, see [Linux support](#-linux-support) below.

Updates install themselves afterwards.

## 🐧 Linux support

Mistly runs natively on Linux since 0.4.0, from the same code as the Windows version.

- **Systems:** x86_64 with glibc 2.35 or newer: Ubuntu 22.04, Debian 12, Fedora 36, Linux Mint 21 and later, and the distributions based on them.
- **Install:** download `Misty-<version>-linux-x86_64.tar.gz` from [mistly.cc](https://mistly.cc/#download) or the [latest release](https://github.com/Mistly-MC/Mistly-Launcher/releases/latest), extract it where you want to keep it (`tar -xzf Misty-<version>-linux-x86_64.tar.gz`) and run `Misty-<version>/Misty`. On first start it adds itself to your applications menu.
- **Needs:** GTK 3 for the file dialogs, and a keyring (GNOME Keyring or KWallet) to keep your Microsoft sign-in safe. Without one, the sign-in is kept in `msa.json` in the data folder, like before 0.4.0.
- **Microsoft sign-in:** opens in your web browser. Once signed in, the page turns blank: copy its full address (it starts with `https://login.live.com/oauth20_desktop.srf?code=`) and paste it into Mistly.
- **Data:** `~/.local/share/Misty` (versions, profiles, settings). Java is downloaded for Linux automatically, like on Windows.
- **Updates:** install themselves, checked with the same SHA-256 and signature as on Windows.
- **Uninstall:** delete the extracted folder and `~/.local/share/applications/mistly.desktop`; delete `~/.local/share/Misty` to remove the game data too.
- **Not yet:** macOS, ARM (aarch64) and 32-bit systems.

To build it yourself: `installer/build-linux.sh` (the packages it needs are listed at the top of the script).

## 🗂️ Project structure

```
Cargo.toml             Cargo workspace (version, shared dependencies, build profiles)
CHANGELOG.md           patch notes (embedded in the exe)
curseforge.key         CurseForge API key (private, ignored by git)
installer/             release script (build.ps1) + Inno Setup installer (Misty.iss)
.github/workflows/     CI (format, clippy, tests) and signed releases from a version tag
misty-client/          Mistly Client (submodule → Mistly-MC/Mistly-Client): the in-game Fabric mod, used by the tests
crates/
├── misty-core/        all the logic, no UI and no Slint (cargo test -p misty-core)
│   ├── build.rs       embeds the CurseForge key
│   ├── lang/          translations (en, fr, es, it, pt, ru), embedded in the exe
│   └── src/
│       ├── auth/      Microsoft accounts, token vault (Windows Credential Manager, Linux keyring)
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

User data lives in `%LOCALAPPDATA%\Misty\` (`~/.local/share/Misty` on Linux) (`reglages.json` settings, `msa.json` Microsoft accounts (names only: the sign-in tokens are kept in the Windows Credential Manager, or the keyring on Linux, one `msa-<uuid>.Misty` entry per account), `instances/<id>/` profiles, plus versions, libraries, assets and Java). The old `data.txt` settings file is converted to `reglages.json` on first start and kept as `data.txt.old`. To add a setting, add a field with `#[serde(default)]` to `Reglages` in `crates/misty-core/src/systeme/reglages.rs`.

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
