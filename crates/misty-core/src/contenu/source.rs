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
//! Une plateforme de contenu (Modrinth, CurseForge) vue de la même façon par le reste du launcher :
//! recherche, détails d'un projet, versions compatibles avec un profil, téléchargement.
//!
//! `source(nom)` donne la plateforme d'un nom enregistré dans les métadonnées ("modrinth",
//! "curseforge"). Les implémentations sont dans `modrinth.rs` et `curseforge.rs`.

use crate::contenu::catalogue::{ProjetDistant, CURSEFORGE};
use crate::minecraft::instances::Instance;
use crate::systeme::download;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;

/// Résultat asynchrone d'une source (en boîte : le trait reste utilisable en `&dyn`).
pub type FutureSource<'a, T> = Pin<Box<dyn Future<Output = anyhow::Result<T>> + Send + 'a>>;

/// Une recherche dans l'explorateur.
#[derive(Clone, Copy)]
pub struct Recherche<'a> {
    pub requete: &'a str,
    /// mod, resourcepack, datapack, shader, modpack
    pub type_: &'a str,
    /// Profil cible (filtre version + loader), ou None pour les modpacks
    pub instance: Option<&'a Instance>,
    /// Clé de tri de la plateforme (voir `tris`)
    pub tri: &'a str,
    pub offset: usize,
    pub limite: usize,
}

/// Un projet, tel que l'installation en a besoin.
#[derive(Clone, Debug, Default)]
pub struct DetailsContenu {
    /// ID réel du projet (Modrinth accepte aussi un slug : c'est l'ID qui est gardé)
    pub id: String,
    pub titre: String,
    pub icone_url: Option<String>,
    /// Côtés client / serveur connus pour tout le projet (Modrinth)
    pub client_side: Option<String>,
    pub server_side: Option<String>,
    /// Site du projet (CurseForge), pour les liens de téléchargement manuel
    pub site: Option<String>,
}

/// Une dépendance requise par une version.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dependance {
    /// ID du projet (peut manquer sur Modrinth quand seule la version est donnée)
    pub projet: Option<String>,
    /// Version précise demandée
    pub version: Option<String>,
}

/// Une version (Modrinth) ou un fichier (CurseForge) d'un projet.
#[derive(Clone, Debug, Default)]
pub struct VersionContenu {
    /// ID de la version Modrinth ou du fichier CurseForge
    pub id: String,
    /// Projet auquel la version appartient ("" si inconnu)
    pub projet: String,
    /// Nom affiché ("1.2.3", "MonMod-1.2.3.jar"…)
    pub nom: String,
    pub nom_fichier: String,
    /// Adresse du fichier ; None : l'auteur interdit le téléchargement hors du site (CurseForge)
    pub url: Option<String>,
    pub sha1: Option<String>,
    /// Version stable (pas une bêta ni une alpha)
    pub stable: bool,
    /// Côtés client / serveur propres à ce fichier (CurseForge)
    pub client_side: Option<String>,
    pub server_side: Option<String>,
    /// Dépendances requises uniquement
    pub dependances: Vec<Dependance>,
}

/// Une plateforme de contenu.
pub trait SourceContenu: Send + Sync {
    /// Nom enregistré dans les métadonnées ("modrinth", "curseforge")
    fn nom(&self) -> &'static str;

    /// Nom affiché ("Modrinth", "CurseForge")
    fn libelle(&self) -> &'static str;

    /// Tris proposés : (clé API, clé de traduction)
    fn tris(&self) -> &'static [(&'static str, &'static str)];

    /// Recherche : les projets d'une page et le nombre total de résultats.
    fn rechercher<'a>(&'a self, r: Recherche<'a>) -> FutureSource<'a, (Vec<ProjetDistant>, i64)>;

    /// Détails d'un projet. Ok(None) : la plateforme ne le connaît pas, mais l'installation peut
    /// continuer (Modrinth) ; une plateforme qui en a besoin renvoie une erreur.
    fn details<'a>(&'a self, id: &'a str) -> FutureSource<'a, Option<DetailsContenu>>;

    /// Versions compatibles avec le profil (version de Minecraft, loader pour les mods), les plus
    /// récentes d'abord.
    fn versions<'a>(
        &'a self,
        id: &'a str,
        instance: &'a Instance,
        type_: &'a str,
    ) -> FutureSource<'a, Vec<VersionContenu>>;

    /// Une version précise (mise à jour, dépendance qui impose sa version). Ok(None) : elle
    /// n'existe pas ou n'est pas compatible avec le profil.
    fn version<'a>(
        &'a self,
        id: &'a str,
        version_id: &'a str,
        instance: &'a Instance,
        type_: &'a str,
    ) -> FutureSource<'a, Option<VersionContenu>>;

    /// Télécharge le fichier d'une version (vérifié avec son SHA-1 s'il est connu).
    fn telecharger<'a>(
        &'a self,
        version: &'a VersionContenu,
        destination: &'a Path,
    ) -> FutureSource<'a, ()> {
        Box::pin(async move {
            let url = version.url.as_deref().ok_or_else(|| {
                anyhow::anyhow!("{} : pas d'adresse de téléchargement", version.nom)
            })?;
            download::fichier(url, destination, version.sha1.as_deref()).await
        })
    }

    /// Clé de traduction de l'erreur « aucune version compatible » ({0} titre, {1} version de
    /// Minecraft, {2} loader)
    fn cle_pas_de_version(&self) -> &'static str;

    /// Page d'une version sur le site, pour un téléchargement manuel.
    fn page_version(
        &self,
        details: Option<&DetailsContenu>,
        id: &str,
        version: &VersionContenu,
    ) -> String;
}

/// La plateforme d'un nom de source (une source inconnue ou absente est Modrinth, comme dans les
/// métadonnées créées avant CurseForge).
pub fn source(nom: &str) -> &'static dyn SourceContenu {
    if nom == CURSEFORGE {
        &crate::contenu::curseforge::CurseForge
    } else {
        &crate::contenu::modrinth::Modrinth
    }
}

/// Privilégie la dernière version stable, sinon la plus récente.
pub fn meilleure(versions: Vec<VersionContenu>) -> Option<VersionContenu> {
    let stable = versions.iter().position(|v| v.stable);
    let mut versions = versions;
    match stable {
        Some(i) => Some(versions.swap_remove(i)),
        None => versions.into_iter().next(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(id: &str, stable: bool) -> VersionContenu {
        VersionContenu {
            id: id.into(),
            stable,
            ..Default::default()
        }
    }

    #[test]
    fn meilleure_version_stable_sinon_la_plus_recente() {
        let choisie = meilleure(vec![
            v("beta", false),
            v("stable", true),
            v("ancienne", true),
        ]);
        assert_eq!(choisie.unwrap().id, "stable");
        assert_eq!(
            meilleure(vec![v("alpha", false), v("beta", false)])
                .unwrap()
                .id,
            "alpha"
        );
        assert!(meilleure(Vec::new()).is_none());
    }

    #[test]
    fn source_par_nom() {
        assert_eq!(source("curseforge").nom(), "curseforge");
        assert_eq!(source("modrinth").nom(), "modrinth");
        // métadonnées d'avant CurseForge : pas de source
        assert_eq!(source("").nom(), "modrinth");
    }
}
