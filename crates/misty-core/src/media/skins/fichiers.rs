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
//! Skins : fichiers sur le disque (historique « Mes skins », cache des profils et des textures).

use super::*;

/// Dossier où sont gardés les skins et capes déjà téléchargés.
/// Skins portés ou envoyés (« Mes skins » de la page Skin), les plus récents d'abord.
pub(super) fn dossier_historique() -> std::path::PathBuf {
    crate::systeme::paths::data_dir().join("skins")
}

/// Nombre de skins gardés dans l'historique.
pub(super) const HISTORIQUE_MAX: usize = 12;

/// Garde un skin dans l'historique (ou le remonte en tête s'il y est déjà). Nom : md5 du PNG, et
/// « -fin » pour les bras fins.
pub fn ajouter_historique(png: &[u8], fin: bool) {
    if !skin_valide(png) {
        return;
    }
    let dossier = dossier_historique();
    let _ = std::fs::create_dir_all(&dossier);
    let base = format!("{:x}", md5::compute(png));
    // le même skin avec l'autre modèle de bras : on remplace
    let _ = std::fs::remove_file(dossier.join(format!("{base}.png")));
    let _ = std::fs::remove_file(dossier.join(format!("{base}-fin.png")));
    let _ = std::fs::write(
        dossier.join(format!("{base}{}.png", if fin { "-fin" } else { "" })),
        png,
    );
    for (ancien, _) in historique().into_iter().skip(HISTORIQUE_MAX) {
        let _ = std::fs::remove_file(ancien);
    }
}

/// Les skins de l'historique (fichier, bras fins), du plus récent au plus ancien.
pub fn historique() -> Vec<(std::path::PathBuf, bool)> {
    let Ok(entrees) = std::fs::read_dir(dossier_historique()) else {
        return Vec::new();
    };
    let mut liste: Vec<(std::time::SystemTime, std::path::PathBuf)> = entrees
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "png"))
        .map(|p| {
            (
                std::fs::metadata(&p)
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::UNIX_EPOCH),
                p,
            )
        })
        .collect();
    liste.sort_by_key(|e| std::cmp::Reverse(e.0));
    liste
        .into_iter()
        .map(|(_, p)| {
            let fin = p
                .file_stem()
                .is_some_and(|n| n.to_string_lossy().ends_with("-fin"));
            (p, fin)
        })
        .collect()
}

/// Identifiant d'un skin de l'historique (son nom de fichier sans « -fin ») : md5 du PNG.
pub fn id_historique(png: &[u8]) -> String {
    format!("{:x}", md5::compute(png))
}

pub(super) fn dossier_cache() -> std::path::PathBuf {
    crate::systeme::paths::data_dir()
        .join("cache")
        .join("skins")
}

/// Télécharge une texture Mojang, ou la relit sur le disque si on l'a déjà eue.
/// L'adresse d'une texture contient son empreinte : le contenu d'une même adresse ne change jamais.
pub async fn telecharger(url: &str) -> anyhow::Result<Vec<u8>> {
    let fichier = dossier_cache().join(format!("{:x}.png", md5::compute(url)));
    if let Ok(octets) = std::fs::read(&fichier) {
        if !octets.is_empty() {
            return Ok(octets);
        }
    }
    if url.starts_with("local:") {
        anyhow::bail!("skin local introuvable"); // pas une vraie adresse : seulement sur le disque
    }
    let octets = http::client()
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?
        .to_vec();
    let _ = std::fs::create_dir_all(dossier_cache());
    let _ = std::fs::write(&fichier, &octets);
    Ok(octets)
}

/// Dernier profil connu d'un compte (relu quand Mojang ne répond pas).
pub fn profil_en_cache(uuid: &str) -> Option<Profil> {
    if uuid.is_empty() {
        return None;
    }
    serde_json::from_str(
        &std::fs::read_to_string(dossier_cache().join(format!("profil-{uuid}.json"))).ok()?,
    )
    .ok()
}

pub fn enregistrer_profil(uuid: &str, profil: &Profil) {
    if uuid.is_empty() {
        return;
    }
    let _ = std::fs::create_dir_all(dossier_cache());
    let _ = std::fs::write(
        dossier_cache().join(format!("profil-{uuid}.json")),
        serde_json::to_string(profil).unwrap_or_default(),
    );
}

/// Après l'envoi d'un skin : on le garde sur le disque, pour l'afficher même si Mojang ne répond pas ensuite.
pub fn memoriser_skin_envoye(uuid: &str, png: &[u8], fin: bool) {
    let adresse = format!("local:{uuid}");
    let _ = std::fs::create_dir_all(dossier_cache());
    let _ = std::fs::write(
        dossier_cache().join(format!("{:x}.png", md5::compute(&adresse))),
        png,
    );
    let mut profil = profil_en_cache(uuid).unwrap_or_default();
    profil.skin_url = adresse;
    profil.fin = fin;
    enregistrer_profil(uuid, &profil);
}
