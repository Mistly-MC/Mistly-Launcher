//! Constantes globales de l'application.

/// Version du launcher, lue dans Cargo.toml (la seule source de la version). Correspond au tag de
/// la release GitHub (sans le "v") et au nom du dossier d'installation "Misty-{VERSION}".
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Version du format de data.txt. À incrémenter quand on ajoute/retire une clé.
pub const DATA_VERSION: &str = "1.3.5";
pub const GITHUB_REPO: &str = "Misty-MC/Misty-Launcher";

/// Clé publique minisign des releases (deuxième ligne de `misty.pub`). La clé privée n'existe que
/// dans le secret GitHub `MINISIGN_KEY`, utilisé par `.github/workflows/release.yml`. La mise à
/// jour automatique refuse tout zip qui n'est pas signé par cette clé (services/update.rs).
pub const CLE_PUBLIQUE_MAJ: &str = "RWS9VUZ3skhhf6tZwpMWvwWzIKpGFXf9Q2+xM/8BgaBNzHfWSPxCzwSL";
pub const DISCORD_APP_ID: &str = "1536551827640680528";
pub const RAM_PAR_DEFAUT_MO: u32 = 4096;

/// User-Agent demandé par Modrinth : "auteur/projet/version (contact)".
pub fn user_agent() -> String {
    format!("{GITHUB_REPO}/{VERSION} (github.com/{GITHUB_REPO})")
}
