//! Mistly Launcher : toute la logique, sans interface ni Slint (l'interface est dans misty-app).
//!
//! - `auth/`      : comptes Microsoft et coffre des jetons
//! - `minecraft/` : installation et lancement du jeu, mod loaders, profils, Mistly Client
//! - `contenu/`   : Modrinth, CurseForge, mods, modpacks, export, partage
//! - `social/`    : amis, présence, Discord, pseudos hors ligne, Horizon
//! - `systeme/`   : chemins, réglages, stockage, mises à jour, journal, traductions, réseau…
//! - `media/`     : skins, musique en jeu, catégories de textures

pub mod auth;
pub mod contenu;
pub mod media;
pub mod minecraft;
pub mod social;
pub mod systeme;

#[cfg(test)]
mod tests;

/// Fonction de progression (texte affiché à l'utilisateur), utilisable depuis n'importe quel thread.
pub type Progress = std::sync::Arc<dyn Fn(String) + Send + Sync>;
