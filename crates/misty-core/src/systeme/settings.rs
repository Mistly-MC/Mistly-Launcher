//! Lecture / écriture de data.txt (format "cle=valeur", une clé par ligne).
//! Même format que la version C#, pour garder les réglages existants.

use crate::{systeme::app_info, systeme::paths};
use std::collections::HashMap;
use std::fs;

pub const VERSION_DATA: &str = "versiondatatxt";
pub const LAST_PSEUDO: &str = "lastpseudo";
pub const LAST_VERSION: &str = "lastversion";
pub const RAM: &str = "ram";
pub const PREMIUM: &str = "premium";
pub const RACCOURCI: &str = "raccourci";
pub const SUPPR: &str = "Suppr";
pub const DERNIER_PROFIL: &str = "lastprofil";
pub const RICH_PRESENCE: &str = "richpresence";
pub const LANGUE: &str = "langue";
/// Apparence : 0 brume (clair), 1 nuit (sombre), 2 comme Windows
pub const THEME: &str = "theme";
pub const FLOU: &str = "flou";
/// Thèmes de saison automatiques (Halloween, Noël)
pub const SAISON: &str = "saison";
/// Flou du paysage en fond (0 = net, 10 = très flou)
pub const FOND_FLOU: &str = "fondflou";
/// Build d'Horizon lancé par JOUER (tag GitHub)
pub const HORIZON_BUILD: &str = "horizonbuild";
/// Nombre de fichiers téléchargés en même temps pendant l’installation du jeu
pub const TELECHARGEMENTS: &str = "telechargements";
/// Console du jeu ouverte au lancement d'une partie
pub const CONSOLE: &str = "console";
/// Télémétrie anonyme : "" pas encore demandé, "true" acceptée, "false" refusée
pub const TELEMETRIE: &str = "telemetrie";
/// Identifiant d'installation aléatoire, envoyé avec la télémétrie
pub const TELEMETRIE_ID: &str = "telemetrieid";
/// Mises à jour : proposer aussi les pré-releases (bêtas) de GitHub
pub const MAJ_PRERELEASE: &str = "majprerelease";
/// Misty Client en jeu (menu, HUD, badge et cosmétiques Misty) sur les profils compatibles
pub const MISTY_CLIENT: &str = "mistyclient";
/// Cosmétique Misty équipé : "" aucun, "misty_cape", "misty_elytra" (voir minecraft::client)
pub const COSMETIQUE: &str = "cosmetique";
/// Chapeau Misty porté : "" aucun, "cloudly" (voir minecraft::client)
pub const CHAPEAU: &str = "chapeau";
/// JOUER lance la version Misty (Fabric + Misty Client)
pub const MISTY_CHOISI: &str = "mistychoisi";
/// Version de Minecraft choisie dans la section Misty Client ("26.3")
pub const MISTY_VERSION: &str = "mistyversion";
/// Serveurs et barres d'objets partagés entre toutes les versions et tous les profils (contenu::partage)
pub const PARTAGE: &str = "partage";
/// Onglets masqués de la barre latérale, séparés par des virgules ("skin,notes")
pub const BARRE_MASQUEE: &str = "barremasquee";
/// Façon de jouer choisie au premier lancement : "" pas encore choisie, "premium" (compte
/// Microsoft) ou "horsligne" (un pseudo)
pub const MODE_COMPTE: &str = "modecompte";
/// Style du pseudo au-dessus de la tête (Misty Client) : "" classique, ou voir minecraft::client::TAGS
pub const TAG: &str = "tag";

/// Ordre des clés dans le fichier.
const CLES: &[&str] = &[
    VERSION_DATA,
    LAST_PSEUDO,
    LAST_VERSION,
    RAM,
    PREMIUM,
    RACCOURCI,
    SUPPR,
    DERNIER_PROFIL,
    RICH_PRESENCE,
    LANGUE,
    THEME,
    FLOU,
    SAISON,
    FOND_FLOU,
    HORIZON_BUILD,
    TELECHARGEMENTS,
    CONSOLE,
    TELEMETRIE,
    TELEMETRIE_ID,
    MAJ_PRERELEASE,
    MISTY_CLIENT,
    COSMETIQUE,
    CHAPEAU,
    MISTY_CHOISI,
    MISTY_VERSION,
    PARTAGE,
    BARRE_MASQUEE,
    MODE_COMPTE,
    TAG,
];

/// data.txt gardé en mémoire : les réglages sont lus des dizaines de fois (à chaque statut
/// Discord, à chaque lancement…), le fichier n'est relu que s'il a changé sur le disque.
struct Cache {
    chemin: std::path::PathBuf,
    /// Date et taille du fichier au moment de la lecture
    empreinte: (Option<std::time::SystemTime>, u64),
    lignes: Vec<String>,
}

static CACHE: std::sync::Mutex<Option<Cache>> = std::sync::Mutex::new(None);

fn empreinte(chemin: &std::path::Path) -> Option<(Option<std::time::SystemTime>, u64)> {
    let meta = fs::metadata(chemin).ok()?;
    Some((meta.modified().ok(), meta.len()))
}

fn lire_lignes() -> Option<Vec<String>> {
    let chemin = paths::data_file();
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    // fichier absent : rien (et rien à garder)
    let Some(actuelle) = empreinte(&chemin) else {
        *cache = None;
        return None;
    };
    if let Some(c) = cache.as_ref() {
        if c.chemin == chemin && c.empreinte == actuelle {
            return Some(c.lignes.clone());
        }
    }
    let texte = fs::read_to_string(&chemin).ok()?;
    let lignes: Vec<String> = texte.lines().map(str::to_string).collect();
    *cache = Some(Cache {
        chemin,
        empreinte: actuelle,
        lignes: lignes.clone(),
    });
    Some(lignes)
}

fn ecrire_lignes(lignes: &[String]) {
    let mut texte = lignes.join("\r\n");
    texte.push_str("\r\n");
    let chemin = paths::data_file();
    let ecrit = crate::systeme::fichiers::ecrire_atomique(&chemin, texte).is_ok();
    // le cache suit ce qui vient d'être écrit ; en cas d'échec, il sera relu sur le disque
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    *cache = match (ecrit, empreinte(&chemin)) {
        (true, Some(e)) => Some(Cache {
            chemin,
            empreinte: e,
            lignes: lignes.to_vec(),
        }),
        _ => None,
    };
}

/// Valeur d'une clé ("" si absente).
pub fn get(cle: &str) -> String {
    let prefixe = format!("{cle}=");
    let mut valeur = String::new();
    for ligne in lire_lignes().unwrap_or_default() {
        if let Some(v) = ligne.strip_prefix(&prefixe) {
            valeur = v.to_string();
        }
    }
    valeur
}

#[cfg(test)]
mod tests_cache {
    /// L'empreinte qui valide le cache change quand le fichier change (et manque sans fichier).
    #[test]
    fn empreinte_suit_le_fichier() {
        let lignes = |t: &str| t.lines().map(str::to_string).collect::<Vec<_>>();
        let dossier = std::env::temp_dir().join(format!("misty-reglages-{}", std::process::id()));
        std::fs::create_dir_all(&dossier).unwrap();
        let chemin = dossier.join("data.txt");
        std::fs::write(&chemin, "a=1\r\n").unwrap();
        let e1 = super::empreinte(&chemin).unwrap();
        assert_eq!(
            lignes(&std::fs::read_to_string(&chemin).unwrap()),
            vec!["a=1"]
        );
        // contenu plus long : l'empreinte change même si la date a la même seconde
        std::fs::write(&chemin, "a=1\r\nb=2\r\n").unwrap();
        assert_ne!(super::empreinte(&chemin).unwrap(), e1);
        assert!(super::empreinte(&dossier.join("absent.txt")).is_none());
        let _ = std::fs::remove_dir_all(dossier);
    }
}

pub fn get_bool(cle: &str, defaut: bool) -> bool {
    let valeur = get(cle);
    if valeur.trim().is_empty() {
        defaut
    } else {
        valeur.eq_ignore_ascii_case("true")
    }
}

/// Modifie une clé. Ne fait rien si data.txt n'existe pas.
pub fn set(cle: &str, valeur: &str) {
    let Some(mut lignes) = lire_lignes() else {
        return;
    };
    let prefixe = format!("{cle}=");
    let mut trouvee = false;
    for ligne in lignes.iter_mut() {
        if ligne.starts_with(&prefixe) {
            *ligne = format!("{cle}={valeur}");
            trouvee = true;
        }
    }
    if !trouvee {
        lignes.push(format!("{cle}={valeur}"));
    }
    ecrire_lignes(&lignes);
}

pub fn set_bool(cle: &str, valeur: bool) {
    set(cle, if valeur { "true" } else { "false" });
}

/// Crée data.txt s'il n'existe pas, ou le réécrit au format actuel (en gardant les valeurs)
/// si sa version est différente de DATA_VERSION.
pub fn initialiser() {
    let _ = fs::create_dir_all(paths::data_dir());

    if !paths::data_file().exists() {
        let mut valeurs = HashMap::new();
        valeurs.insert(LAST_PSEUDO.to_string(), "Mistly".to_string());
        ecrire(valeurs);
        return;
    }

    if get(VERSION_DATA) != app_info::DATA_VERSION {
        let valeurs = CLES.iter().map(|c| (c.to_string(), get(c))).collect();
        ecrire(valeurs);
    }
}

fn ecrire(mut valeurs: HashMap<String, String>) {
    valeurs.insert(VERSION_DATA.to_string(), app_info::DATA_VERSION.to_string());
    let lignes: Vec<String> = CLES
        .iter()
        .map(|c| format!("{c}={}", valeurs.get(*c).cloned().unwrap_or_default()))
        .collect();
    ecrire_lignes(&lignes);
}

/// Téléchargements simultanés (2 à 32, 16 par défaut).
pub fn telechargements() -> usize {
    get(TELECHARGEMENTS)
        .trim()
        .parse::<usize>()
        .unwrap_or(16)
        .clamp(2, 32)
}

/// Fête en cours, 3 jours avant et après : 1 Halloween (31 octobre), 2 Noël (25 décembre), 0 sinon.
pub fn saison_du_jour() -> i32 {
    use chrono::Datelike;
    let jour = chrono::Local::now().date_naive();
    saison(jour.month(), jour.day())
}

fn saison(mois: u32, jour: u32) -> i32 {
    match (mois, jour) {
        (10, 28..=31) | (11, 1..=3) => 1,
        (12, 22..=28) => 2,
        _ => 0,
    }
}

/// Flou du paysage en fond, de 0 (net) à 10 (2 par défaut : un léger voile de brume).
pub fn fond_flou() -> u32 {
    get(FOND_FLOU).trim().parse::<u32>().unwrap_or(2).min(10)
}

/// Thème choisi (0 brume, 1 nuit, 2 comme Windows).
pub fn theme() -> i32 {
    get(THEME)
        .trim()
        .parse::<i32>()
        .ok()
        .filter(|t| (0..=2).contains(t))
        .unwrap_or(0)
}

/// RAM choisie pour le jeu, en Mo.
pub fn ram_mo() -> u32 {
    get(RAM)
        .trim()
        .parse()
        .unwrap_or(app_info::RAM_PAR_DEFAUT_MO)
}

#[cfg(test)]
mod tests {
    #[test]
    fn dates_des_saisons() {
        assert_eq!(super::saison(10, 27), 0);
        assert_eq!(super::saison(10, 28), 1);
        assert_eq!(super::saison(10, 31), 1);
        assert_eq!(super::saison(11, 3), 1);
        assert_eq!(super::saison(11, 4), 0);
        assert_eq!(super::saison(12, 21), 0);
        assert_eq!(super::saison(12, 22), 2);
        assert_eq!(super::saison(12, 28), 2);
        assert_eq!(super::saison(12, 29), 0);
    }
}
