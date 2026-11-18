//! Client minimal de l'API Modrinth v2 (https://docs.modrinth.com/api/).

use crate::contenu::catalogue::{ProjetDistant, MODRINTH};
use crate::contenu::content;
use crate::contenu::source::{
    Dependance, DetailsContenu, FutureSource, Recherche, SourceContenu, VersionContenu,
};
use crate::minecraft::instances::Instance;
use crate::systeme::http;
use serde::Deserialize;
use std::collections::HashMap;

const BASE: &str = "https://api.modrinth.com/v2/";

/// Tris proposés par la recherche Modrinth : (clé API, clé de traduction).
pub const TRIS: &[(&str, &str)] = &[
    ("relevance", "Tri_Pertinence"),
    ("downloads", "Tri_Telechargements"),
    ("follows", "Tri_Popularite"),
    ("newest", "Tri_Recents"),
    ("updated", "Tri_MisAJour"),
];

#[derive(Deserialize, Default)]
pub struct ProjetModrinth {
    #[serde(default)]
    pub project_id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub downloads: i64,
    pub icon_url: Option<String>,
    #[serde(default)]
    pub categories: Vec<String>,
}

#[derive(Deserialize, Default)]
pub struct ResultatRecherche {
    #[serde(default)]
    pub hits: Vec<ProjetModrinth>,
    #[serde(default)]
    pub total_hits: i64,
}

/// Projet renvoyé par /project/{id} ou /projects (champs utiles seulement).
#[derive(Deserialize, Clone)]
pub struct ProjetDetail {
    pub id: String,
    #[serde(default)]
    pub title: String,
    pub icon_url: Option<String>,
    /// required / optional / unsupported / unknown
    pub client_side: Option<String>,
    pub server_side: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct VersionModrinth {
    pub id: String,
    #[serde(default)]
    pub project_id: String,
    #[serde(default)]
    pub version_number: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub version_type: String,
    #[serde(default)]
    pub files: Vec<FichierModrinth>,
    #[serde(default)]
    pub dependencies: Vec<DependanceModrinth>,
}

impl VersionModrinth {
    pub fn fichier_principal(&self) -> Option<&FichierModrinth> {
        self.files
            .iter()
            .find(|f| f.primary)
            .or_else(|| self.files.first())
    }
}

#[derive(Deserialize, Clone)]
pub struct FichierModrinth {
    pub url: String,
    pub filename: String,
    #[serde(default)]
    pub primary: bool,
    #[serde(default)]
    pub hashes: HashMap<String, String>,
}

#[derive(Deserialize, Clone)]
pub struct DependanceModrinth {
    pub version_id: Option<String>,
    pub project_id: Option<String>,
    #[serde(default)]
    pub dependency_type: String,
}

// ── Format .mrpack (modrinth.index.json) ──

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexModpack {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version_id: String,
    #[serde(default)]
    pub files: Vec<FichierModpack>,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
}

#[derive(Deserialize)]
pub struct FichierModpack {
    pub path: String,
    #[serde(default)]
    pub hashes: HashMap<String, String>,
    pub env: Option<HashMap<String, String>>,
    #[serde(default)]
    pub downloads: Vec<String>,
}

fn json_url(valeur: &impl serde::Serialize) -> String {
    urlencoding::encode(&serde_json::to_string(valeur).unwrap_or_default()).into_owned()
}

/// `type_` : mod, resourcepack, datapack, shader, modpack.
/// `categories` : catégories acceptées (OU), ex. le loader : ["fabric"].
pub async fn rechercher(
    requete: &str,
    type_: &str,
    version_mc: Option<&str>,
    categories: Option<&[&str]>,
    tri: &str,
    offset: usize,
    limite: usize,
) -> anyhow::Result<ResultatRecherche> {
    let mut facettes: Vec<Vec<String>> = vec![vec![format!("project_type:{type_}")]];
    if let Some(v) = version_mc.filter(|v| !v.is_empty()) {
        facettes.push(vec![format!("versions:{v}")]);
    }
    if let Some(c) = categories.filter(|c| !c.is_empty()) {
        facettes.push(c.iter().map(|c| format!("categories:{c}")).collect());
    }
    let url = format!(
        "{BASE}search?query={}&index={tri}&offset={offset}&limit={limite}&facets={}",
        urlencoding::encode(requete),
        json_url(&facettes)
    );
    http::get_json(&url).await
}

/// Versions d'un projet, les plus récentes d'abord.
pub async fn versions(
    projet: &str,
    loaders: Option<&[&str]>,
    version_mc: Option<&str>,
) -> anyhow::Result<Vec<VersionModrinth>> {
    let mut parametres = Vec::new();
    if let Some(l) = loaders.filter(|l| !l.is_empty()) {
        parametres.push(format!("loaders={}", json_url(&l)));
    }
    if let Some(v) = version_mc.filter(|v| !v.is_empty()) {
        parametres.push(format!("game_versions={}", json_url(&[v])));
    }
    let mut url = format!("{BASE}project/{}/version", urlencoding::encode(projet));
    if !parametres.is_empty() {
        url.push('?');
        url.push_str(&parametres.join("&"));
    }
    http::get_json(&url).await
}

pub async fn version(version_id: &str) -> anyhow::Result<VersionModrinth> {
    http::get_json(&format!(
        "{BASE}version/{}",
        urlencoding::encode(version_id)
    ))
    .await
}

pub async fn projets(ids: &[String]) -> anyhow::Result<Vec<ProjetDetail>> {
    let mut liste: Vec<&String> = ids.iter().collect();
    liste.sort();
    liste.dedup();
    if liste.is_empty() {
        return Ok(Vec::new());
    }
    http::get_json(&format!("{BASE}projects?ids={}", json_url(&liste))).await
}

/// Identifie des fichiers par leur SHA-1. Renvoie hash → version.
pub async fn versions_par_hash(
    sha1: &[String],
) -> anyhow::Result<HashMap<String, VersionModrinth>> {
    if sha1.is_empty() {
        return Ok(HashMap::new());
    }
    let corps = serde_json::json!({ "hashes": sha1, "algorithm": "sha1" });
    let reponse = http::client()
        .post(format!("{BASE}version_files"))
        .json(&corps)
        .send()
        .await?
        .error_for_status()?;
    Ok(reponse.json().await?)
}

/// Dernière version compatible (loaders + version de Minecraft) de chaque fichier, par SHA-1.
/// Renvoie hash du fichier installé → version la plus récente.
pub async fn dernieres_versions(
    sha1: &[String],
    loaders: &[&str],
    version_mc: &str,
) -> anyhow::Result<HashMap<String, VersionModrinth>> {
    if sha1.is_empty() {
        return Ok(HashMap::new());
    }
    let corps = serde_json::json!({ "hashes": sha1, "algorithm": "sha1", "loaders": loaders, "game_versions": [version_mc] });
    let reponse = http::client()
        .post(format!("{BASE}version_files/update"))
        .json(&corps)
        .send()
        .await?
        .error_for_status()?;
    Ok(reponse.json().await?)
}

/// Conversion vers le modèle commun de l'explorateur.
pub fn vers_projet(p: ProjetModrinth) -> ProjetDistant {
    ProjetDistant {
        source: MODRINTH.into(),
        id: p.project_id,
        titre: p.title,
        description: p.description,
        auteur: p.author,
        telechargements: p.downloads,
        icone_url: p.icon_url,
        categories: p.categories,
    }
}

/// 1234567 -> "1.2 M"
pub fn formater_nombre(n: i64) -> String {
    let f = |x: f64| {
        let s = format!("{x:.1}");
        s.strip_suffix(".0").map(str::to_string).unwrap_or(s)
    };
    if n >= 1_000_000 {
        format!("{} M", f(n as f64 / 1_000_000.0))
    } else if n >= 1_000 {
        format!("{} k", f(n as f64 / 1_000.0))
    } else {
        n.to_string()
    }
}

// ───────────────────────── Source de contenu ─────────────────────────

/// Modrinth comme source de contenu (voir `source::SourceContenu`).
pub struct Modrinth;

impl VersionModrinth {
    /// Modèle commun : fichier principal, dépendances requises seulement.
    pub fn vers_contenu(self) -> Option<VersionContenu> {
        let fichier = self.fichier_principal()?.clone();
        Some(VersionContenu {
            id: self.id,
            projet: self.project_id,
            nom: self.version_number,
            nom_fichier: fichier.filename,
            url: Some(fichier.url),
            sha1: fichier.hashes.get("sha1").cloned(),
            stable: self.version_type == "release",
            // les côtés client / serveur sont ceux du projet (DetailsContenu)
            client_side: None,
            server_side: None,
            dependances: self
                .dependencies
                .into_iter()
                .filter(|d| d.dependency_type == "required")
                .map(|d| Dependance {
                    projet: d.project_id,
                    version: d.version_id,
                })
                .collect(),
        })
    }
}

impl SourceContenu for Modrinth {
    fn nom(&self) -> &'static str {
        MODRINTH
    }

    fn libelle(&self) -> &'static str {
        "Modrinth"
    }

    fn tris(&self) -> &'static [(&'static str, &'static str)] {
        TRIS
    }

    fn rechercher<'a>(&'a self, r: Recherche<'a>) -> FutureSource<'a, (Vec<ProjetDistant>, i64)> {
        Box::pin(async move {
            let categories = r
                .instance
                .and_then(|i| content::categories_recherche(i, r.type_));
            let reponse = rechercher(
                r.requete,
                r.type_,
                r.instance.map(|i| i.version_mc.as_str()),
                categories.as_deref(),
                r.tri,
                r.offset,
                r.limite,
            )
            .await?;
            Ok((
                reponse.hits.into_iter().map(vers_projet).collect(),
                reponse.total_hits,
            ))
        })
    }

    fn details<'a>(&'a self, id: &'a str) -> FutureSource<'a, Option<DetailsContenu>> {
        Box::pin(async move {
            Ok(projets(&[id.to_string()])
                .await?
                .into_iter()
                .next()
                .map(|p| DetailsContenu {
                    id: p.id,
                    titre: p.title,
                    icone_url: p.icon_url,
                    client_side: p.client_side,
                    server_side: p.server_side,
                    site: None,
                }))
        })
    }

    fn versions<'a>(
        &'a self,
        id: &'a str,
        instance: &'a Instance,
        type_: &'a str,
    ) -> FutureSource<'a, Vec<VersionContenu>> {
        Box::pin(async move {
            let loaders = content::loaders_modrinth(instance, type_);
            Ok(versions(id, loaders.as_deref(), Some(&instance.version_mc))
                .await?
                .into_iter()
                .filter_map(VersionModrinth::vers_contenu)
                .collect())
        })
    }

    /// La version demandée telle quelle (Modrinth la donne directement par son ID).
    fn version<'a>(
        &'a self,
        _id: &'a str,
        version_id: &'a str,
        _instance: &'a Instance,
        _type_: &'a str,
    ) -> FutureSource<'a, Option<VersionContenu>> {
        Box::pin(async move { Ok(version(version_id).await?.vers_contenu()) })
    }

    fn cle_pas_de_version(&self) -> &'static str {
        "Erreur_PasDeVersion"
    }

    fn page_version(
        &self,
        _details: Option<&DetailsContenu>,
        id: &str,
        version: &VersionContenu,
    ) -> String {
        format!("https://modrinth.com/project/{id}/version/{}", version.id)
    }
}

#[cfg(test)]
mod tests_source {
    use super::*;

    #[test]
    fn version_modrinth_vers_contenu() {
        let v: VersionModrinth = serde_json::from_value(serde_json::json!({
            "id": "AbCd", "project_id": "P1", "version_number": "1.2.3", "version_type": "beta",
            "files": [
                { "url": "https://x/autre.jar", "filename": "autre.jar", "primary": false, "hashes": {} },
                { "url": "https://x/mod.jar", "filename": "mod.jar", "primary": true, "hashes": { "sha1": "abc" } }
            ],
            "dependencies": [
                { "project_id": "fabric-api", "version_id": null, "dependency_type": "required" },
                { "project_id": "modmenu", "dependency_type": "optional" },
                { "version_id": "V9", "dependency_type": "required" }
            ]
        }))
        .unwrap();
        let c = v.vers_contenu().unwrap();
        assert_eq!(
            (c.id.as_str(), c.projet.as_str(), c.nom.as_str()),
            ("AbCd", "P1", "1.2.3")
        );
        assert_eq!(c.nom_fichier, "mod.jar");
        assert_eq!(c.url.as_deref(), Some("https://x/mod.jar"));
        assert_eq!(c.sha1.as_deref(), Some("abc"));
        assert!(!c.stable);
        assert_eq!(
            c.dependances,
            vec![
                Dependance {
                    projet: Some("fabric-api".into()),
                    version: None
                },
                Dependance {
                    projet: None,
                    version: Some("V9".into())
                },
            ]
        );
    }

    #[test]
    fn version_sans_fichier_ignoree() {
        let v: VersionModrinth =
            serde_json::from_value(serde_json::json!({ "id": "x", "files": [] })).unwrap();
        assert!(v.vers_contenu().is_none());
    }
}
