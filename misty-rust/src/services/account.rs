//! Session Microsoft (premium).
//!
//! Chaîne d'authentification (ce que faisait CmlLib.Core.Auth.Microsoft en C#) :
//!   1. Connexion Microsoft dans une fenêtre web → code
//!   2. code → jeton Microsoft (+ jeton de renouvellement, mis en cache)
//!   3. jeton Microsoft → jeton Xbox Live → jeton XSTS
//!   4. XSTS → jeton Minecraft → profil (pseudo + UUID)
//!
//! La fenêtre web tourne dans un sous-processus (`Misty.exe --ms-login`), car l'interface Slint
//! et la WebView ne peuvent pas partager la même boucle d'événements.

use super::coffre::{self, Coffre, Secrets};
use super::{http, loc, paths, settings};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::RwLock;

/// Identifiant du launcher Minecraft officiel (accepté par l'API Minecraft sans validation).
pub const CLIENT_ID: &str = "00000000402b5328";
pub const REDIRECT: &str = "https://login.live.com/oauth20_desktop.srf";
const SCOPE: &str = "service::user.auth.xboxlive.com::MBI_SSL";

pub fn url_connexion() -> String {
    format!(
        "https://login.live.com/oauth20_authorize.srf?client_id={CLIENT_ID}&response_type=code&scope={}&redirect_uri={}&prompt=select_account",
        urlencoding::encode(SCOPE),
        urlencoding::encode(REDIRECT)
    )
}

/// Session de jeu Minecraft.
#[derive(Clone, Debug)]
pub struct Session {
    pub pseudo: String,
    pub uuid: String,
    pub jeton: String,
    pub xuid: String,
}

/// Un compte Microsoft mis en cache.
///
/// Les jetons ne sont jamais écrits dans msa.json : ils vont dans le coffre (Gestionnaire
/// d'identifiants Windows, voir `coffre`). Les anciens msa.json qui en contiennent encore sont
/// migrés à la première lecture.
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
struct Cache {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    refresh_token: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    mc_token: String,
    #[serde(default)]
    mc_expire: i64,
    #[serde(default)]
    pseudo: String,
    #[serde(default)]
    uuid: String,
    #[serde(default)]
    xuid: String,
}

impl Cache {
    fn secrets(&self) -> Secrets {
        Secrets {
            refresh_token: self.refresh_token.clone(),
            mc_token: self.mc_token.clone(),
        }
    }
}

/// Contenu de DataDir\msa.json : tous les comptes + celui qui est utilisé.
/// (L'ancien format, avec un seul compte, est encore lu.)
#[derive(Serialize, Deserialize, Default, Debug, PartialEq)]
struct Fichier {
    #[serde(default)]
    actif: String,
    #[serde(default)]
    comptes: Vec<Cache>,
}

/// Compte affiché dans les listes.
#[derive(Clone, Debug)]
pub struct CompteInfo {
    pub pseudo: String,
    pub uuid: String,
    pub actif: bool,
}

static SESSION: RwLock<Option<Session>> = RwLock::new(None);

pub fn session() -> Option<Session> {
    SESSION.read().unwrap().clone()
}

fn fichier_cache() -> std::path::PathBuf {
    paths::data_dir().join("msa.json")
}

/// Lit msa.json (nouveau ou ancien format), sans les jetons.
fn analyser(texte: &str) -> Fichier {
    if let Ok(f) = serde_json::from_str::<Fichier>(texte) {
        if !f.comptes.is_empty() {
            return f;
        }
    }
    // Ancien format : un seul compte à la racine
    match serde_json::from_str::<Cache>(texte) {
        Ok(c) if !c.uuid.is_empty() => Fichier {
            actif: c.uuid.clone(),
            comptes: vec![c],
        },
        _ => Fichier::default(),
    }
}

/// Range dans le coffre les jetons encore présents dans `f` et les retire de `f`.
/// Un compte dont le coffre refuse les jetons les garde dans le fichier : rien n'est perdu.
/// Renvoie true si `f` a changé.
fn ranger_jetons(f: &mut Fichier, coffre: &dyn Coffre) -> bool {
    let mut change = false;
    for c in f.comptes.iter_mut().filter(|c| !c.secrets().est_vide()) {
        match coffre.ecrire(&c.uuid, &c.secrets()) {
            Ok(()) => {
                c.refresh_token.clear();
                c.mc_token.clear();
                change = true;
            }
            Err(e) => super::journal::info(&format!(
                "Jetons du compte {} gardés dans msa.json (coffre indisponible : {e})",
                c.pseudo
            )),
        }
    }
    change
}

/// Lecture de msa.json, avec la migration des anciens fichiers : s'il contient encore des jetons,
/// ils passent dans le coffre. Renvoie le fichier sans jetons, et son nouveau texte s'il faut le
/// réécrire.
fn lire_et_migrer(texte: &str, coffre: &dyn Coffre) -> (Fichier, Option<String>) {
    let mut f = analyser(texte);
    let reecrire = ranger_jetons(&mut f, coffre)
        .then(|| serde_json::to_string_pretty(&f).ok())
        .flatten();
    (f, reecrire)
}

fn lire_fichier() -> Fichier {
    let Ok(texte) = std::fs::read_to_string(fichier_cache()) else {
        return Fichier::default();
    };
    let (f, reecrire) = lire_et_migrer(&texte, coffre::coffre());
    if let Some(nouveau) = reecrire {
        if super::fichiers::ecrire_atomique(fichier_cache(), nouveau).is_ok() {
            super::journal::info("Jetons Microsoft déplacés de msa.json vers le coffre de Windows");
        }
    }
    f
}

/// Au démarrage : migre un ancien msa.json qui contient encore des jetons.
pub fn migrer_jetons() {
    let _ = lire_fichier();
}

/// Écrit msa.json. Les jetons des comptes de `f` vont d'abord dans le coffre (puis le fichier est
/// écrit sans eux) : si le launcher s'arrête entre les deux, la prochaine lecture finit la migration.
fn ecrire_fichier(f: &Fichier) {
    let mut sans_jetons = Fichier {
        actif: f.actif.clone(),
        comptes: f.comptes.clone(),
    };
    ranger_jetons(&mut sans_jetons, coffre::coffre());
    let _ = super::fichiers::ecrire_atomique(
        fichier_cache(),
        serde_json::to_string_pretty(&sans_jetons).unwrap_or_default(),
    );
}

/// Le compte avec ses jetons, relus dans le coffre.
fn avec_jetons(mut c: Cache, coffre: &dyn Coffre) -> Cache {
    if c.secrets().est_vide() {
        if let Some(s) = coffre.lire(&c.uuid) {
            c.refresh_token = s.refresh_token;
            c.mc_token = s.mc_token;
        }
    }
    c
}

/// Ajoute ou met à jour un compte, et le rend actif.
fn enregistrer(cache: Cache) {
    let mut f = lire_fichier();
    f.actif = cache.uuid.clone();
    match f.comptes.iter_mut().find(|c| c.uuid == cache.uuid) {
        Some(existant) => *existant = cache,
        None => f.comptes.push(cache),
    }
    ecrire_fichier(&f);
}

/// Compte actif en cache (sinon le premier).
fn compte_actif() -> Option<Cache> {
    let f = lire_fichier();
    f.comptes
        .iter()
        .find(|c| c.uuid == f.actif)
        .or_else(|| f.comptes.first())
        .cloned()
        .map(|c| avec_jetons(c, coffre::coffre()))
}

/// Tous les comptes Microsoft connus.
pub fn comptes() -> Vec<CompteInfo> {
    let f = lire_fichier();
    let actif = f
        .comptes
        .iter()
        .find(|c| c.uuid == f.actif)
        .or_else(|| f.comptes.first())
        .map(|c| c.uuid.clone())
        .unwrap_or_default();
    f.comptes
        .iter()
        .map(|c| CompteInfo {
            pseudo: c.pseudo.clone(),
            uuid: c.uuid.clone(),
            actif: c.uuid == actif,
        })
        .collect()
}

/// Choisit le compte à utiliser, puis s'y connecte.
pub async fn activer(uuid: &str) -> anyhow::Result<Session> {
    let mut f = lire_fichier();
    if f.comptes.iter().any(|c| c.uuid == uuid) {
        f.actif = uuid.to_string();
        ecrire_fichier(&f);
    }
    connecter().await
}

/// Ajoute un autre compte : ouvre toujours la fenêtre de connexion.
pub async fn ajouter() -> anyhow::Result<Session> {
    let session = connexion_interactive().await?;
    *SESSION.write().unwrap() = Some(session.clone());
    settings::set(settings::PREMIUM, "True");
    Ok(session)
}

/// Oublie un seul compte. S'il était connecté, on se déconnecte.
pub fn oublier(uuid: &str) {
    let mut f = lire_fichier();
    f.comptes.retain(|c| c.uuid != uuid);
    coffre::coffre().supprimer(uuid);
    if f.actif == uuid {
        f.actif = f
            .comptes
            .first()
            .map(|c| c.uuid.clone())
            .unwrap_or_default();
    }
    if f.comptes.is_empty() {
        let _ = std::fs::remove_file(fichier_cache());
    } else {
        ecrire_fichier(&f);
    }
    if session().is_some_and(|s| s.uuid == uuid) {
        deconnecter();
    }
}

/// Dossier des données de la fenêtre de connexion (cookies du compte Microsoft).
pub fn dossier_webview() -> std::path::PathBuf {
    paths::data_dir().join("webview")
}

/// Connexion : session en cache si elle est encore valide, sinon renouvellement, sinon fenêtre de connexion.
pub async fn connecter() -> anyhow::Result<Session> {
    let maintenant = chrono::Utc::now().timestamp();
    let cache = compte_actif();

    let session = match cache {
        Some(c) if !c.mc_token.is_empty() && c.mc_expire > maintenant + 300 => Session {
            pseudo: c.pseudo,
            uuid: c.uuid,
            jeton: c.mc_token,
            xuid: c.xuid,
        },
        Some(c) if !c.refresh_token.is_empty() => match renouveler(&c.refresh_token).await {
            Ok(s) => s,
            Err(_) => connexion_interactive().await?, // jeton expiré ou révoqué
        },
        _ => connexion_interactive().await?,
    };

    *SESSION.write().unwrap() = Some(session.clone());
    settings::set(settings::PREMIUM, "True");
    Ok(session)
}

/// Avant de lancer le jeu : la session Microsoft, avec un jeton qui tiendra toute la partie.
///
/// Le jeton de Minecraft dure 24 h et le jeu le garde jusqu'à sa fermeture. Celui de la session
/// date de l'ouverture du launcher : si le launcher est resté ouvert longtemps, le jeu partait avec
/// un jeton près d'expirer (ou expiré), et rejoindre un serveur affichait « Invalid session ».
/// Il est donc renouvelé s'il lui reste moins de 12 h. Si le renouvellement échoue (hors ligne),
/// la session actuelle est gardée.
pub async fn session_pour_jouer() -> Option<Session> {
    const MARGE: i64 = 12 * 3600;
    let session = session()?;
    let maintenant = chrono::Utc::now().timestamp();
    let f = lire_fichier();
    let Some(cache) = f.comptes.iter().find(|c| c.uuid == session.uuid).cloned() else {
        return Some(session);
    };
    let cache = avec_jetons(cache, coffre::coffre());
    // le jeton du cache (le même, ou un plus récent renouvelé entre-temps) tient encore assez
    if cache.mc_expire > maintenant + MARGE && !cache.mc_token.is_empty() {
        let s = Session {
            pseudo: cache.pseudo,
            uuid: cache.uuid,
            jeton: cache.mc_token,
            xuid: cache.xuid,
        };
        *SESSION.write().unwrap() = Some(s.clone());
        return Some(s);
    }
    if cache.refresh_token.is_empty() {
        return Some(session);
    }
    match renouveler(&cache.refresh_token).await {
        Ok(s) if s.uuid == session.uuid => {
            super::journal::info("Jeton Minecraft renouvelé avant le lancement");
            *SESSION.write().unwrap() = Some(s.clone());
            Some(s)
        }
        Ok(_) => Some(session),
        Err(e) => {
            super::journal::info(&format!(
                "Jeton Minecraft non renouvelé ({e}) : la session actuelle est gardée"
            ));
            Some(session)
        }
    }
}

/// Déconnexion rapide : le compte reste en cache pour la prochaine connexion.
pub fn deconnecter() {
    *SESSION.write().unwrap() = None;
    settings::set(settings::PREMIUM, "False");
}

/// Déconnexion totale : supprime aussi le compte mis en cache.
pub fn deconnexion_totale() -> anyhow::Result<()> {
    for c in lire_fichier().comptes {
        coffre::coffre().supprimer(&c.uuid);
    }
    let _ = std::fs::remove_file(fichier_cache());
    if dossier_webview().exists() {
        std::fs::remove_dir_all(dossier_webview())?;
    }
    deconnecter();
    Ok(())
}

/// Ouvre la fenêtre de connexion (sous-processus) et attend le code.
async fn connexion_interactive() -> anyhow::Result<Session> {
    let sortie = tokio::process::Command::new(paths::app_exe())
        .arg("--ms-login")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .await?;
    let texte = String::from_utf8_lossy(&sortie.stdout);
    let code = texte
        .lines()
        .find_map(|l| l.strip_prefix("CODE:"))
        .map(str::trim)
        .map(str::to_string);
    let Some(code) = code else {
        if let Some(erreur) = texte.lines().find_map(|l| l.strip_prefix("ERREUR:")) {
            anyhow::bail!("{}", erreur.trim());
        }
        anyhow::bail!(loc::t("Compte_Annule"));
    };
    let jetons = jeton_microsoft(&[("code", &code), ("grant_type", "authorization_code")]).await?;
    terminer(jetons).await
}

async fn renouveler(refresh_token: &str) -> anyhow::Result<Session> {
    let jetons = jeton_microsoft(&[
        ("refresh_token", refresh_token),
        ("grant_type", "refresh_token"),
    ])
    .await?;
    terminer(jetons).await
}

#[derive(Deserialize)]
struct JetonsMs {
    access_token: String,
    #[serde(default)]
    refresh_token: String,
}

async fn jeton_microsoft(parametres: &[(&str, &str)]) -> anyhow::Result<JetonsMs> {
    let mut formulaire: Vec<(&str, &str)> = vec![
        ("client_id", CLIENT_ID),
        ("redirect_uri", REDIRECT),
        ("scope", SCOPE),
    ];
    formulaire.extend_from_slice(parametres);
    let reponse = http::client()
        .post("https://login.live.com/oauth20_token.srf")
        .form(&formulaire)
        .send()
        .await?;
    if !reponse.status().is_success() {
        anyhow::bail!("Microsoft : {}", reponse.text().await.unwrap_or_default());
    }
    Ok(reponse.json().await?)
}

/// Étapes Xbox Live → XSTS → Minecraft → profil, puis mise en cache.
async fn terminer(jetons: JetonsMs) -> anyhow::Result<Session> {
    let client = http::client();

    // Xbox Live. Selon le compte, le ticket doit être préfixé ("t=" / "d=") ou non.
    let mut xbl: Option<serde_json::Value> = None;
    for prefixe in ["", "t=", "d="] {
        let corps = json!({
            "Properties": { "AuthMethod": "RPS", "SiteName": "user.auth.xboxlive.com", "RpsTicket": format!("{prefixe}{}", jetons.access_token) },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        });
        let r = client
            .post("https://user.auth.xboxlive.com/user/authenticate")
            .header("Accept", "application/json")
            .json(&corps)
            .send()
            .await?;
        if r.status().is_success() {
            xbl = Some(r.json().await?);
            break;
        }
    }
    let xbl = xbl.ok_or_else(|| anyhow::anyhow!("Xbox Live : authentification refusée."))?;
    let jeton_xbl = xbl["Token"].as_str().unwrap_or_default();

    // XSTS
    let corps = json!({
        "Properties": { "SandboxId": "RETAIL", "UserTokens": [jeton_xbl] },
        "RelyingParty": "rp://api.minecraftservices.com/",
        "TokenType": "JWT"
    });
    let r = client
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .header("Accept", "application/json")
        .json(&corps)
        .send()
        .await?;
    if !r.status().is_success() {
        let erreur: serde_json::Value = r.json().await.unwrap_or_default();
        let message = match erreur["XErr"].as_i64() {
            Some(2148916233) => loc::t("Compte_PasDeXbox"),
            Some(2148916235) => loc::t("Compte_PaysInterdit"),
            Some(2148916236) | Some(2148916237) => loc::t("Compte_VerifAge"),
            Some(2148916238) => loc::t("Compte_Enfant"),
            _ => format!("XSTS : {erreur}"),
        };
        anyhow::bail!(message);
    }
    let xsts: serde_json::Value = r.json().await?;
    let jeton_xsts = xsts["Token"].as_str().unwrap_or_default();
    let uhs = xsts["DisplayClaims"]["xui"][0]["uhs"]
        .as_str()
        .unwrap_or_default();
    let xuid = xsts["DisplayClaims"]["xui"][0]["xid"]
        .as_str()
        .unwrap_or_default()
        .to_string();

    // Minecraft
    let r = client
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&json!({ "identityToken": format!("XBL3.0 x={uhs};{jeton_xsts}") }))
        .send()
        .await?;
    if !r.status().is_success() {
        anyhow::bail!("Minecraft : {}", r.text().await.unwrap_or_default());
    }
    let mc: serde_json::Value = r.json().await?;
    let jeton_mc = mc["access_token"].as_str().unwrap_or_default().to_string();
    let expire = chrono::Utc::now().timestamp() + mc["expires_in"].as_i64().unwrap_or(86400);

    // Profil (404 = le compte ne possède pas Minecraft Java)
    let r = client
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(&jeton_mc)
        .send()
        .await?;
    if r.status() == reqwest::StatusCode::NOT_FOUND {
        anyhow::bail!(loc::t("Compte_PasDeJeu"));
    }
    let profil: serde_json::Value = r.error_for_status()?.json().await?;
    let session = Session {
        pseudo: profil["name"].as_str().unwrap_or_default().to_string(),
        uuid: profil["id"].as_str().unwrap_or_default().to_string(),
        jeton: jeton_mc.clone(),
        xuid: xuid.clone(),
    };

    enregistrer(Cache {
        refresh_token: jetons.refresh_token,
        mc_token: jeton_mc,
        mc_expire: expire,
        pseudo: session.pseudo.clone(),
        uuid: session.uuid.clone(),
        xuid,
    });
    Ok(session)
}

/// UUID hors-ligne, calculé comme Minecraft : UUID v3 de "OfflinePlayer:<pseudo>".
pub fn uuid_hors_ligne(pseudo: &str) -> String {
    let mut octets = md5::compute(format!("OfflinePlayer:{pseudo}")).0;
    octets[6] = (octets[6] & 0x0f) | 0x30;
    octets[8] = (octets[8] & 0x3f) | 0x80;
    hex::encode(octets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::coffre::CoffreMemoire;

    /// msa.json écrit par les versions qui gardaient les jetons dans le fichier
    const ANCIEN_MSA: &str = r#"{
  "actif": "uuid-b",
  "comptes": [
    { "refresh_token": "M.R-a", "mc_token": "eyJ-a", "mc_expire": 1700000000, "pseudo": "Alice", "uuid": "uuid-a", "xuid": "1" },
    { "refresh_token": "M.R-b", "mc_token": "eyJ-b", "mc_expire": 1700000001, "pseudo": "Bob", "uuid": "uuid-b", "xuid": "2" }
  ]
}"#;

    #[test]
    fn migration_deplace_les_jetons_dans_le_coffre() {
        let coffre = CoffreMemoire::default();
        let (f, nouveau) = lire_et_migrer(ANCIEN_MSA, &coffre);
        let nouveau = nouveau.expect("le fichier doit être réécrit");
        // plus aucun jeton dans le fichier…
        assert!(!nouveau.contains("M.R-") && !nouveau.contains("eyJ-"));
        assert!(!nouveau.contains("refresh_token") && !nouveau.contains("mc_token"));
        // … mais les comptes et le compte actif sont gardés
        let relu = analyser(&nouveau);
        assert_eq!(relu, f);
        assert_eq!(relu.actif, "uuid-b");
        let pseudos: Vec<_> = relu.comptes.iter().map(|c| c.pseudo.as_str()).collect();
        assert_eq!(pseudos, ["Alice", "Bob"]);
        assert_eq!(relu.comptes[1].mc_expire, 1700000001);
        // les jetons sont dans le coffre, un jeu par compte
        let b = avec_jetons(relu.comptes[1].clone(), &coffre);
        assert_eq!(
            (b.refresh_token.as_str(), b.mc_token.as_str()),
            ("M.R-b", "eyJ-b")
        );
        assert_eq!(coffre.lire("uuid-a").unwrap().refresh_token, "M.R-a");
        // deuxième lecture : rien à migrer
        assert!(lire_et_migrer(&nouveau, &coffre).1.is_none());
    }

    #[test]
    fn migration_du_tres_ancien_format_a_un_compte() {
        let coffre = CoffreMemoire::default();
        let ancien = r#"{"refresh_token":"M.R","mc_token":"eyJ","mc_expire":5,"pseudo":"Solo","uuid":"u","xuid":"x"}"#;
        let (f, nouveau) = lire_et_migrer(ancien, &coffre);
        assert!(!nouveau.unwrap().contains("M.R"));
        assert_eq!(f.actif, "u");
        assert_eq!(coffre.lire("u").unwrap().mc_token, "eyJ");
    }

    #[test]
    fn coffre_indisponible_ne_perd_aucun_jeton() {
        let coffre = CoffreMemoire {
            en_panne: true,
            ..Default::default()
        };
        let (f, nouveau) = lire_et_migrer(ANCIEN_MSA, &coffre);
        assert!(nouveau.is_none(), "le fichier ne doit pas être réécrit");
        assert_eq!(f.comptes[0].refresh_token, "M.R-a");
    }
}
