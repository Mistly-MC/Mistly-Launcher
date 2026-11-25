//! Réglages du launcher : `reglages.json` dans le dossier de données.
//!
//! Lecture : `reglages::lire().ram_mo`. Modification : `reglages::modifier(|r| r.ram_mo = 6144)`.
//!
//! Nouveau réglage : ajouter un champ à `Reglages` avec `#[serde(default)]` (ou
//! `#[serde(default = "…")]` si la valeur par défaut n'est pas celle du type). Rien d'autre à faire :
//! les anciens fichiers reçoivent la valeur par défaut, et les champs inconnus (écrits par une
//! version plus récente) sont ignorés.
//!
//! Migration : l'ancien `data.txt` (« cle=valeur », une clé par ligne, même format que la version
//! C#) est converti au premier démarrage, puis renommé en `data.txt.old`.

use crate::systeme::{app_info, journal, paths};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Façon de jouer choisie au premier lancement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ModeCompte {
    /// Pas encore choisie : l'écran d'accueil la demande
    #[default]
    NonChoisi,
    /// Compte Microsoft
    Premium,
    /// Un pseudo, sans compte
    HorsLigne,
}

impl ModeCompte {
    fn texte(self) -> &'static str {
        match self {
            ModeCompte::NonChoisi => "",
            ModeCompte::Premium => "premium",
            ModeCompte::HorsLigne => "horsligne",
        }
    }

    /// Valeur inconnue (version plus récente, fichier modifié à la main) : pas encore choisie.
    fn depuis(texte: &str) -> Self {
        match texte.trim() {
            "premium" => ModeCompte::Premium,
            "horsligne" => ModeCompte::HorsLigne,
            _ => ModeCompte::NonChoisi,
        }
    }
}

impl Serialize for ModeCompte {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.texte())
    }
}

impl<'de> Deserialize<'de> for ModeCompte {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let valeur = serde_json::Value::deserialize(d)?;
        Ok(ModeCompte::depuis(valeur.as_str().unwrap_or("")))
    }
}

fn vrai() -> bool {
    true
}
fn theme_par_defaut() -> i32 {
    1
}
/// Pas de pseudo d'office : le premier lancement fait choisir un compte (Microsoft ou un pseudo).
fn pseudo_par_defaut() -> String {
    String::new()
}
fn ram_par_defaut() -> u32 {
    app_info::RAM_PAR_DEFAUT_MO
}
fn fond_flou_par_defaut() -> u32 {
    2
}
fn telechargements_par_defaut() -> usize {
    16
}

/// Tous les réglages. Les noms entre parenthèses sont les anciennes clés de data.txt.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Reglages {
    /// Pseudo hors ligne utilisé en dernier (lastpseudo)
    #[serde(default = "pseudo_par_defaut")]
    pub pseudo: String,
    /// Version de Minecraft lancée en dernier en jeu rapide (lastversion)
    #[serde(default)]
    pub derniere_version: String,
    /// Mémoire donnée au jeu, en Mo (ram)
    #[serde(default = "ram_par_defaut")]
    pub ram_mo: u32,
    /// Connecté avec un compte Microsoft (premium)
    #[serde(default)]
    pub premium: bool,
    /// Raccourci du bureau créé par le joueur, mis à jour après chaque mise à jour (raccourci)
    #[serde(default)]
    pub raccourci: String,
    /// Dossier de l'ancienne version, supprimé au démarrage qui suit une mise à jour (Suppr)
    #[serde(default)]
    pub ancien_dossier: String,
    /// Profil choisi sur l'accueil (lastprofil)
    #[serde(default)]
    pub dernier_profil: String,
    /// Statut Discord (richpresence)
    #[serde(default = "vrai")]
    pub rich_presence: bool,
    /// Langue choisie ("" : pas encore choisie, l'anglais) (langue)
    #[serde(default)]
    pub langue: String,
    /// Apparence : 0 brume (clair), 1 nuit (sombre), 2 comme Windows (theme). Nuit pour une
    /// nouvelle installation ; un réglage déjà enregistré est gardé.
    #[serde(default = "theme_par_defaut")]
    pub theme: i32,
    /// Fenêtre floutée (flou)
    #[serde(default = "vrai")]
    pub flou: bool,
    /// Dessin de l'interface par la carte graphique (OpenGL). Désactivé par défaut : le dessin par
    /// le processeur utilise environ 150 Mo de mémoire de moins, pour la même image.
    #[serde(default)]
    pub acceleration_gpu: bool,
    /// Thèmes de saison automatiques : Halloween, Noël (saison)
    #[serde(default = "vrai")]
    pub saison: bool,
    /// Flou du paysage en fond, de 0 (net) à 10 (très flou) (fondflou)
    #[serde(default = "fond_flou_par_defaut")]
    pub fond_flou: u32,
    /// Build d'Horizon lancé par JOUER, tag GitHub (horizonbuild)
    #[serde(default)]
    pub horizon_build: String,
    /// Fichiers téléchargés en même temps pendant l'installation du jeu, de 2 à 32 (telechargements)
    #[serde(default = "telechargements_par_defaut")]
    pub telechargements: usize,
    /// Console du jeu ouverte au lancement d'une partie (console)
    #[serde(default = "vrai")]
    pub console: bool,
    /// Télémétrie anonyme : None pas encore demandé, Some(true) acceptée, Some(false) refusée (telemetrie)
    #[serde(default)]
    pub telemetrie: Option<bool>,
    /// Identifiant d'installation aléatoire, envoyé avec la télémétrie (telemetrieid)
    #[serde(default)]
    pub telemetrie_id: String,
    /// Mises à jour : proposer aussi les pré-releases (bêtas) de GitHub (majprerelease)
    #[serde(default)]
    pub maj_prerelease: bool,
    /// Mistly Client en jeu sur les profils compatibles (mistyclient)
    #[serde(default = "vrai")]
    pub misty_client: bool,
    /// Cosmétique équipé : "" aucun, "misty_cape", "misty_elytra" (voir minecraft::client) (cosmetique)
    #[serde(default)]
    pub cosmetique: String,
    /// Chapeau porté : "" aucun, "cloudly" (voir minecraft::client) (chapeau)
    #[serde(default)]
    pub chapeau: String,
    /// Ailes de brume (modèle du site), portées en plus de la cape
    #[serde(default)]
    pub ailes_brume: bool,
    /// Couleur des ailes de brume : un préréglage du site (« misty »…), et des couleurs libres
    /// (« #rrggbb ») pour la membrane et les os ("" : celles du préréglage)
    #[serde(default)]
    pub ailes_teinte: String,
    #[serde(default)]
    pub ailes_membrane: String,
    #[serde(default)]
    pub ailes_os: String,
    /// JOUER lance la version Mistly (Fabric + Mistly Client). None : pas encore choisi (mistychoisi)
    #[serde(default)]
    pub misty_choisi: Option<bool>,
    /// Version de Minecraft choisie dans la section Mistly Client, ex. "26.3" (mistyversion)
    #[serde(default)]
    pub misty_version: String,
    /// Édition choisie dans la barre du haut : 0 Mistly Client · 1 Java · 2 Bedrock · 3 Horizon
    #[serde(default)]
    pub edition: u8,
    /// Serveurs et barres d'objets partagés entre toutes les versions et tous les profils (partage)
    #[serde(default = "vrai")]
    pub partage: bool,
    /// Onglets masqués de la barre latérale, ex. ["skin", "notes"] (barremasquee)
    #[serde(default)]
    pub barre_masquee: Vec<String>,
    /// Façon de jouer choisie au premier lancement (modecompte)
    #[serde(default)]
    pub mode_compte: ModeCompte,
    /// Style du pseudo au-dessus de la tête : "" classique, ou voir minecraft::client::TAGS (tag)
    #[serde(default)]
    pub tag: String,
    /// Premier lancement terminé : langue choisie, licence et conditions acceptées
    #[serde(default)]
    pub accueil_fait: bool,
    /// Taille de la fenêtre du jeu en pixels (0 : celle du jeu)
    #[serde(default)]
    pub fenetre_largeur: u32,
    #[serde(default)]
    pub fenetre_hauteur: u32,
    /// Le jeu s'ouvre en plein écran (--fullscreen)
    #[serde(default)]
    pub plein_ecran: bool,
    /// Avant chaque partie, demander quel compte utiliser
    #[serde(default)]
    pub demander_compte: bool,
    /// Pas de téléchargement en arrière-plan pendant une partie (repris à la fin de la partie)
    #[serde(default = "vrai")]
    pub pause_telechargements: bool,
}

impl Default for Reglages {
    /// Les valeurs par défaut sont celles de `#[serde(default)]` : une seule source.
    fn default() -> Self {
        serde_json::from_str("{}").expect("réglages par défaut")
    }
}

impl Reglages {
    /// Ramène les valeurs dans leurs limites (fichier modifié à la main, ancienne version).
    fn normaliser(&mut self) {
        if !(0..=2).contains(&self.theme) {
            self.theme = 0;
        }
        self.fond_flou = self.fond_flou.min(10);
        self.telechargements = self.telechargements.clamp(2, 32);
        // fenêtre du jeu : 0 (taille du jeu) ou une taille raisonnable
        if self.fenetre_largeur != 0 || self.fenetre_hauteur != 0 {
            self.fenetre_largeur = self.fenetre_largeur.clamp(320, 7680);
            self.fenetre_hauteur = self.fenetre_hauteur.clamp(240, 4320);
        }
    }

    /// Convertit le contenu de l'ancien data.txt (mêmes règles que l'ancien `settings::get_bool`,
    /// etc. : une clé vide ou illisible prend la valeur par défaut).
    pub fn depuis_data_txt(texte: &str) -> Reglages {
        let mut valeurs: HashMap<&str, &str> = HashMap::new();
        for ligne in texte.trim_start_matches('\u{feff}').lines() {
            // la dernière occurrence d'une clé l'emporte, comme dans l'ancienne lecture
            if let Some((cle, valeur)) = ligne.split_once('=') {
                valeurs.insert(cle, valeur.trim_end_matches('\r'));
            }
        }
        let texte = |cle: &str| valeurs.get(cle).copied().unwrap_or("").to_string();
        let defaut = Reglages::default();
        let booleen = |cle: &str, defaut: bool| match valeurs.get(cle).map(|v| v.trim()) {
            None | Some("") => defaut,
            Some(v) => v.eq_ignore_ascii_case("true"),
        };
        let choix = |cle: &str| match valeurs.get(cle).map(|v| v.trim()) {
            None | Some("") => None,
            Some(v) => Some(v.eq_ignore_ascii_case("true")),
        };
        let nombre = |cle: &str| valeurs.get(cle).and_then(|v| v.trim().parse::<i64>().ok());
        let mut r = Reglages {
            // pas de clé du tout : le pseudo par défaut ; une clé vide reste vide
            pseudo: valeurs
                .get("lastpseudo")
                .map(|v| v.to_string())
                .unwrap_or(defaut.pseudo.clone()),
            derniere_version: texte("lastversion"),
            ram_mo: nombre("ram")
                .and_then(|n| u32::try_from(n).ok())
                .unwrap_or(defaut.ram_mo),
            premium: booleen("premium", defaut.premium),
            raccourci: texte("raccourci"),
            ancien_dossier: texte("Suppr"),
            dernier_profil: texte("lastprofil"),
            rich_presence: booleen("richpresence", defaut.rich_presence),
            langue: texte("langue"),
            theme: nombre("theme")
                .and_then(|n| i32::try_from(n).ok())
                .unwrap_or(defaut.theme),
            flou: booleen("flou", defaut.flou),
            saison: booleen("saison", defaut.saison),
            fond_flou: nombre("fondflou")
                .and_then(|n| u32::try_from(n).ok())
                .unwrap_or(defaut.fond_flou),
            horizon_build: texte("horizonbuild"),
            telechargements: nombre("telechargements")
                .and_then(|n| usize::try_from(n).ok())
                .unwrap_or(defaut.telechargements),
            console: booleen("console", defaut.console),
            telemetrie: choix("telemetrie"),
            telemetrie_id: texte("telemetrieid").trim().to_string(),
            maj_prerelease: booleen("majprerelease", defaut.maj_prerelease),
            misty_client: booleen("mistyclient", defaut.misty_client),
            cosmetique: texte("cosmetique"),
            chapeau: texte("chapeau"),
            misty_choisi: choix("mistychoisi"),
            misty_version: texte("mistyversion"),
            edition: 0,
            partage: booleen("partage", defaut.partage),
            barre_masquee: texte("barremasquee")
                .split(',')
                .map(|o| o.trim().to_string())
                .filter(|o| !o.is_empty())
                .collect(),
            mode_compte: ModeCompte::depuis(&texte("modecompte")),
            tag: texte("tag"),
            ..defaut.clone()
        };
        r.normaliser();
        r
    }
}

const FICHIER: &str = "reglages.json";
const ANCIEN_FICHIER: &str = "data.txt";
const ANCIEN_FICHIER_MIGRE: &str = "data.txt.old";

fn ecrire_dans(dossier: &Path, r: &Reglages) -> std::io::Result<()> {
    let texte = serde_json::to_string_pretty(r).map_err(std::io::Error::other)?;
    crate::systeme::fichiers::ecrire_atomique(dossier.join(FICHIER), texte)
}

/// Lit les réglages de `dossier`, en migrant data.txt s'il le faut. Ne perd jamais de réglages :
/// - reglages.json existe : il est lu (un data.txt encore là, migration interrompue, est renommé) ;
/// - sinon data.txt est converti, reglages.json écrit, puis data.txt renommé en data.txt.old ;
/// - reglages.json illisible : il est mis de côté (reglages.json.invalide) et les réglages
///   repartent de data.txt.old s'il existe, sinon des valeurs par défaut.
pub fn charger_depuis(dossier: &Path) -> Reglages {
    let json = dossier.join(FICHIER);
    let ancien = dossier.join(ANCIEN_FICHIER);
    let migre = dossier.join(ANCIEN_FICHIER_MIGRE);

    if let Ok(texte) = std::fs::read_to_string(&json) {
        match serde_json::from_str::<Reglages>(texte.trim_start_matches('\u{feff}')) {
            Ok(mut r) => {
                r.normaliser();
                if ancien.exists() {
                    let _ = std::fs::rename(&ancien, &migre);
                }
                return r;
            }
            Err(e) => {
                journal::info(&format!("{FICHIER} illisible ({e}) : mis de côté"));
                let _ = std::fs::rename(&json, dossier.join(format!("{FICHIER}.invalide")));
                if let Ok(texte) = std::fs::read_to_string(&migre) {
                    let r = Reglages::depuis_data_txt(&texte);
                    let _ = ecrire_dans(dossier, &r);
                    return r;
                }
            }
        }
    }

    match std::fs::read_to_string(&ancien) {
        Ok(texte) => {
            let r = Reglages::depuis_data_txt(&texte);
            // data.txt n'est renommé qu'une fois reglages.json bien écrit
            match ecrire_dans(dossier, &r) {
                Ok(()) => {
                    let _ = std::fs::rename(&ancien, &migre);
                    journal::info("Réglages : data.txt converti en reglages.json");
                }
                Err(e) => journal::info(&format!(
                    "Réglages : {FICHIER} impossible à écrire ({e}), data.txt gardé"
                )),
            }
            r
        }
        // premier démarrage
        Err(_) => Reglages::default(),
    }
}

/// Réglages gardés en mémoire : ils sont lus des dizaines de fois (à chaque statut Discord, à
/// chaque lancement…). Le fichier n'est relu que s'il a changé sur le disque (un autre processus
/// de Misty, comme l'ancienne version pendant une mise à jour, peut l'avoir écrit).
struct Cache {
    chemin: PathBuf,
    /// Date et taille du fichier au moment de la lecture (None : pas encore de fichier)
    empreinte: Option<(Option<std::time::SystemTime>, u64)>,
    reglages: Reglages,
}

static CACHE: Mutex<Option<Cache>> = Mutex::new(None);

fn empreinte(chemin: &Path) -> Option<(Option<std::time::SystemTime>, u64)> {
    let meta = std::fs::metadata(chemin).ok()?;
    Some((meta.modified().ok(), meta.len()))
}

/// Réglages à jour, sous le verrou du cache.
fn actuels(cache: &mut Option<Cache>) -> Reglages {
    let dossier = paths::data_dir();
    let chemin = dossier.join(FICHIER);
    let actuelle = empreinte(&chemin);
    if let Some(c) = cache.as_ref() {
        if c.chemin == chemin && c.empreinte == actuelle && actuelle.is_some() {
            return c.reglages.clone();
        }
    }
    let reglages = charger_depuis(&dossier);
    *cache = Some(Cache {
        empreinte: empreinte(&chemin),
        chemin,
        reglages: reglages.clone(),
    });
    reglages
}

/// Les réglages actuels.
pub fn lire() -> Reglages {
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    actuels(&mut cache)
}

/// Modifie les réglages et les enregistre aussitôt (écriture atomique).
pub fn modifier(changement: impl FnOnce(&mut Reglages)) {
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let mut r = actuels(&mut cache);
    changement(&mut r);
    r.normaliser();
    let dossier = paths::data_dir();
    let _ = std::fs::create_dir_all(&dossier);
    if let Err(e) = ecrire_dans(&dossier, &r) {
        journal::info(&format!("Réglages non enregistrés : {e}"));
    }
    let chemin = dossier.join(FICHIER);
    *cache = Some(Cache {
        empreinte: empreinte(&chemin),
        chemin,
        reglages: r,
    });
}

/// Au démarrage : migre data.txt s'il le faut, et crée reglages.json au premier lancement.
pub fn initialiser() {
    let dossier = paths::data_dir();
    let _ = std::fs::create_dir_all(&dossier);
    let r = lire();
    if !dossier.join(FICHIER).exists() {
        let _ = ecrire_dans(&dossier, &r);
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Un vrai data.txt, écrit par Misty 0.3.9 (clés dans l'ordre de l'ancien CLES, fins de ligne
    /// Windows), avec un compte Microsoft, un raccourci et une mise à jour en cours.
    const DATA_TXT: &str = "versiondatatxt=1.3.5\r\n\
lastpseudo=EnderMythex\r\n\
lastversion=1.21.4\r\n\
ram=6144\r\n\
premium=True\r\n\
raccourci=C:\\Users\\Joueur\\Desktop\\Mistly.lnk\r\n\
Suppr=C:\\Users\\Joueur\\AppData\\Local\\Programs\\Misty\\Misty-0.3.8\r\n\
lastprofil=3f2a9c1e\r\n\
richpresence=false\r\n\
langue=fr\r\n\
theme=1\r\n\
flou=true\r\n\
saison=False\r\n\
fondflou=4\r\n\
horizonbuild=v0.9.2\r\n\
telechargements=24\r\n\
console=\r\n\
telemetrie=true\r\n\
telemetrieid=6b1f4c1e-8a3d-4f3e-9d47-1c2b3a4d5e6f\r\n\
majprerelease=true\r\n\
mistyclient=true\r\n\
cosmetique=misty_cape\r\n\
chapeau=cloudly\r\n\
mistychoisi=True\r\n\
mistyversion=26.3\r\n\
partage=false\r\n\
barremasquee=skin, notes\r\n\
modecompte=premium\r\n\
tag=arc-en-ciel\r\n";

    fn dossier_test(nom: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("misty-reglages-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn attendus() -> Reglages {
        Reglages {
            pseudo: "EnderMythex".into(),
            derniere_version: "1.21.4".into(),
            ram_mo: 6144,
            premium: true,
            raccourci: r"C:\Users\Joueur\Desktop\Mistly.lnk".into(),
            ancien_dossier: r"C:\Users\Joueur\AppData\Local\Programs\Misty\Misty-0.3.8".into(),
            dernier_profil: "3f2a9c1e".into(),
            rich_presence: false,
            langue: "fr".into(),
            theme: 1,
            flou: true,
            saison: false,
            fond_flou: 4,
            horizon_build: "v0.9.2".into(),
            telechargements: 24,
            console: true,
            telemetrie: Some(true),
            telemetrie_id: "6b1f4c1e-8a3d-4f3e-9d47-1c2b3a4d5e6f".into(),
            maj_prerelease: true,
            misty_client: true,
            cosmetique: "misty_cape".into(),
            chapeau: "cloudly".into(),
            misty_choisi: Some(true),
            misty_version: "26.3".into(),
            edition: 0,
            partage: false,
            barre_masquee: vec!["skin".into(), "notes".into()],
            mode_compte: ModeCompte::Premium,
            tag: "arc-en-ciel".into(),
            ..Reglages::default()
        }
    }

    #[test]
    fn migration_d_un_vrai_data_txt() {
        let d = dossier_test("migration");
        std::fs::write(d.join("data.txt"), DATA_TXT).unwrap();

        let r = charger_depuis(&d);
        assert_eq!(r, attendus());
        // data.txt renommé, reglages.json écrit
        assert!(!d.join("data.txt").exists());
        assert_eq!(
            std::fs::read_to_string(d.join("data.txt.old")).unwrap(),
            DATA_TXT
        );
        assert!(d.join("reglages.json").exists());
        // démarrage suivant : relu depuis reglages.json, identique
        assert_eq!(charger_depuis(&d), attendus());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn migration_interrompue_reprise() {
        // reglages.json écrit mais data.txt pas encore renommé : reglages.json fait foi
        let d = dossier_test("interrompue");
        std::fs::write(d.join("data.txt"), DATA_TXT).unwrap();
        ecrire_dans(&d, &attendus()).unwrap();
        std::fs::write(d.join("data.txt"), "ram=1024\r\n").unwrap();
        assert_eq!(charger_depuis(&d).ram_mo, 6144);
        assert!(!d.join("data.txt").exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn data_txt_vide_ou_minimal() {
        // data.txt créé au tout premier lancement par l'ancienne version
        // (son pseudo « Mistly » est gardé : c'est celui du joueur)
        let r = Reglages::depuis_data_txt("versiondatatxt=1.3.5\r\nlastpseudo=Mistly\r\n");
        assert_eq!(
            r,
            Reglages {
                pseudo: "Mistly".into(),
                ..Reglages::default()
            }
        );
        let r = Reglages::depuis_data_txt("");
        assert_eq!(r, Reglages::default());
        // valeurs hors limites ou illisibles : ramenées dans les limites ou par défaut
        let r = Reglages::depuis_data_txt(
            "theme=7\r\nfondflou=50\r\ntelechargements=1\r\nram=beaucoup\r\nmodecompte=autre\r\n",
        );
        assert_eq!((r.theme, r.fond_flou, r.telechargements), (0, 10, 2));
        assert_eq!(r.ram_mo, app_info::RAM_PAR_DEFAUT_MO);
        assert_eq!(r.mode_compte, ModeCompte::NonChoisi);
        // un pseudo vide reste vide (le joueur l'a effacé)
        assert_eq!(Reglages::depuis_data_txt("lastpseudo=\r\n").pseudo, "");
    }

    #[test]
    fn valeurs_par_defaut_et_champs_inconnus() {
        let d = Reglages::default();
        assert_eq!(d.pseudo, "");
        assert_eq!(d.ram_mo, app_info::RAM_PAR_DEFAUT_MO);
        assert!(d.rich_presence && d.flou && d.saison && d.console && d.misty_client && d.partage);
        assert_eq!((d.fond_flou, d.telechargements), (2, 16));
        assert_eq!((d.telemetrie, d.misty_choisi), (None, None));
        // fichier d'une version plus récente : champ inconnu et valeur d'énumération inconnue
        let r: Reglages = serde_json::from_str(
            r#"{ "ram_mo": 8192, "nouveau_reglage": [1, 2], "mode_compte": "invite" }"#,
        )
        .unwrap();
        assert_eq!(r.ram_mo, 8192);
        assert_eq!(r.mode_compte, ModeCompte::NonChoisi);
        assert_eq!(r.pseudo, "");
    }

    #[test]
    fn reglages_json_illisible_repart_de_data_txt_old() {
        let d = dossier_test("illisible");
        std::fs::write(d.join("reglages.json"), "{ pas du json").unwrap();
        std::fs::write(d.join("data.txt.old"), DATA_TXT).unwrap();
        assert_eq!(charger_depuis(&d), attendus());
        assert!(d.join("reglages.json.invalide").exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn premier_demarrage() {
        let d = dossier_test("premier");
        assert_eq!(charger_depuis(&d), Reglages::default());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn dates_des_saisons() {
        assert_eq!(saison(10, 27), 0);
        assert_eq!(saison(10, 28), 1);
        assert_eq!(saison(10, 31), 1);
        assert_eq!(saison(11, 3), 1);
        assert_eq!(saison(11, 4), 0);
        assert_eq!(saison(12, 21), 0);
        assert_eq!(saison(12, 22), 2);
        assert_eq!(saison(12, 28), 2);
        assert_eq!(saison(12, 29), 0);
    }
}
