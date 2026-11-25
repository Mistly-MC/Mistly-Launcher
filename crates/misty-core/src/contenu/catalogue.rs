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
//! Recherche de contenu, quelle que soit la plateforme (Modrinth ou CurseForge) : chaque appel
//! passe par la source correspondante (voir `source::SourceContenu`).

use crate::contenu::source::{source, Recherche};
use crate::minecraft::instances::Instance;
use crate::systeme::loc;

pub const MODRINTH: &str = "modrinth";
pub const CURSEFORGE: &str = "curseforge";

pub fn libelle_source(nom: &str) -> &'static str {
    source(nom).libelle()
}

/// Un projet (mod, pack, modpack…) trouvé sur Modrinth ou CurseForge, tel qu'affiché dans l'explorateur.
#[derive(Clone, Debug, Default)]
pub struct ProjetDistant {
    pub source: String,
    /// ID du projet sur sa plateforme (numérique pour CurseForge).
    pub id: String,
    pub titre: String,
    pub description: String,
    pub auteur: String,
    pub telechargements: i64,
    pub icone_url: Option<String>,
    pub categories: Vec<String>,
}

/// Tris proposés par la plateforme : (clé API, libellé traduit).
pub fn tris(nom: &str) -> Vec<(String, String)> {
    source(nom)
        .tris()
        .iter()
        .map(|(cle, libelle)| (cle.to_string(), loc::t(libelle)))
        .collect()
}

/// `instance` : profil cible (filtre version + loader), ou None pour les modpacks.
pub async fn rechercher(
    nom: &str,
    requete: &str,
    type_: &str,
    instance: Option<&Instance>,
    tri: &str,
    offset: usize,
    limite: usize,
) -> anyhow::Result<(Vec<ProjetDistant>, i64)> {
    source(nom)
        .rechercher(Recherche {
            requete,
            type_,
            instance,
            tri,
            offset,
            limite,
        })
        .await
}
