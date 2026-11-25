//  ███╗   ███╗██╗███████╗████████╗██╗     ██╗   ██╗
//  ████╗ ████║██║██╔════╝╚══██╔══╝██║     ╚██╗ ██╔╝
//  ██╔████╔██║██║███████╗   ██║   ██║      ╚████╔╝
//  ██║╚██╔╝██║██║╚════██║   ██║   ██║       ╚██╔╝
//  ██║ ╚═╝ ██║██║███████║   ██║   ███████╗   ██║
//  ╚═╝     ╚═╝╚═╝╚══════╝   ╚═╝   ╚══════╝   ╚═╝
//
//  Mistly Launcher · https://mistly.cc
//  Code : https://github.com/Mistly-MC/Mistly-Launcher
//  Discord : https://discord.gg/rdQJUgvUh3
//  Misty License 1.0 : free to use, modify and share, not to sell (LICENSE.txt)
//
//! Constantes globales de l'application.

/// Version du launcher, lue dans Cargo.toml (la seule source de la version). Correspond au tag de
/// la release GitHub (sans le "v") et au nom du dossier d'installation "Misty-{VERSION}".
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GITHUB_REPO: &str = "Mistly-MC/Mistly-Launcher";

/// Clé publique minisign des releases (deuxième ligne de `misty.pub`). La clé privée n'existe que
/// dans le secret GitHub `MINISIGN_KEY`, utilisé par `.github/workflows/release.yml`. La mise à
/// jour automatique refuse tout zip qui n'est pas signé par cette clé (misty-core, systeme/update.rs).
pub const CLE_PUBLIQUE_MAJ: &str = "RWS9VUZ3skhhf6tZwpMWvwWzIKpGFXf9Q2+xM/8BgaBNzHfWSPxCzwSL";
pub const DISCORD_APP_ID: &str = "1536551827640680528";
/// Invitation au serveur Discord de Mistly (tuile Discord en haut du launcher).
pub const DISCORD_INVITE: &str = "https://discord.gg/rdQJUgvUh3";
pub const RAM_PAR_DEFAUT_MO: u32 = 4096;

/// User-Agent demandé par Modrinth : "auteur/projet/version (contact)".
pub fn user_agent() -> String {
    format!("{GITHUB_REPO}/{VERSION} (github.com/{GITHUB_REPO})")
}
