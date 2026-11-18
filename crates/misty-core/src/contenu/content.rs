//! Contenu d'un profil : mods, packs de ressources, datapacks, shaders.
//! Installation depuis Modrinth ou CurseForge (avec dépendances), liste, activation/désactivation, suppression.

use crate::contenu::catalogue::{CURSEFORGE, MODRINTH};
use crate::contenu::source::{self, SourceContenu, VersionContenu};
use crate::minecraft::instances::Instance;
use crate::minecraft::loaders;
use crate::{contenu::curseforge, contenu::modrinth, systeme::download, systeme::loc, Progress};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

pub const MOD: &str = "mod";
pub const PACK_RESSOURCES: &str = "resourcepack";
pub const DATAPACK: &str = "datapack";
pub const SHADER: &str = "shader";
pub const MODPACK: &str = "modpack";

/// Un fichier désactivé est renommé "xxx.jar.disabled" (même convention que Modrinth App / Prism).
pub const SUFFIXE_DESACTIVE: &str = ".disabled";

const FICHIER_META: &str = "misty-contenu.json";

// Projets Modrinth utiles pour les shaders
pub const ID_IRIS: &str = "YL57xq9U";
pub const ID_OCULUS: &str = "GchcoXML";

/// Métadonnées d'un fichier installé depuis Modrinth / CurseForge (ou identifié par son hash).
/// Stockées dans instances\{Id}\misty-contenu.json (noms de champs de la version C#).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContenuMeta {
    /// Chemin relatif au dossier du profil, sans le suffixe ".disabled".
    #[serde(default)]
    pub chemin: String,
    /// modrinth ou curseforge (None = modrinth, fichiers créés avant CurseForge).
    pub source: Option<String>,
    pub project_id: Option<String>,
    pub version_id: Option<String>,
    #[serde(default)]
    pub titre: String,
    pub version_nom: Option<String>,
    pub icone_url: Option<String>,
    /// Le champ JSON s'appelle "Type" : `type` est un mot réservé en Rust.
    #[serde(default, rename = "Type")]
    pub type_: String,
    /// Côté client / serveur indiqué par Modrinth (required, optional, unsupported, unknown).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_side: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_side: Option<String>,
}

impl ContenuMeta {
    pub fn correspond(&self, source: &str, id: &str) -> bool {
        self.source.as_deref().unwrap_or(MODRINTH) == source
            && self.project_id.as_deref() == Some(id)
    }

    pub fn source(&self) -> &str {
        self.source.as_deref().unwrap_or(MODRINTH)
    }
}

/// Où un mod doit être installé, d'après Modrinth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Environnement {
    Inconnu,
    Client,
    Serveur,
    LesDeux,
}

impl Environnement {
    pub fn depuis(client: Option<&str>, serveur: Option<&str>) -> Self {
        let utile = |c: Option<&str>| matches!(c, Some("required") | Some("optional"));
        let connu = |c: Option<&str>| {
            matches!(c, Some("required") | Some("optional") | Some("unsupported"))
        };
        if !connu(client) || !connu(serveur) {
            return Environnement::Inconnu;
        }
        match (utile(client), utile(serveur)) {
            (true, true) => Environnement::LesDeux,
            (true, false) => Environnement::Client,
            (false, true) => Environnement::Serveur,
            _ => Environnement::Inconnu,
        }
    }

    /// 0 inconnu, 1 client, 2 serveur, 3 les deux (valeur passée à l'interface).
    pub fn code(self) -> i32 {
        match self {
            Environnement::Inconnu => 0,
            Environnement::Client => 1,
            Environnement::Serveur => 2,
            Environnement::LesDeux => 3,
        }
    }
}

/// Nouvelle version disponible pour un fichier installé.
#[derive(Clone, Debug)]
pub struct MajDispo {
    /// ID de la version Modrinth ou du fichier CurseForge.
    pub version_id: String,
    pub version_nom: String,
}

/// Un fichier présent dans le profil (mod, pack de ressources, datapack, shader).
#[derive(Clone, Debug)]
pub struct ContenuInstalle {
    pub chemin_fichier: PathBuf,
    pub type_: String,
    pub monde: Option<String>,
    pub meta: Option<ContenuMeta>,
}

impl ContenuInstalle {
    pub fn nom_fichier(&self) -> String {
        self.chemin_fichier
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default()
    }

    pub fn titre(&self) -> String {
        match &self.meta {
            Some(m) if !m.titre.is_empty() => m.titre.clone(),
            _ => nom_sans_suffixe(&self.nom_fichier()),
        }
    }

    pub fn details(&self) -> String {
        let mut morceaux = vec![libelle_type(&self.type_)];
        if let Some(v) = self
            .meta
            .as_ref()
            .and_then(|m| m.version_nom.clone())
            .filter(|v| !v.is_empty())
        {
            morceaux.push(v);
        }
        if self.meta.as_ref().and_then(|m| m.source.as_deref()) == Some(CURSEFORGE) {
            morceaux.push("CurseForge".into());
        }
        if let Some(monde) = &self.monde {
            morceaux.push(loc::tf("Contenu_Monde", &[monde]));
        }
        if self
            .meta
            .as_ref()
            .and_then(|m| m.project_id.as_ref())
            .is_none()
        {
            morceaux.push(self.nom_fichier());
        }
        morceaux.join(" · ")
    }

    pub fn actif(&self) -> bool {
        !self
            .nom_fichier()
            .to_lowercase()
            .ends_with(SUFFIXE_DESACTIVE)
    }

    pub fn icone_url(&self) -> Option<String> {
        self.meta.as_ref().and_then(|m| m.icone_url.clone())
    }

    /// Client / serveur : seulement pour les mods dont Modrinth donne l'information.
    pub fn environnement(&self) -> Environnement {
        match &self.meta {
            Some(m) if self.type_ == MOD => {
                Environnement::depuis(m.client_side.as_deref(), m.server_side.as_deref())
            }
            _ => Environnement::Inconnu,
        }
    }
}

/// Types installables dans un profil, avec leur libellé dans la langue actuelle.
pub fn types() -> Vec<(String, String)> {
    vec![
        (MOD.into(), loc::t("Type_Mods")),
        (PACK_RESSOURCES.into(), loc::t("Type_Packs")),
        (DATAPACK.into(), loc::t("Type_Datapacks")),
        (SHADER.into(), loc::t("Type_Shaders")),
    ]
}

pub fn libelle_type(type_: &str) -> String {
    match type_ {
        MOD => loc::t("Type_Mod"),
        PACK_RESSOURCES => loc::t("Type_Pack"),
        DATAPACK => loc::t("Type_Datapack"),
        SHADER => loc::t("Type_Shader"),
        MODPACK => loc::t("Type_Modpack"),
        autre => autre.to_string(),
    }
}

/// "mod" / "mods" selon le nombre, en minuscules (ex. "3 mods").
pub fn libelle_nombre(type_: &str, n: usize) -> String {
    let pluriel = n > 1;
    match type_ {
        MOD => loc::t(if pluriel { "Nb_Mods" } else { "Nb_Mod" }),
        PACK_RESSOURCES => loc::t(if pluriel { "Nb_Packs" } else { "Nb_Pack" }),
        DATAPACK => loc::t(if pluriel {
            "Nb_Datapacks"
        } else {
            "Nb_Datapack"
        }),
        SHADER => loc::t(if pluriel { "Nb_Shaders" } else { "Nb_Shader" }),
        autre => autre.to_string(),
    }
}

pub fn supporte_mods(loader: &str) -> bool {
    matches!(
        loader,
        loaders::FABRIC | loaders::QUILT | loaders::FORGE | loaders::NEOFORGE | loaders::LITELOADER
    )
}

/// Loaders Modrinth compatibles avec le profil pour ce type de contenu (None = pas de filtre).
pub fn loaders_modrinth(instance: &Instance, type_: &str) -> Option<Vec<&'static str>> {
    match type_ {
        MOD => Some(match instance.loader.as_str() {
            loaders::FABRIC => vec!["fabric"],
            loaders::QUILT => vec!["quilt", "fabric"], // Quilt charge aussi les mods Fabric
            loaders::FORGE => vec!["forge"],
            loaders::NEOFORGE => vec!["neoforge"],
            loaders::LITELOADER => vec!["liteloader"],
            _ => vec!["__aucun__"],
        }),
        PACK_RESSOURCES => Some(vec!["minecraft"]),
        DATAPACK => Some(vec!["datapack"]),
        SHADER => Some(if instance.loader == loaders::OPTIFINE {
            vec!["optifine"]
        } else {
            vec!["iris", "optifine"]
        }),
        _ => None,
    }
}

/// Catégories à passer à la recherche (le loader pour les mods et shaders).
pub fn categories_recherche(instance: &Instance, type_: &str) -> Option<Vec<&'static str>> {
    if type_ == MOD || type_ == SHADER {
        loaders_modrinth(instance, type_)
    } else {
        None
    }
}

pub fn dossier(instance: &Instance, type_: &str, monde: Option<&str>) -> anyhow::Result<PathBuf> {
    let base = instance.dossier();
    Ok(match type_ {
        MOD => base.join("mods"),
        PACK_RESSOURCES => base.join("resourcepacks"),
        SHADER => base.join("shaderpacks"),
        DATAPACK => base
            .join("saves")
            .join(monde.ok_or_else(|| anyhow::anyhow!("Monde requis pour un datapack"))?)
            .join("datapacks"),
        autre => anyhow::bail!("Type inconnu : {autre}"),
    })
}

/// Mondes du profil, du plus récent au plus ancien.
pub fn mondes(instance: &Instance) -> Vec<String> {
    let Ok(entrees) = fs::read_dir(instance.dossier().join("saves")) else {
        return Vec::new();
    };
    let mut liste: Vec<(std::time::SystemTime, String)> = entrees
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| {
            let date = e
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::UNIX_EPOCH);
            (date, e.file_name().to_string_lossy().to_string())
        })
        .collect();
    liste.sort_by_key(|e| std::cmp::Reverse(e.0));
    liste.into_iter().map(|(_, n)| n).collect()
}

pub fn nom_sans_suffixe(nom_fichier: &str) -> String {
    let mut nom = nom_fichier;
    if nom.to_lowercase().ends_with(SUFFIXE_DESACTIVE) {
        nom = &nom[..nom.len() - SUFFIXE_DESACTIVE.len()];
    }
    match nom.rfind('.') {
        Some(i) if i > 0 => nom[..i].to_string(),
        _ => nom.to_string(),
    }
}

// ───────────────────────── Liste ─────────────────────────

pub fn lister(instance: &Instance) -> Vec<ContenuInstalle> {
    let mut metas: HashMap<String, ContenuMeta> = HashMap::new();
    for m in lire_meta(instance) {
        metas.insert(m.chemin.to_lowercase(), m);
    }
    let mut resultat = Vec::new();

    let mut scanner =
        |dossier: PathBuf, type_: &str, extensions: &[&str], monde: Option<String>| {
            let Ok(entrees) = fs::read_dir(&dossier) else {
                return;
            };
            for entree in entrees.flatten() {
                let chemin = entree.path();
                if !chemin.is_file() {
                    continue;
                }
                let nom = entree.file_name().to_string_lossy().to_lowercase();
                let nom = nom.strip_suffix(SUFFIXE_DESACTIVE).unwrap_or(&nom);
                if !extensions.iter().any(|e| nom.ends_with(e)) {
                    continue;
                }
                let meta = metas
                    .get(&relatif(instance, &chemin).to_lowercase())
                    .cloned();
                resultat.push(ContenuInstalle {
                    chemin_fichier: chemin,
                    type_: type_.to_string(),
                    monde: monde.clone(),
                    meta,
                });
            }
        };

    let base = instance.dossier();
    scanner(base.join("mods"), MOD, &[".jar", ".litemod"], None);
    scanner(base.join("resourcepacks"), PACK_RESSOURCES, &[".zip"], None);
    scanner(base.join("shaderpacks"), SHADER, &[".zip"], None);
    for monde in mondes(instance) {
        if let Ok(d) = dossier(instance, DATAPACK, Some(&monde)) {
            scanner(d, DATAPACK, &[".zip"], Some(monde));
        }
    }

    resultat.sort_by(|a, b| {
        a.type_
            .cmp(&b.type_)
            .then_with(|| a.titre().to_lowercase().cmp(&b.titre().to_lowercase()))
    });
    resultat
}

/// Empreinte des dossiers de contenu (noms, tailles, dates) : elle change dès qu'un fichier est
/// ajouté, retiré, renommé ou remplacé à la main, sans rien lire des fichiers eux-mêmes.
pub fn empreinte(instance: &Instance) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let base = instance.dossier();
    let mut dossiers = vec![
        base.join("mods"),
        base.join("resourcepacks"),
        base.join("shaderpacks"),
    ];
    for monde in mondes(instance) {
        dossiers.push(base.join("saves").join(monde).join("datapacks"));
    }
    for d in dossiers {
        d.hash(&mut h);
        let Ok(entrees) = fs::read_dir(&d) else {
            continue;
        };
        let mut fichiers: Vec<(String, u64, u128)> = entrees
            .flatten()
            .map(|e| {
                let m = e.metadata().ok();
                let date = m
                    .as_ref()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|t| t.as_millis())
                    .unwrap_or(0);
                (
                    e.file_name().to_string_lossy().to_string(),
                    m.map(|m| m.len()).unwrap_or(0),
                    date,
                )
            })
            .collect();
        fichiers.sort();
        fichiers.hash(&mut h);
    }
    h.finish()
}

/// Indique si un projet (d'une source donnée) est présent dans la liste.
pub fn est_installe(contenus: &[ContenuInstalle], source: &str, id: &str) -> bool {
    contenus
        .iter()
        .any(|c| c.meta.as_ref().is_some_and(|m| m.correspond(source, id)))
}

/// Identifie les fichiers ajoutés à la main, pour afficher leur nom et leur icône :
/// d'abord via Modrinth (SHA-1), puis via CurseForge (empreinte) pour ceux qui restent inconnus.
/// Les fichiers introuvables sont mémorisés pour ne pas être re-hachés à chaque fois.
/// Renvoie la liste mise à jour.
pub async fn identifier(
    instance: &Instance,
    mut contenus: Vec<ContenuInstalle>,
) -> anyhow::Result<Vec<ContenuInstalle>> {
    let inconnus: Vec<usize> = (0..contenus.len())
        .filter(|&i| contenus[i].meta.is_none())
        .collect();
    if inconnus.is_empty() {
        return Ok(contenus);
    }

    // Chaque fichier inconnu reçoit une métadonnée (vide si on ne le trouve nulle part)
    for &i in &inconnus {
        let c = &mut contenus[i];
        c.meta = Some(ContenuMeta {
            chemin: relatif(instance, &c.chemin_fichier),
            type_: c.type_.clone(),
            ..Default::default()
        });
    }

    // 1. Modrinth
    let mut par_hash: HashMap<String, usize> = HashMap::new();
    for &i in &inconnus {
        let chemin = contenus[i].chemin_fichier.clone();
        if let Ok(Ok(h)) =
            tokio::task::spawn_blocking(move || download::sha1_fichier(&chemin)).await
        {
            par_hash.insert(h, i);
        }
    }
    let hashes: Vec<String> = par_hash.keys().cloned().collect();
    let versions = modrinth::versions_par_hash(&hashes).await?;
    let ids: Vec<String> = versions.values().map(|v| v.project_id.clone()).collect();
    let projets: HashMap<String, modrinth::ProjetDetail> = modrinth::projets(&ids)
        .await?
        .into_iter()
        .map(|p| (p.id.clone(), p))
        .collect();

    for (hash, &i) in &par_hash {
        let Some(version) = versions.get(hash) else {
            continue;
        };
        let Some(projet) = projets.get(&version.project_id) else {
            continue;
        };
        let meta = contenus[i].meta.as_mut().unwrap();
        meta.source = Some(MODRINTH.into());
        meta.project_id = Some(projet.id.clone());
        meta.version_id = Some(version.id.clone());
        meta.titre = projet.title.clone();
        meta.version_nom = Some(version.version_number.clone());
        meta.icone_url = projet.icon_url.clone();
        meta.client_side = projet.client_side.clone();
        meta.server_side = projet.server_side.clone();
    }

    // 2. CurseForge, pour ce qui n'est pas sur Modrinth (injoignable : on garde les noms de fichiers)
    let restants: Vec<usize> = inconnus
        .iter()
        .copied()
        .filter(|&i| contenus[i].meta.as_ref().unwrap().project_id.is_none())
        .collect();
    if !restants.is_empty() && curseforge::disponible() {
        let mut par_empreinte: HashMap<u32, usize> = HashMap::new();
        for &i in &restants {
            let chemin = contenus[i].chemin_fichier.clone();
            if let Ok(Ok(e)) =
                tokio::task::spawn_blocking(move || curseforge::empreinte(&chemin)).await
            {
                par_empreinte.insert(e, i);
            }
        }
        let empreintes: Vec<u32> = par_empreinte.keys().copied().collect();
        if let Ok(correspondances) = curseforge::empreintes(&empreintes).await {
            let ids: Vec<i64> = correspondances.iter().map(|c| c.id).collect();
            if let Ok(mods) = curseforge::mods(&ids).await {
                let mods: HashMap<i64, curseforge::ModCF> =
                    mods.into_iter().map(|m| (m.id, m)).collect();
                for c in &correspondances {
                    let Some(&i) = par_empreinte.get(&(c.file.file_fingerprint as u32)) else {
                        continue;
                    };
                    let Some(m) = mods.get(&c.id) else { continue };
                    let meta = contenus[i].meta.as_mut().unwrap();
                    meta.source = Some(CURSEFORGE.into());
                    meta.project_id = Some(m.id.to_string());
                    meta.version_id = Some(c.file.id.to_string());
                    meta.titre = m.name.clone();
                    meta.version_nom = Some(c.file.display_name.clone());
                    meta.icone_url = m.logo.as_ref().and_then(|l| l.thumbnail_url.clone());
                    let (client, serveur) = c.file.environnement();
                    meta.client_side = Some(client);
                    meta.server_side = Some(serveur);
                }
            }
        }
    }

    let nouvelles: Vec<ContenuMeta> = inconnus
        .iter()
        .map(|&i| contenus[i].meta.clone().unwrap())
        .collect();
    enregistrer_metas(instance, nouvelles)?;
    Ok(contenus)
}

/// Ajoute ou remplace des métadonnées (même chemin) dans misty-contenu.json.
pub fn enregistrer_metas(instance: &Instance, nouvelles: Vec<ContenuMeta>) -> anyhow::Result<()> {
    let mut metas = lire_meta(instance);
    for meta in nouvelles {
        metas.retain(|m| !m.chemin.eq_ignore_ascii_case(&meta.chemin));
        metas.push(meta);
    }
    ecrire_meta(instance, &metas)
}

// ───────────────────────── Mises à jour ─────────────────────────

/// Cherche les mises à jour du contenu installé (Modrinth par SHA-1, CurseForge par le dernier fichier
/// connu du projet) et complète au passage le côté client / serveur des mods Modrinth.
/// Renvoie chemin relatif → mise à jour disponible, et la liste avec les métadonnées complétées.
pub async fn verifier(
    instance: &Instance,
    mut contenus: Vec<ContenuInstalle>,
) -> anyhow::Result<(HashMap<String, MajDispo>, Vec<ContenuInstalle>)> {
    let mut majs = HashMap::new();

    // ── Modrinth ──
    let mr: Vec<usize> = (0..contenus.len())
        .filter(|&i| {
            contenus[i]
                .meta
                .as_ref()
                .is_some_and(|m| m.source() == MODRINTH && m.project_id.is_some())
        })
        .collect();

    // Côté client / serveur des mods (demandé une fois, puis gardé dans misty-contenu.json)
    let sans_env: Vec<usize> = mr
        .iter()
        .copied()
        .filter(|&i| {
            contenus[i].type_ == MOD && contenus[i].meta.as_ref().unwrap().client_side.is_none()
        })
        .collect();
    if !sans_env.is_empty() {
        let ids: Vec<String> = sans_env
            .iter()
            .filter_map(|&i| contenus[i].meta.as_ref().unwrap().project_id.clone())
            .collect();
        let mut projets: HashMap<String, modrinth::ProjetDetail> = HashMap::new();
        for paquet in ids.chunks(100) {
            for p in modrinth::projets(paquet).await? {
                projets.insert(p.id.clone(), p);
            }
        }
        let mut modifiees = Vec::new();
        for &i in &sans_env {
            let meta = contenus[i].meta.as_mut().unwrap();
            let Some(p) = meta.project_id.as_ref().and_then(|id| projets.get(id)) else {
                continue;
            };
            meta.client_side = Some(p.client_side.clone().unwrap_or_else(|| "unknown".into()));
            meta.server_side = Some(p.server_side.clone().unwrap_or_else(|| "unknown".into()));
            modifiees.push(meta.clone());
        }
        enregistrer_metas(instance, modifiees)?;
    }

    // Dernières versions compatibles, par type (chaque type a ses propres loaders)
    let chemins: Vec<(usize, PathBuf)> = mr
        .iter()
        .map(|&i| (i, contenus[i].chemin_fichier.clone()))
        .collect();
    let hashes: Vec<(usize, String)> = tokio::task::spawn_blocking(move || {
        chemins
            .into_iter()
            .filter_map(|(i, c)| download::sha1_fichier(&c).ok().map(|h| (i, h)))
            .collect()
    })
    .await?;
    let mut par_type: HashMap<String, Vec<(usize, String)>> = HashMap::new();
    for (i, h) in hashes {
        par_type
            .entry(contenus[i].type_.clone())
            .or_default()
            .push((i, h));
    }
    for (type_, fichiers) in par_type {
        let Some(loaders) = loaders_modrinth(instance, &type_) else {
            continue;
        };
        let liste: Vec<String> = fichiers.iter().map(|(_, h)| h.clone()).collect();
        let actuelles = modrinth::versions_par_hash(&liste).await?;
        let dernieres =
            modrinth::dernieres_versions(&liste, &loaders, &instance.version_mc).await?;
        for (i, h) in &fichiers {
            let (Some(actuelle), Some(derniere)) = (actuelles.get(h), dernieres.get(h)) else {
                continue;
            };
            // pas de bêta proposée à qui a une version stable
            let acceptable =
                derniere.version_type == "release" || actuelle.version_type != "release";
            if derniere.id != actuelle.id
                && derniere.project_id == actuelle.project_id
                && acceptable
            {
                majs.insert(
                    relatif(instance, &contenus[*i].chemin_fichier),
                    MajDispo {
                        version_id: derniere.id.clone(),
                        version_nom: derniere.version_number.clone(),
                    },
                );
            }
        }
    }

    // Côté client / serveur des mods CurseForge : étiquettes du fichier installé
    let cf_sans_env: Vec<(usize, i64)> = contenus
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            let m = c.meta.as_ref()?;
            (c.type_ == MOD && m.source() == CURSEFORGE && m.client_side.is_none())
                .then_some((i, m.version_id.as_ref()?.parse().ok()?))
        })
        .collect();
    if !cf_sans_env.is_empty() && curseforge::disponible() {
        let ids: Vec<i64> = cf_sans_env.iter().map(|x| x.1).collect();
        if let Ok(fichiers) = curseforge::fichiers_par_ids(&ids).await {
            let fichiers: HashMap<i64, curseforge::FichierCF> =
                fichiers.into_iter().map(|f| (f.id, f)).collect();
            let mut modifiees = Vec::new();
            for (i, id) in cf_sans_env {
                let Some(f) = fichiers.get(&id) else { continue };
                let (client, serveur) = f.environnement();
                let meta = contenus[i].meta.as_mut().unwrap();
                meta.client_side = Some(client);
                meta.server_side = Some(serveur);
                modifiees.push(meta.clone());
            }
            enregistrer_metas(instance, modifiees)?;
        }
    }

    // ── CurseForge ──
    let cf: Vec<(usize, i64, i64)> = contenus
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            let m = c.meta.as_ref()?;
            if m.source() != CURSEFORGE {
                return None;
            }
            Some((
                i,
                m.project_id.as_ref()?.parse().ok()?,
                m.version_id.as_ref()?.parse().ok()?,
            ))
        })
        .collect();
    if !cf.is_empty() && curseforge::disponible() {
        let ids: Vec<i64> = cf.iter().map(|x| x.1).collect();
        if let Ok(mods) = curseforge::mods(&ids).await {
            let mods: HashMap<i64, curseforge::ModCF> =
                mods.into_iter().map(|m| (m.id, m)).collect();
            let loaders_cf: Vec<i64> = match instance.loader.as_str() {
                loaders::QUILT => vec![5, 4], // Quilt charge aussi les mods Fabric
                l => curseforge::type_loader(l).into_iter().collect(),
            };
            for (i, mod_id, fichier_id) in cf {
                let Some(m) = mods.get(&mod_id) else { continue };
                let est_mod = contenus[i].type_ == MOD;
                let candidats: Vec<&curseforge::IndexFichierCF> = m
                    .latest_files_indexes
                    .iter()
                    .filter(|f| {
                        f.game_version == instance.version_mc
                            && (!est_mod || f.mod_loader.is_some_and(|l| loaders_cf.contains(&l)))
                    })
                    .collect();
                let meilleur = candidats
                    .iter()
                    .filter(|f| f.release_type == 1)
                    .max_by_key(|f| f.file_id)
                    .or_else(|| candidats.iter().max_by_key(|f| f.file_id));
                if let Some(f) = meilleur.filter(|f| f.file_id > fichier_id) {
                    majs.insert(
                        relatif(instance, &contenus[i].chemin_fichier),
                        MajDispo {
                            version_id: f.file_id.to_string(),
                            version_nom: nom_sans_suffixe(&f.filename),
                        },
                    );
                }
            }
        }
    }

    Ok((majs, contenus))
}

/// Remplace un fichier par la version `maj` (avec ses nouvelles dépendances requises).
/// Un fichier désactivé le reste. Renvoie les noms des projets installés.
pub async fn mettre_a_jour(
    instance: &Instance,
    contenu: &ContenuInstalle,
    maj: &MajDispo,
    progression: Option<Progress>,
) -> anyhow::Result<Vec<String>> {
    let meta = contenu
        .meta
        .clone()
        .ok_or_else(|| anyhow::anyhow!(loc::t("Erreur_AucunFichier")))?;
    let project_id = meta
        .project_id
        .clone()
        .ok_or_else(|| anyhow::anyhow!(loc::t("Erreur_AucunFichier")))?;
    let source = meta.source().to_string();
    let mut etat = Installation {
        instance,
        metas: lire_meta(instance),
        installes: Vec::new(),
        visites: HashSet::new(),
        progression,
    };
    let resultat = installer_projet(
        &mut etat,
        source::source(&source),
        project_id.clone(),
        contenu.type_.clone(),
        contenu.monde.clone(),
        Choix::ParId(maj.version_id.clone()),
    )
    .await;
    if resultat.is_ok() && !contenu.actif() {
        let nouveau = etat
            .metas
            .iter()
            .find(|m| m.correspond(&source, &project_id) && m.type_ == contenu.type_)
            .map(|m| instance.dossier().join(&m.chemin));
        if let Some(chemin) = nouveau {
            let _ = fs::rename(&chemin, format!("{}{SUFFIXE_DESACTIVE}", chemin.display()));
        }
    }
    ecrire_meta(instance, &etat.metas)?;
    resultat?;
    Ok(etat.installes)
}

/// Active / désactive un fichier. Renvoie le nouveau chemin.
pub fn basculer(contenu: &ContenuInstalle) -> anyhow::Result<PathBuf> {
    let chemin = contenu.chemin_fichier.to_string_lossy().to_string();
    let nouveau = if contenu.actif() {
        format!("{chemin}{SUFFIXE_DESACTIVE}")
    } else {
        chemin[..chemin.len() - SUFFIXE_DESACTIVE.len()].to_string()
    };
    fs::rename(&contenu.chemin_fichier, &nouveau)?;
    Ok(PathBuf::from(nouveau))
}

pub fn supprimer(instance: &Instance, contenu: &ContenuInstalle) -> anyhow::Result<()> {
    if contenu.chemin_fichier.exists() {
        fs::remove_file(&contenu.chemin_fichier)?;
    }
    let rel = relatif(instance, &contenu.chemin_fichier);
    let mut metas = lire_meta(instance);
    metas.retain(|m| !m.chemin.eq_ignore_ascii_case(&rel));
    ecrire_meta(instance, &metas)
}

// ───────────────────────── Installation ─────────────────────────

/// État partagé d'une installation (métadonnées, projets installés, projets déjà visités).
struct Installation<'a> {
    instance: &'a Instance,
    metas: Vec<ContenuMeta>,
    installes: Vec<String>,
    /// (source, ID ou slug) des projets déjà traités : une dépendance commune n'est installée
    /// qu'une fois, et les dépendances circulaires s'arrêtent
    visites: HashSet<(&'static str, String)>,
    progression: Option<Progress>,
}

/// Installe la meilleure version compatible d'un projet (et ses dépendances requises pour les mods).
/// Remplace la version déjà installée du même projet. Renvoie les noms des projets installés.
pub async fn installer(
    instance: &Instance,
    source: &str,
    project_id: &str,
    type_: &str,
    monde: Option<&str>,
    progression: Option<Progress>,
) -> anyhow::Result<Vec<String>> {
    let mut etat = Installation {
        instance,
        metas: lire_meta(instance),
        installes: Vec::new(),
        visites: HashSet::new(),
        progression,
    };
    let resultat = installer_projet(
        &mut etat,
        source::source(source),
        project_id.to_string(),
        type_.to_string(),
        monde.map(str::to_string),
        Choix::Meilleure,
    )
    .await;
    // On garde ce qui a déjà été installé même en cas d'erreur
    ecrire_meta(instance, &etat.metas)?;
    resultat?;
    Ok(etat.installes)
}

type Fut<'a> = Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>;

/// Version d'un projet à installer.
enum Choix {
    /// La meilleure version compatible (la dernière stable, sinon la plus récente)
    Meilleure,
    /// Une version précise (mise à jour) : erreur si elle n'existe pas ou n'est pas compatible
    ParId(String),
    /// Une version déjà récupérée (dépendance qui impose sa version)
    Connue(VersionContenu),
}

/// Installe un projet d'une source, puis ses dépendances requises (mods seulement).
fn installer_projet<'a>(
    etat: &'a mut Installation<'_>,
    source: &'static dyn SourceContenu,
    id: String,
    type_: String,
    monde: Option<String>,
    choix: Choix,
) -> Fut<'a> {
    Box::pin(async move {
        if !etat.visites.insert((source.nom(), id.clone())) {
            return Ok(());
        }
        let instance = etat.instance;

        let details = source.details(&id).await?;
        let version = match choix {
            Choix::Meilleure => source::meilleure(source.versions(&id, instance, &type_).await?),
            Choix::ParId(version_id) => source.version(&id, &version_id, instance, &type_).await?,
            Choix::Connue(v) => Some(v),
        };
        let Some(version) = version else {
            let titre = details.map(|d| d.titre).unwrap_or(id);
            anyhow::bail!(loc::tf(
                source.cle_pas_de_version(),
                &[&titre, &instance.version_mc, &instance.loader]
            ));
        };
        // On garde l'ID réel, même si on a reçu un slug
        let id = details.as_ref().map(|d| d.id.clone()).unwrap_or(id);
        etat.visites.insert((source.nom(), id.clone()));
        let titre = details
            .as_ref()
            .map(|d| d.titre.clone())
            .unwrap_or_else(|| version.nom_fichier.clone());

        // Téléchargement interdit hors du site (CurseForge) : on s'arrête avant de retirer
        // l'ancienne version
        if version.url.is_none() {
            anyhow::bail!(loc::tf(
                "Erreur_CFInterdit",
                &[
                    &titre,
                    &source.page_version(details.as_ref(), &id, &version)
                ]
            ));
        }

        let dossier = dossier(instance, &type_, monde.as_deref())?;
        retirer_ancienne_version(
            instance,
            &mut etat.metas,
            source.nom(),
            &id,
            &type_,
            &dossier,
        );

        if let Some(p) = &etat.progression {
            p(loc::tf("Progres_Telechargement", &[&titre]));
        }
        let destination = dossier.join(download::nom_fichier_sur(&version.nom_fichier));
        source.telecharger(&version, &destination).await?;

        let rel = relatif(instance, &destination);
        etat.metas.retain(|m| !m.chemin.eq_ignore_ascii_case(&rel));
        etat.metas.push(ContenuMeta {
            chemin: rel,
            source: Some(source.nom().into()),
            project_id: Some(id.clone()),
            version_id: Some(version.id.clone()),
            titre: titre.clone(),
            version_nom: Some(version.nom.clone()),
            icone_url: details.as_ref().and_then(|d| d.icone_url.clone()),
            type_: type_.clone(),
            // côtés du fichier (CurseForge), sinon ceux du projet (Modrinth)
            client_side: version
                .client_side
                .clone()
                .or_else(|| details.as_ref().and_then(|d| d.client_side.clone())),
            server_side: version
                .server_side
                .clone()
                .or_else(|| details.as_ref().and_then(|d| d.server_side.clone())),
        });
        etat.installes.push(titre);

        if type_ != MOD {
            return Ok(());
        }

        // Dépendances requises
        for dep in &version.dependances {
            // version imposée par la dépendance ; si elle est introuvable, la meilleure compatible
            let connue = match &dep.version {
                Some(v) => source
                    .version(dep.projet.as_deref().unwrap_or(""), v, instance, MOD)
                    .await
                    .ok()
                    .flatten(),
                None => None,
            };
            let Some(id_dep) = dep.projet.clone().or_else(|| {
                connue
                    .as_ref()
                    .map(|v| v.projet.clone())
                    .filter(|p| !p.is_empty())
            }) else {
                continue;
            };
            if etat
                .metas
                .iter()
                .any(|m| m.correspond(source.nom(), &id_dep))
            {
                continue; // déjà là
            }
            let choix = match connue {
                Some(v) => Choix::Connue(v),
                None => Choix::Meilleure,
            };
            installer_projet(etat, source, id_dep, MOD.into(), None, choix).await?;
        }
        Ok(())
    })
}

/// Mise à jour : retire l'ancienne version du même projet (au même endroit).
fn retirer_ancienne_version(
    instance: &Instance,
    metas: &mut Vec<ContenuMeta>,
    source: &str,
    project_id: &str,
    type_: &str,
    dossier: &Path,
) {
    let base = instance.dossier();
    metas.retain(|ancien| {
        if !(ancien.correspond(source, project_id) && ancien.type_ == type_) {
            return true;
        }
        let chemin = base.join(&ancien.chemin);
        let meme_dossier = chemin
            .parent()
            .map(|p| {
                p.to_string_lossy().to_lowercase().replace('/', "\\")
                    == dossier.to_string_lossy().to_lowercase().replace('/', "\\")
            })
            .unwrap_or(false);
        if !meme_dossier {
            return true;
        }
        let _ = fs::remove_file(&chemin);
        let _ = fs::remove_file(format!("{}{SUFFIXE_DESACTIVE}", chemin.display()));
        false
    });
}

/// Prérequis manquant pour les shaders : renvoie l'ID Modrinth du mod à installer (Iris / Oculus),
/// Some("") si les shaders sont impossibles avec ce loader, None si tout est prêt.
/// Iris / Oculus installés depuis CurseForge ou à la main sont reconnus à leur nom.
pub fn prerequis_shaders(
    instance: &Instance,
    contenus: &[ContenuInstalle],
) -> Option<&'static str> {
    let present = |id: &str, nom: &str| {
        contenus.iter().any(|c| {
            c.type_ == MOD
                && (c.meta.as_ref().is_some_and(|m| m.correspond(MODRINTH, id))
                    || c.titre().to_lowercase().starts_with(&nom.to_lowercase()))
        })
    };
    match instance.loader.as_str() {
        loaders::OPTIFINE => None,
        loaders::FABRIC | loaders::QUILT | loaders::NEOFORGE => {
            (!present(ID_IRIS, "Iris")).then_some(ID_IRIS)
        }
        loaders::FORGE => (!present(ID_OCULUS, "Oculus")).then_some(ID_OCULUS),
        _ => Some(""),
    }
}

// ───────────────────────── Utilitaires ─────────────────────────

/// Chemin relatif au profil, avec des "/" et sans ".disabled".
pub fn relatif(instance: &Instance, chemin: &Path) -> String {
    let mut texte = chemin.to_string_lossy().to_string();
    if texte.to_lowercase().ends_with(SUFFIXE_DESACTIVE) {
        texte.truncate(texte.len() - SUFFIXE_DESACTIVE.len());
    }
    let base = instance.dossier();
    let rel = Path::new(&texte)
        .strip_prefix(&base)
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|_| PathBuf::from(&texte));
    rel.to_string_lossy().replace('\\', "/")
}

fn lire_meta(instance: &Instance) -> Vec<ContenuMeta> {
    let chemin = instance.dossier().join(FICHIER_META);
    fs::read_to_string(chemin)
        .ok()
        .and_then(|t| serde_json::from_str(t.trim_start_matches('\u{feff}')).ok())
        .unwrap_or_default()
}

fn ecrire_meta(instance: &Instance, metas: &[ContenuMeta]) -> anyhow::Result<()> {
    fs::create_dir_all(instance.dossier())?;
    crate::systeme::fichiers::ecrire_atomique(
        instance.dossier().join(FICHIER_META),
        serde_json::to_string_pretty(metas)?,
    )?;
    Ok(())
}

/// Installation en vrai (réseau) par les deux sources, sur un dossier de données à part :
/// `MISTY_DATA_DIR=<dossier temporaire> cargo test installation_reseau -- --ignored --nocapture`.
#[cfg(test)]
mod tests_reseau {
    use super::*;
    use crate::minecraft::instances;

    fn dossier_de_test() -> bool {
        // jamais sur les vraies données du joueur
        std::env::var_os("MISTY_DATA_DIR").is_some()
            && crate::systeme::paths::data_dir()
                .starts_with(std::env::var_os("MISTY_DATA_DIR").unwrap())
    }

    fn meta<'a>(metas: &'a [ContenuMeta], source: &str, id: &str) -> &'a ContenuMeta {
        metas
            .iter()
            .find(|m| m.correspond(source, id))
            .unwrap_or_else(|| panic!("{source} {id} absent des métadonnées"))
    }

    /// Modrinth : un slug, sa dépendance requise, puis une mise à jour vers une version précise.
    #[tokio::test]
    #[ignore]
    async fn installation_reseau_modrinth() {
        assert!(dossier_de_test(), "MISTY_DATA_DIR requis");
        let i = instances::creer("Test Modrinth", "1.21.1", loaders::FABRIC, None).unwrap();

        // Mod Menu (slug « modmenu ») exige Fabric API
        let installes = installer(&i, MODRINTH, "modmenu", MOD, None, None)
            .await
            .unwrap();
        println!("installés : {installes:?}");
        let metas = lire_meta(&i);
        let mm = meta(&metas, MODRINTH, "mOgUt4GM"); // ID réel gardé, pas le slug
        let api = meta(&metas, MODRINTH, "P7dR8mSH");
        for m in [mm, api] {
            assert!(i.dossier().join(&m.chemin).exists(), "{} absent", m.chemin);
            assert!(m.client_side.is_some() && m.icone_url.is_some());
        }

        // mise à jour vers la deuxième version compatible (une version précise)
        let versions = modrinth::versions("mOgUt4GM", Some(&["fabric"]), Some("1.21.1"))
            .await
            .unwrap();
        let cible = &versions[1];
        let contenu = lister(&i)
            .into_iter()
            .find(|c| {
                c.meta
                    .as_ref()
                    .is_some_and(|m| m.correspond(MODRINTH, "mOgUt4GM"))
            })
            .unwrap();
        let maj = MajDispo {
            version_id: cible.id.clone(),
            version_nom: cible.version_number.clone(),
        };
        mettre_a_jour(&i, &contenu, &maj, None).await.unwrap();
        let metas = lire_meta(&i);
        let mm = meta(&metas, MODRINTH, "mOgUt4GM");
        assert_eq!(mm.version_id.as_deref(), Some(cible.id.as_str()));
        assert!(i.dossier().join(&mm.chemin).exists());
        assert!(
            !contenu.chemin_fichier.exists()
                || contenu.chemin_fichier == i.dossier().join(&mm.chemin)
        );
        instances::supprimer(&i).unwrap();
    }

    /// CurseForge : JEI, puis une mise à jour vers un fichier précis.
    #[tokio::test]
    #[ignore]
    async fn installation_reseau_curseforge() {
        assert!(dossier_de_test(), "MISTY_DATA_DIR requis");
        assert!(curseforge::disponible(), "clé CurseForge requise");
        let i = instances::creer("Test CurseForge", "1.20.1", loaders::FORGE, None).unwrap();

        let installes = installer(&i, CURSEFORGE, "238222", MOD, None, None)
            .await
            .unwrap();
        println!("installés : {installes:?}");
        let metas = lire_meta(&i);
        let jei = meta(&metas, CURSEFORGE, "238222");
        assert!(i.dossier().join(&jei.chemin).exists());
        assert_eq!(jei.titre, "Just Enough Items (JEI)");

        let compatibles = curseforge::fichiers_compatibles(
            curseforge::fichiers(238222, Some("1.20.1")).await.unwrap(),
            &i,
            MOD,
        );
        let cible = &compatibles[1];
        let contenu = lister(&i)
            .into_iter()
            .find(|c| {
                c.meta
                    .as_ref()
                    .is_some_and(|m| m.correspond(CURSEFORGE, "238222"))
            })
            .unwrap();
        let maj = MajDispo {
            version_id: cible.id.to_string(),
            version_nom: cible.display_name.clone(),
        };
        mettre_a_jour(&i, &contenu, &maj, None).await.unwrap();
        let jei = meta(&lire_meta(&i), CURSEFORGE, "238222").clone();
        assert_eq!(jei.version_id, Some(cible.id.to_string()));
        assert!(i.dossier().join(&jei.chemin).exists());

        // un fichier qui n'est pas pour ce profil est refusé, l'ancien reste
        let maj = MajDispo {
            version_id: "1".into(),
            version_nom: "?".into(),
        };
        let contenu = lister(&i)
            .into_iter()
            .find(|c| {
                c.meta
                    .as_ref()
                    .is_some_and(|m| m.correspond(CURSEFORGE, "238222"))
            })
            .unwrap();
        assert!(mettre_a_jour(&i, &contenu, &maj, None).await.is_err());
        assert!(i.dossier().join(&jei.chemin).exists());
        instances::supprimer(&i).unwrap();
    }
}
