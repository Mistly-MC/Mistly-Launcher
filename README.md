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
CHANGELOG.md           patch notes (embedded in the exe)
installer/             release script (build.ps1) + Inno Setup installer (Misty.iss)
misty-client/          Mistly Client (submodule → Misty-MC/Misty-Client): the in-game Fabric mod, used by the tests
misty-rust/
├── build.rs           embeds the icon, CurseForge key and changelog
├── misty-ui/          crate that compiles the Slint interface (kept apart so Rust edits rebuild fast)
├── assets/            icon, Silkscreen pixel font (OFL), flags
├── lang/              translations (en, fr, es, it, ru), embedded in the exe
├── ui/                Slint interface (theme, widgets, state, one file per page)
└── src/
    ├── main.rs        startup
    ├── ms_login.rs    Microsoft sign-in window (subprocess "Misty.exe --ms-login")
    ├── ui/            code behind each page
    ├── minecraft/     game install and launch (versions, Java, loaders)
    └── services/      logic without UI (profiles, content, Modrinth, CurseForge,
                       modpacks, skins, storage, updates, settings, Discord…)
```

The version comes from `misty-rust/Cargo.toml` only: the app and `installer/build.ps1` both read it there, and the release tag (without `v`) must match it.

User data lives in `%LOCALAPPDATA%\Misty\` (`data.txt` settings, `msa.json` Microsoft account, `instances/<id>/` profiles, plus versions, libraries, assets and Java). When a key is added to or removed from `data.txt`, bump `DATA_VERSION` in `app_info.rs` and add the key to `services/settings.rs`.

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
