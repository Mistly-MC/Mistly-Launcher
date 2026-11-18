//! Client minimal de l'API CurseForge v1 (https://docs.curseforge.com/rest-api/).
//!
//! L'API exige une clé personnelle (console.curseforge.com), qui doit rester privée :
//! elle est lue depuis la variable d'environnement MISTY_CURSEFORGE_KEY, sinon depuis
//! curseforge.key (ignoré par git, intégré à l'exe à la compilation par build.rs).

use crate::contenu::catalogue::{ProjetDistant, CURSEFORGE};
use crate::contenu::content::{DATAPACK, MOD, MODPACK, PACK_RESSOURCES, SHADER};
use crate::contenu::source::{
    Dependance, DetailsContenu, FutureSource, Recherche, SourceContenu, VersionContenu,
};
use crate::minecraft::instances::Instance;
use crate::minecraft::loaders;
use crate::systeme::app_info;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::sync::OnceLock;
use std::time::Duration;

const BASE: &str = "https://api.curseforge.com/v1/";
const ID_MINECRAFT: i64 = 432;

// Classes CurseForge (types de projets) pour Minecraft
pub const CLASSE_MODS: i64 = 6;
pub const CLASSE_PACKS_RESSOURCES: i64 = 12;
pub const CLASSE_SHADERS: i64 = 6552;
pub const CLASSE_DATAPACKS: i64 = 6945;
pub const CLASSE_MODPACKS: i64 = 4471;

pub const TRIS: &[(&str, &str)] = &[
    ("2", "Tri_Popularite"),
    ("6", "Tri_Telechargements"),
    ("3", "Tri_MisAJour"),
    ("1", "Tri_Vedette"),
    ("4", "Tri_Nom"),
];

/// Clé API, ou None si elle n'est pas configurée.
pub fn cle() -> Option<&'static str> {
    static CLE: OnceLock<Option<String>> = OnceLock::new();
    CLE.get_or_init(|| {
        let cle = std::env::var("MISTY_CURSEFORGE_KEY")
            .ok()
            .filter(|c| !c.trim().is_empty())
            .or_else(|| option_env!("MISTY_CF_KEY").map(str::to_string));
        cle.map(|c| c.trim().to_string()).filter(|c| !c.is_empty())
    })
    .as_deref()
}

pub fn disponible() -> bool {
    cle().is_some()
}

fn client() -> anyhow::Result<&'static reqwest::Client> {
    static C: OnceLock<reqwest::Client> = OnceLock::new();
    let Some(cle) = cle() else {
        anyhow::bail!(crate::contenu::ContenuErreur::CleCF)
    };
    Ok(C.get_or_init(|| {
        let mut entetes = reqwest::header::HeaderMap::new();
        if let Ok(v) = reqwest::header::HeaderValue::from_str(cle) {
            entetes.insert("x-api-key", v);
        }
        reqwest::Client::builder()
            .user_agent(app_info::user_agent())
            .default_headers(entetes)
            .timeout(Duration::from_secs(300))
            .build()
            .expect("client CurseForge")
    }))
}

// ───────────────────────── DTO ─────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReponseCF<T> {
    pub data: T,
    pub pagination: Option<PaginationCF>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginationCF {
    #[serde(default)]
    pub total_count: i64,
}

#[derive(Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModCF {
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub download_count: f64,
    #[serde(default)]
    pub class_id: Option<i64>,
    pub logo: Option<LogoCF>,
    #[serde(default)]
    pub authors: Vec<NomCF>,
    #[serde(default)]
    pub categories: Vec<NomCF>,
    pub links: Option<LiensCF>,
    /// Dernier fichier par (version de Minecraft, loader, type de version).
    #[serde(default)]
    pub latest_files_indexes: Vec<IndexFichierCF>,
}

#[derive(Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct IndexFichierCF {
    #[serde(default)]
    pub game_version: String,
    pub file_id: i64,
    #[serde(default)]
    pub filename: String,
    /// 1 = release, 2 = bêta, 3 = alpha.
    #[serde(default)]
    pub release_type: i64,
    pub mod_loader: Option<i64>,
}

#[derive(Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct LogoCF {
    pub thumbnail_url: Option<String>,
    pub url: Option<String>,
}

#[derive(Deserialize, Clone, Default)]
pub struct NomCF {
    #[serde(default)]
    pub name: String,
}

#[derive(Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct LiensCF {
    pub website_url: Option<String>,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FichierCF {
    pub id: i64,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub file_name: String,
    /// 1 = release, 2 = bêta, 3 = alpha.
    #[serde(default)]
    pub release_type: i64,
    /// None si l'auteur interdit le téléchargement par des applications tierces.
    pub download_url: Option<String>,
    #[serde(default)]
    pub file_date: String,
    #[serde(default = "vrai")]
    pub is_available: bool,
    /// Empreinte MurmurHash2 du fichier (voir `empreinte`).
    #[serde(default)]
    pub file_fingerprint: i64,
    #[serde(default)]
    pub hashes: Vec<HashCF>,
    #[serde(default)]
    pub dependencies: Vec<DependanceCF>,
    /// Versions de Minecraft ET noms de loaders ("1.21.1", "Fabric"…).
    #[serde(default)]
    pub game_versions: Vec<String>,
}

fn vrai() -> bool {
    true
}

impl FichierCF {
    pub fn sha1(&self) -> Option<&str> {
        self.hashes
            .iter()
            .find(|h| h.algo == 1)
            .map(|h| h.value.as_str())
    }

    /// Côtés client / serveur au format Modrinth, d'après les étiquettes « Client » / « Server »
    /// que CurseForge ajoute aux versions du fichier ("unknown" si l'auteur n'a rien indiqué).
    pub fn environnement(&self) -> (String, String) {
        let a = |nom: &str| {
            self.game_versions
                .iter()
                .any(|v| v.eq_ignore_ascii_case(nom))
        };
        let (client, serveur) = (a("Client"), a("Server"));
        if !client && !serveur {
            return ("unknown".into(), "unknown".into());
        }
        let cote = |oui: bool| if oui { "optional" } else { "unsupported" }.to_string();
        (cote(client), cote(serveur))
    }
}

#[derive(Deserialize, Clone)]
pub struct HashCF {
    #[serde(default)]
    pub value: String,
    /// 1 = SHA-1, 2 = MD5.
    #[serde(default)]
    pub algo: i64,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DependanceCF {
    pub mod_id: i64,
    /// 3 = dépendance requise.
    #[serde(default)]
    pub relation_type: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EmpreintesCF {
    #[serde(default)]
    exact_matches: Vec<CorrespondanceCF>,
}

#[derive(Deserialize)]
pub struct CorrespondanceCF {
    pub id: i64,
    pub file: FichierCF,
}

// ── Format des modpacks CurseForge (manifest.json) ──

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestCF {
    pub minecraft: MinecraftCF,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub files: Vec<FichierManifestCF>,
    pub overrides: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MinecraftCF {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub mod_loaders: Vec<LoaderManifestCF>,
}

#[derive(Deserialize)]
pub struct LoaderManifestCF {
    /// "forge-47.2.0", "fabric-0.15.7", "neoforge-21.1.77"…
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub primary: bool,
}

#[derive(Deserialize)]
pub struct FichierManifestCF {
    #[serde(rename = "projectID")]
    pub project_id: i64,
    #[serde(rename = "fileID")]
    pub file_id: i64,
    #[serde(default = "vrai")]
    pub required: bool,
}

// ───────────────────────── Correspondances ─────────────────────────

pub fn classe(type_: &str) -> i64 {
    match type_ {
        PACK_RESSOURCES => CLASSE_PACKS_RESSOURCES,
        SHADER => CLASSE_SHADERS,
        DATAPACK => CLASSE_DATAPACKS,
        MODPACK => CLASSE_MODPACKS,
        _ => CLASSE_MODS,
    }
}

/// Type de contenu Misty correspondant à une classe CurseForge (pour ranger les fichiers d'un modpack).
pub fn type_depuis_classe(classe: i64) -> &'static str {
    match classe {
        CLASSE_PACKS_RESSOURCES => PACK_RESSOURCES,
        CLASSE_SHADERS => SHADER,
        _ => MOD,
    }
}

/// Identifiant CurseForge du loader (ModLoaderType), ou None.
pub fn type_loader(loader: &str) -> Option<i64> {
    match loader {
        loaders::FORGE => Some(1),
        loaders::LITELOADER => Some(3),
        loaders::FABRIC => Some(4),
        loaders::QUILT => Some(5),
        loaders::NEOFORGE => Some(6),
        _ => None,
    }
}

/// Noms de loaders acceptés dans GameVersions d'un fichier de mod.
pub fn noms_loader(loader: &str) -> Vec<&str> {
    match loader {
        loaders::QUILT => vec!["Quilt", "Fabric"], // Quilt charge aussi les mods Fabric
        autre => vec![autre],
    }
}

// ───────────────────────── Requêtes ─────────────────────────

async fn get<T: DeserializeOwned>(url: &str) -> anyhow::Result<T> {
    Ok(client()?
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

async fn post<T: DeserializeOwned>(url: &str, corps: serde_json::Value) -> anyhow::Result<T> {
    Ok(client()?
        .post(url)
        .json(&corps)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

pub async fn rechercher(
    requete: &str,
    classe: i64,
    version_mc: Option<&str>,
    type_loader: Option<i64>,
    tri: &str,
    index: usize,
    taille: usize,
) -> anyhow::Result<(Vec<ModCF>, i64)> {
    // CurseForge refuse index + pageSize > 10 000
    let taille = taille.min(10_000usize.saturating_sub(index));
    if taille == 0 {
        return Ok((Vec::new(), index as i64));
    }
    let mut url = format!(
        "{BASE}mods/search?gameId={ID_MINECRAFT}&classId={classe}&searchFilter={}&sortField={tri}&sortOrder=desc&index={index}&pageSize={taille}",
        urlencoding::encode(requete)
    );
    if let Some(v) = version_mc.filter(|v| !v.is_empty()) {
        url += &format!("&gameVersion={}", urlencoding::encode(v));
    }
    if let Some(l) = type_loader {
        url += &format!("&modLoaderType={l}");
    }
    let reponse: ReponseCF<Vec<ModCF>> = get(&url).await?;
    let total = reponse
        .pagination
        .map(|p| p.total_count)
        .unwrap_or(0)
        .min(10_000);
    Ok((reponse.data, total))
}

/// Fichiers d'un projet (les plus récents d'abord), filtrés sur une version de Minecraft.
pub async fn fichiers(mod_id: i64, version_mc: Option<&str>) -> anyhow::Result<Vec<FichierCF>> {
    let mut fichiers = Vec::new();
    // On se limite aux 500 fichiers les plus récents
    for index in (0..500).step_by(50) {
        let mut url = format!("{BASE}mods/{mod_id}/files?index={index}&pageSize=50");
        if let Some(v) = version_mc.filter(|v| !v.is_empty()) {
            url += &format!("&gameVersion={}", urlencoding::encode(v));
        }
        let reponse: ReponseCF<Vec<FichierCF>> = get(&url).await?;
        let n = reponse.data.len();
        fichiers.extend(reponse.data);
        let total = reponse.pagination.map(|p| p.total_count).unwrap_or(0);
        if n < 50 || fichiers.len() as i64 >= total {
            break;
        }
    }
    // Les dates ISO 8601 se trient comme du texte
    fichiers.sort_by(|a, b| b.file_date.cmp(&a.file_date));
    Ok(fichiers)
}

fn distincts(ids: &[i64]) -> Vec<i64> {
    let mut v = ids.to_vec();
    v.sort_unstable();
    v.dedup();
    v
}

pub async fn fichiers_par_ids(ids: &[i64]) -> anyhow::Result<Vec<FichierCF>> {
    let mut resultat = Vec::new();
    for paquet in distincts(ids).chunks(500) {
        let r: ReponseCF<Vec<FichierCF>> = post(
            &format!("{BASE}mods/files"),
            serde_json::json!({ "fileIds": paquet }),
        )
        .await?;
        resultat.extend(r.data);
    }
    Ok(resultat)
}

pub async fn mods(ids: &[i64]) -> anyhow::Result<Vec<ModCF>> {
    let mut resultat = Vec::new();
    for paquet in distincts(ids).chunks(500) {
        let r: ReponseCF<Vec<ModCF>> = post(
            &format!("{BASE}mods"),
            serde_json::json!({ "modIds": paquet }),
        )
        .await?;
        resultat.extend(r.data);
    }
    Ok(resultat)
}

/// Identifie des fichiers par leur empreinte CurseForge (voir `empreinte`).
pub async fn empreintes(empreintes: &[u32]) -> anyhow::Result<Vec<CorrespondanceCF>> {
    let mut liste = empreintes.to_vec();
    liste.sort_unstable();
    liste.dedup();
    if liste.is_empty() {
        return Ok(Vec::new());
    }
    let r: ReponseCF<EmpreintesCF> = post(
        &format!("{BASE}fingerprints/{ID_MINECRAFT}"),
        serde_json::json!({ "fingerprints": liste }),
    )
    .await?;
    Ok(r.data.exact_matches)
}

/// Page du fichier sur le site CurseForge (pour un téléchargement manuel).
pub fn page_fichier(m: Option<&ModCF>, mod_id: i64, file_id: i64) -> String {
    match m
        .and_then(|m| m.links.as_ref())
        .and_then(|l| l.website_url.as_deref())
        .filter(|s| !s.is_empty())
    {
        Some(site) => format!("{}/files/{file_id}", site.trim_end_matches('/')),
        None => format!("https://www.curseforge.com/projects/{mod_id}"),
    }
}

/// Empreinte CurseForge d'un fichier : MurmurHash2 (graine 1) calculé en ignorant
/// les octets d'espacement (tabulation, retour chariot, saut de ligne, espace).
pub fn empreinte(chemin: &std::path::Path) -> std::io::Result<u32> {
    let octets: Vec<u8> = std::fs::read(chemin)?
        .into_iter()
        .filter(|b| !matches!(b, 9 | 10 | 13 | 32))
        .collect();

    const M: u32 = 0x5bd1_e995;
    const R: u32 = 24;
    let mut longueur = octets.len() as u32;
    let mut h: u32 = 1 ^ longueur;
    let mut i = 0usize;

    while longueur >= 4 {
        let mut k = u32::from_le_bytes([octets[i], octets[i + 1], octets[i + 2], octets[i + 3]]);
        k = k.wrapping_mul(M);
        k ^= k >> R;
        k = k.wrapping_mul(M);
        h = h.wrapping_mul(M);
        h ^= k;
        i += 4;
        longueur -= 4;
    }

    if longueur == 3 {
        h ^= (octets[i + 2] as u32) << 16;
    }
    if longueur >= 2 {
        h ^= (octets[i + 1] as u32) << 8;
    }
    if longueur >= 1 {
        h ^= octets[i] as u32;
        h = h.wrapping_mul(M);
    }

    h ^= h >> 13;
    h = h.wrapping_mul(M);
    h ^= h >> 15;
    Ok(h)
}

/// Conversion vers le modèle commun de l'explorateur.
pub fn vers_projet(m: ModCF) -> ProjetDistant {
    ProjetDistant {
        source: CURSEFORGE.into(),
        id: m.id.to_string(),
        titre: m.name,
        description: m.summary,
        auteur: m
            .authors
            .first()
            .map(|a| a.name.clone())
            .unwrap_or_default(),
        telechargements: m.download_count as i64,
        icone_url: m
            .logo
            .as_ref()
            .and_then(|l| l.thumbnail_url.clone().or_else(|| l.url.clone())),
        categories: m.categories.into_iter().map(|c| c.name).collect(),
    }
}

// ───────────────────────── Source de contenu ─────────────────────────

/// CurseForge comme source de contenu (voir `source::SourceContenu`).
pub struct CurseForge;

impl FichierCF {
    /// Modèle commun : dépendances requises seulement (relationType 3).
    pub fn vers_contenu(&self, mod_id: i64) -> VersionContenu {
        let (client, serveur) = self.environnement();
        VersionContenu {
            id: self.id.to_string(),
            projet: mod_id.to_string(),
            nom: self.display_name.clone(),
            nom_fichier: self.file_name.clone(),
            url: self.download_url.clone().filter(|u| !u.is_empty()),
            sha1: self.sha1().map(str::to_string),
            stable: self.release_type == 1,
            client_side: Some(client),
            server_side: Some(serveur),
            dependances: self
                .dependencies
                .iter()
                .filter(|d| d.relation_type == 3)
                .map(|d| Dependance {
                    projet: Some(d.mod_id.to_string()),
                    version: None,
                })
                .collect(),
        }
    }
}

/// Fichiers installables dans le profil : disponibles, pour sa version de Minecraft (et son
/// loader pour les mods), dans l'ordre reçu (les plus récents d'abord).
pub fn fichiers_compatibles(
    fichiers: Vec<FichierCF>,
    instance: &Instance,
    type_: &str,
) -> Vec<FichierCF> {
    let noms = noms_loader(&instance.loader);
    fichiers
        .into_iter()
        .filter(|f| f.is_available && f.game_versions.contains(&instance.version_mc))
        .filter(|f| {
            type_ != MOD
                || f.game_versions
                    .iter()
                    .any(|v| noms.iter().any(|n| n.eq_ignore_ascii_case(v)))
        })
        .collect()
}

fn id_numerique(id: &str) -> anyhow::Result<i64> {
    Ok(id.trim().parse()?)
}

impl SourceContenu for CurseForge {
    fn nom(&self) -> &'static str {
        CURSEFORGE
    }

    fn libelle(&self) -> &'static str {
        "CurseForge"
    }

    fn tris(&self) -> &'static [(&'static str, &'static str)] {
        TRIS
    }

    fn rechercher<'a>(&'a self, r: Recherche<'a>) -> FutureSource<'a, (Vec<ProjetDistant>, i64)> {
        Box::pin(async move {
            // Sur CurseForge, les mods Quilt sont presque tous publiés comme mods Fabric
            let loader = match r.instance {
                Some(i) if r.type_ == MOD => {
                    let l = if i.loader == loaders::QUILT {
                        loaders::FABRIC
                    } else {
                        i.loader.as_str()
                    };
                    type_loader(l)
                }
                _ => None,
            };
            let (mods, total) = rechercher(
                r.requete,
                classe(r.type_),
                r.instance.map(|i| i.version_mc.as_str()),
                loader,
                r.tri,
                r.offset,
                r.limite,
            )
            .await?;
            Ok((mods.into_iter().map(vers_projet).collect(), total))
        })
    }

    /// Un projet inconnu est une erreur : son nom sert à tous les messages.
    fn details<'a>(&'a self, id: &'a str) -> FutureSource<'a, Option<DetailsContenu>> {
        Box::pin(async move {
            let mod_id = id_numerique(id)?;
            let m = mods(&[mod_id])
                .await?
                .into_iter()
                .next()
                .ok_or(crate::contenu::ContenuErreur::ProjetCF(mod_id))?;
            Ok(Some(DetailsContenu {
                id: m.id.to_string(),
                titre: m.name,
                icone_url: m.logo.and_then(|l| l.thumbnail_url),
                client_side: None,
                server_side: None,
                site: m
                    .links
                    .and_then(|l| l.website_url)
                    .filter(|s| !s.is_empty()),
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
            let mod_id = id_numerique(id)?;
            let liste = fichiers(mod_id, Some(&instance.version_mc)).await?;
            Ok(fichiers_compatibles(liste, instance, type_)
                .iter()
                .map(|f| f.vers_contenu(mod_id))
                .collect())
        })
    }

    /// Le fichier demandé, seulement s'il est compatible avec le profil.
    fn version<'a>(
        &'a self,
        id: &'a str,
        version_id: &'a str,
        instance: &'a Instance,
        type_: &'a str,
    ) -> FutureSource<'a, Option<VersionContenu>> {
        Box::pin(async move {
            Ok(self
                .versions(id, instance, type_)
                .await?
                .into_iter()
                .find(|v| v.id == version_id.trim()))
        })
    }

    fn cle_pas_de_version(&self) -> &'static str {
        "Erreur_PasDeVersionCF"
    }

    fn page_version(
        &self,
        details: Option<&DetailsContenu>,
        id: &str,
        version: &VersionContenu,
    ) -> String {
        match details.and_then(|d| d.site.as_deref()) {
            Some(site) => format!("{}/files/{}", site.trim_end_matches('/'), version.id),
            None => format!("https://www.curseforge.com/projects/{id}"),
        }
    }
}

#[cfg(test)]
mod tests_source {
    use super::*;

    fn fichier(id: i64, versions: &[&str], stable: bool, url: Option<&str>) -> FichierCF {
        serde_json::from_value(serde_json::json!({
            "id": id, "displayName": format!("Mod {id}"), "fileName": format!("mod-{id}.jar"),
            "releaseType": if stable { 1 } else { 2 }, "downloadUrl": url,
            "gameVersions": versions,
            "hashes": [{ "value": "md5", "algo": 2 }, { "value": "sha1-du-fichier", "algo": 1 }],
            "dependencies": [{ "modId": 306612, "relationType": 3 }, { "modId": 1, "relationType": 2 }]
        }))
        .unwrap()
    }

    fn profil(version: &str, loader: &str) -> Instance {
        serde_json::from_value(serde_json::json!({ "VersionMc": version, "Loader": loader }))
            .unwrap()
    }

    #[test]
    fn fichiers_compatibles_version_et_loader() {
        let liste = vec![
            fichier(1, &["1.21.1", "NeoForge"], true, Some("u")),
            fichier(2, &["1.21.1", "Fabric", "Client"], false, Some("u")),
            fichier(3, &["1.20.1", "Fabric"], true, Some("u")),
        ];
        let fabric = profil("1.21.1", loaders::FABRIC);
        let ids = |l: Vec<FichierCF>| l.iter().map(|f| f.id).collect::<Vec<_>>();
        assert_eq!(ids(fichiers_compatibles(liste.clone(), &fabric, MOD)), [2]);
        // pas de filtre de loader pour un pack de ressources
        assert_eq!(
            ids(fichiers_compatibles(liste, &fabric, PACK_RESSOURCES)),
            [1, 2]
        );
    }

    #[test]
    fn fichier_cf_vers_contenu() {
        let c = fichier(
            7,
            &["1.21.1", "Fabric", "Client"],
            true,
            Some("https://edge/mod-7.jar"),
        )
        .vers_contenu(42);
        assert_eq!((c.id.as_str(), c.projet.as_str()), ("7", "42"));
        assert_eq!(c.sha1.as_deref(), Some("sha1-du-fichier"));
        assert!(c.stable);
        assert_eq!(
            (c.client_side.as_deref(), c.server_side.as_deref()),
            (Some("optional"), Some("unsupported"))
        );
        assert_eq!(
            c.dependances,
            vec![Dependance {
                projet: Some("306612".into()),
                version: None
            }]
        );
        // téléchargement interdit par l'auteur : pas d'adresse
        assert!(fichier(8, &[], true, None).vers_contenu(42).url.is_none());
        assert!(fichier(9, &[], true, Some(""))
            .vers_contenu(42)
            .url
            .is_none());
    }

    #[test]
    fn page_d_un_fichier() {
        let v = VersionContenu {
            id: "7".into(),
            ..Default::default()
        };
        let d = DetailsContenu {
            site: Some("https://www.curseforge.com/minecraft/mc-mods/jei/".into()),
            ..Default::default()
        };
        assert_eq!(
            CurseForge.page_version(Some(&d), "42", &v),
            "https://www.curseforge.com/minecraft/mc-mods/jei/files/7"
        );
        assert_eq!(
            CurseForge.page_version(None, "42", &v),
            "https://www.curseforge.com/projects/42"
        );
    }
}
