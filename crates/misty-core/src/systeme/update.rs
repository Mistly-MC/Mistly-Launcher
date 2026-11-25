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
//! Mise à jour automatique via les releases GitHub, avec le site en secours.
//!
//! Fonctionnement : le launcher est installé dans un dossier "Misty-{version}". La mise à jour
//! télécharge le zip de la dernière release à côté, l'extrait (=> "Misty-{nouvelle}"), note l'ancien
//! dossier dans les réglages (`ancien_dossier`) puis lance le nouvel exe. Au démarrage suivant, le nouvel exe
//! supprime l'ancien dossier et met à jour le raccourci.
//!
//! Vérification : avant l'extraction, le zip doit correspondre à `Misty-x.y.z.zip.sha256` et être
//! signé (`Misty-x.y.z.zip.minisig`) par la clé `app_info::CLE_PUBLIQUE_MAJ`. Sinon il est supprimé
//! et rien n'est installé. Ces deux fichiers sont publiés par `.github/workflows/release.yml`.
//!
//! Secours : le site garde une copie de la dernière version. Si l'API GitHub ne répond pas (quota,
//! panne), la version vient de `/api/latest` ; si le téléchargement depuis GitHub échoue, le zip
//! vient de `/download/portable`.

use crate::systeme::MajErreur;
use crate::{
    systeme::app_info, systeme::http, systeme::journal, systeme::paths, systeme::reglages,
    systeme::shortcut,
};
use std::fs;
use std::path::{Path, PathBuf};

/// Site de Misty : copie de la dernière version (voir Misty-Web, `miroir`).
const SITE: &str = "https://mistly.cc";

pub struct ReleaseInfo {
    pub version: String,
    /// Pré-release (bêta) de GitHub
    pub prerelease: bool,
    /// Adresses du zip, essayées dans l'ordre (GitHub puis le site)
    pub sources: Vec<String>,
    /// Taille du zip annoncée par GitHub ou le site (0 si inconnue)
    pub taille: u64,
}

/// Étape de l'installation, pour l'affichage.
pub enum Etape {
    /// octets reçus, taille totale (0 si inconnue), téléchargement depuis le site
    Telechargement {
        recu: u64,
        total: u64,
        site: bool,
    },
    Extraction,
    Lancement,
}

/// Archive d'une version pour ce système, telle que publiée dans la release GitHub (dossier
/// « Misty-x.y.z » à la racine) : Misty-x.y.z.zip sous Windows, Misty-x.y.z-linux-x86_64.tar.gz
/// sous Linux. Une seule archive en .zip par release : les launchers d'avant 0.4 prennent le
/// premier .zip qu'ils trouvent.
pub fn nom_archive(version: &str) -> String {
    if cfg!(windows) {
        format!("Misty-{version}.zip")
    } else {
        format!("Misty-{version}-linux-x86_64.tar.gz")
    }
}

fn url_site() -> String {
    format!("{SITE}/download/portable")
}

/// Renvoie la dernière release si elle est plus récente que la version actuelle, sinon None.
/// GitHub d'abord, le site si GitHub ne répond pas.
pub async fn verifier() -> Result<Option<ReleaseInfo>, MajErreur> {
    match verifier_github().await {
        Ok(r) => Ok(r),
        // le site ne propose que la version Windows
        Err(e_github) if !cfg!(windows) => Err(e_github),
        Err(e_github) => match verifier_site().await {
            Ok(r) => Ok(r),
            // les deux sont injoignables : l'erreur de GitHub est la plus parlante
            Err(_) => Err(e_github),
        },
    }
}

async fn verifier_site() -> Result<Option<ReleaseInfo>, MajErreur> {
    let doc: serde_json::Value = http::get_json(&format!("{SITE}/api/latest")).await?;
    let version = doc["version"]
        .as_str()
        .unwrap_or("")
        .trim_start_matches('v')
        .to_string();
    if version.is_empty() {
        return Err(MajErreur::SiteSansVersion(SITE));
    }
    let taille = doc["portableSize"].as_u64().unwrap_or(0);
    Ok(
        est_plus_recente(&version, app_info::VERSION).then(|| ReleaseInfo {
            version,
            prerelease: false,
            sources: vec![url_site()],
            taille,
        }),
    )
}

async fn verifier_github() -> Result<Option<ReleaseInfo>, MajErreur> {
    // GitHub refuse les requêtes sans User-Agent (le client partagé en a un)
    let doc: serde_json::Value = if reglages::lire().maj_prerelease {
        // canal pré-release : la plus haute version parmi les releases publiées, bêtas comprises
        let liste: serde_json::Value = http::get_json(&format!(
            "https://api.github.com/repos/{}/releases?per_page=30",
            app_info::GITHUB_REPO
        ))
        .await?;
        let mut candidates: Vec<serde_json::Value> = liste
            .as_array()
            .into_iter()
            .flatten()
            .filter(|r| {
                !r["draft"].as_bool().unwrap_or(false)
                    && r["tag_name"].as_str().is_some_and(|t| !t.is_empty())
            })
            .cloned()
            .collect();
        let tag = |r: &serde_json::Value| {
            r["tag_name"]
                .as_str()
                .unwrap_or("")
                .trim_start_matches('v')
                .to_string()
        };
        candidates.sort_by(|a, b| comparer_versions(&tag(a), &tag(b)));
        match candidates.pop() {
            Some(r) => r,
            None => return Ok(None),
        }
    } else {
        http::get_json(&format!(
            "https://api.github.com/repos/{}/releases/latest",
            app_info::GITHUB_REPO
        ))
        .await?
    };
    let prerelease = doc["prerelease"].as_bool().unwrap_or(false);

    let version = doc["tag_name"]
        .as_str()
        .unwrap_or("")
        .trim_start_matches('v')
        .to_string();
    // L'archive de ce système (la release contient aussi l'installateur, l'archive de l'autre
    // système et les preuves .sha256 / .minisig)
    let attendu = nom_archive(&version);
    let zip = doc["assets"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|a| a["name"].as_str() == Some(attendu.as_str()));
    let url = zip
        .and_then(|a| a["browser_download_url"].as_str())
        .map(str::to_string);
    let taille = zip.and_then(|a| a["size"].as_u64()).unwrap_or(0);

    if version.is_empty() {
        return Err(MajErreur::GitHubSansVersion);
    }
    if !est_plus_recente(&version, app_info::VERSION) {
        return Ok(None);
    }
    // sans zip sur GitHub (pas encore envoyé), le site peut quand même l'avoir
    // (le site ne garde que la dernière version stable pour Windows : pas de secours pour une
    // pré-release ni hors Windows)
    let sources = if prerelease || !cfg!(windows) {
        url.into_iter().collect()
    } else {
        url.into_iter().chain([url_site()]).collect()
    };
    Ok(Some(ReleaseInfo {
        version,
        prerelease,
        sources,
        taille,
    }))
}

/// Vrai si `distante` est plus récente que `locale` ("0.2.3.9.5.1" > "0.2.3.9.5",
/// "0.3.7" > "0.3.7-beta.2" > "0.3.7-beta.1" > "0.3.6").
pub fn est_plus_recente(distante: &str, locale: &str) -> bool {
    comparer_versions(distante, locale) == std::cmp::Ordering::Greater
}

/// Ordre des versions : les nombres d'abord ("0.3.7"), puis le suffixe de pré-release ("-beta.1",
/// "b2", "-rc1"…) : une version sans suffixe passe après ses pré-releases.
pub fn comparer_versions(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    // "0.3.7-beta.1" -> ([0, 3, 7], "beta.1")
    let decouper = |v: &str| -> (Vec<u64>, String) {
        let v = v.trim().trim_start_matches(['v', 'V']);
        let fin = v
            .find(|c: char| !(c.is_ascii_digit() || c == '.'))
            .unwrap_or(v.len());
        let nombres = v[..fin]
            .split('.')
            .filter(|s| !s.is_empty())
            .map(|s| s.parse().unwrap_or(0))
            .collect();
        (
            nombres,
            v[fin..]
                .trim_start_matches(['-', '.', '_', '+'])
                .to_lowercase(),
        )
    };
    // "beta.10" -> ["beta", 10] : les nombres se comparent comme des nombres
    let morceaux = |s: &str| -> Vec<Result<u64, String>> {
        let mut liste = Vec::new();
        let mut courant = String::new();
        for c in s.chars() {
            let change = !courant.is_empty()
                && courant.chars().last().is_some_and(|d| d.is_ascii_digit()) != c.is_ascii_digit();
            if (!c.is_ascii_alphanumeric() || change) && !courant.is_empty() {
                liste.push(courant.parse::<u64>().map_err(|_| courant.clone()));
                courant.clear();
            }
            if c.is_ascii_alphanumeric() {
                courant.push(c);
            }
        }
        if !courant.is_empty() {
            liste.push(courant.parse::<u64>().map_err(|_| courant.clone()));
        }
        liste
    };
    let ((na, sa), (nb, sb)) = (decouper(a), decouper(b));
    for i in 0..na.len().max(nb.len()) {
        let (x, y) = (
            na.get(i).copied().unwrap_or(0),
            nb.get(i).copied().unwrap_or(0),
        );
        if x != y {
            return x.cmp(&y);
        }
    }
    match (sa.is_empty(), sb.is_empty()) {
        (true, true) => return Ordering::Equal,
        (true, false) => return Ordering::Greater,
        (false, true) => return Ordering::Less,
        _ => {}
    }
    let (ma, mb) = (morceaux(&sa), morceaux(&sb));
    for i in 0..ma.len().max(mb.len()) {
        let ordre = match (ma.get(i), mb.get(i)) {
            (Some(Ok(x)), Some(Ok(y))) => x.cmp(y),
            (Some(Err(x)), Some(Err(y))) => x.cmp(y),
            // un nombre passe avant un mot ("1" < "beta")
            (Some(Ok(_)), Some(Err(_))) => Ordering::Less,
            (Some(Err(_)), Some(Ok(_))) => Ordering::Greater,
            (Some(_), None) => Ordering::Greater,
            (None, Some(_)) => Ordering::Less,
            (None, None) => Ordering::Equal,
        };
        if ordre != Ordering::Equal {
            return ordre;
        }
    }
    Ordering::Equal
}

/// Dossier qui contient les dossiers "Misty-{version}".
fn racine() -> PathBuf {
    let dossier_actuel = format!("Misty-{}", app_info::VERSION);
    let app = paths::app_dir().to_string_lossy().to_string();
    match app.find(&dossier_actuel) {
        Some(i) => PathBuf::from(&app[..i]),
        None => PathBuf::from(app),
    }
}

/// Télécharge le zip depuis une source, en signalant l'avancement.
async fn telecharger(
    url: &str,
    version: &str,
    taille: u64,
    zip: &Path,
    avancement: &(dyn Fn(Etape) + Send + Sync),
) -> Result<(), MajErreur> {
    use futures::StreamExt;
    use tokio::io::AsyncWriteExt;
    let site = url.starts_with(SITE);
    let reponse = http::client()
        .get(url)
        // le délai du client partagé (5 min) est trop court pour une connexion lente
        .timeout(std::time::Duration::from_secs(3600))
        // un zip ne gagne rien à être recompressé, et la taille reste annoncée
        .header(reqwest::header::ACCEPT_ENCODING, "identity")
        .send()
        .await?
        .error_for_status()?;
    // le site sert toujours sa dernière version : elle doit être celle qu'on attend
    if site {
        let attendu = format!("Misty-{version}.zip");
        let nom = reponse
            .headers()
            .get(reqwest::header::CONTENT_DISPOSITION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !nom.contains(&attendu) {
            return Err(MajErreur::AutreFichier(attendu));
        }
    }
    let total = reponse
        .content_length()
        .filter(|&n| n > 0)
        .unwrap_or(taille);
    let tmp = zip.with_extension("zip.part");
    let mut recu = 0u64;
    avancement(Etape::Telechargement { recu, total, site });
    let resultat = async {
        let mut sortie = tokio::fs::File::create(&tmp).await?;
        let mut flux = reponse.bytes_stream();
        while let Some(morceau) = flux.next().await {
            let morceau = morceau?;
            sortie.write_all(&morceau).await?;
            recu += morceau.len() as u64;
            avancement(Etape::Telechargement { recu, total, site });
        }
        sortie.flush().await?;
        if total != 0 && recu != total {
            return Err(MajErreur::Incomplet { recu, total });
        }
        Ok::<(), MajErreur>(())
    }
    .await;
    if let Err(e) = resultat {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    let _ = fs::remove_file(zip);
    fs::rename(&tmp, zip)?;
    Ok(())
}

/// Vérifie un zip de mise à jour avant de l'extraire : son empreinte SHA-256 (`Misty-x.y.z.zip.sha256`,
/// au format de `sha256sum`), puis sa signature minisign (`Misty-x.y.z.zip.minisig`) avec
/// `cle_publique`. L'erreur est le message traduit à montrer au joueur.
pub fn verifier_archive(
    octets: &[u8],
    sha256: &str,
    signature: &str,
    cle_publique: &str,
) -> Result<(), MajErreur> {
    use sha2::{Digest, Sha256};
    let attendu = sha256
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    if attendu != hex::encode(Sha256::digest(octets)) {
        return Err(MajErreur::Empreinte);
    }
    let signature_valide =
        minisign_verify::PublicKey::from_base64(cle_publique.trim()).and_then(|cle| {
            let sig = minisign_verify::Signature::decode(signature)?;
            // pas d'ancien format de signature (sans pré-hachage) : release.yml n'en produit pas
            cle.verify(octets, &sig, false)
        });
    if let Err(e) = signature_valide {
        journal::info(&format!("Signature de la mise à jour refusée : {e}"));
        return Err(MajErreur::Signature);
    }
    Ok(())
}

/// `Misty-x.y.z.zip.sha256` et `.minisig`, toujours pris sur la release GitHub (même quand le zip
/// vient du site) : une copie du zip ne peut pas fournir ses propres preuves.
async fn telecharger_preuves(version: &str) -> Result<(String, String), MajErreur> {
    let base = format!(
        "https://github.com/{}/releases/download/{version}/{}",
        app_info::GITHUB_REPO,
        nom_archive(version)
    );
    let (url_sha256, url_signature) = (format!("{base}.sha256"), format!("{base}.minisig"));
    let (sha256, signature) =
        futures::try_join!(http::get_text(&url_sha256), http::get_text(&url_signature)).map_err(
            |e| {
                journal::info(&format!("Preuves de la mise à jour introuvables : {e:#}"));
                MajErreur::Preuves
            },
        )?;
    Ok((sha256, signature))
}

/// Vérifie le zip téléchargé ; s'il est refusé, il est supprimé.
async fn verifier_zip(zip: &Path, version: &str) -> Result<(), MajErreur> {
    let resultat = async {
        let (sha256, signature) = telecharger_preuves(version).await?;
        let zip = zip.to_path_buf();
        tokio::task::spawn_blocking(move || {
            verifier_archive(
                &fs::read(&zip)?,
                &sha256,
                &signature,
                app_info::CLE_PUBLIQUE_MAJ,
            )
        })
        .await?
    }
    .await;
    if resultat.is_err() {
        let _ = fs::remove_file(zip);
    }
    resultat
}

/// Télécharge, extrait et lance la nouvelle version. L'appelant doit ensuite quitter l'application.
pub async fn installer(
    release: &ReleaseInfo,
    avancement: impl Fn(Etape) + Send + Sync,
) -> Result<(), MajErreur> {
    let racine = racine();
    let zip = racine.join(nom_archive(&release.version));
    let nouveau_dossier = racine.join(format!("Misty-{}", release.version));

    // chaque source à son tour : GitHub, puis la copie du site. Un zip refusé à la vérification
    // est supprimé et la source suivante est essayée ; rien n'est extrait sans vérification.
    let mut erreur = None;
    for url in &release.sources {
        match telecharger(url, &release.version, release.taille, &zip, &avancement).await {
            Ok(()) => match verifier_zip(&zip, &release.version).await {
                Ok(()) => {
                    erreur = None;
                    break;
                }
                Err(e) => erreur = Some(e),
            },
            Err(e) => {
                erreur = Some(MajErreur::Source {
                    url: url.clone(),
                    erreur: Box::new(e),
                })
            }
        }
    }
    if let Some(e) = erreur {
        return Err(e);
    }
    avancement(Etape::Extraction);
    {
        let zip = zip.clone();
        let racine = racine.clone();
        tokio::task::spawn_blocking(move || -> Result<(), MajErreur> {
            crate::systeme::archive::extraire(&zip, &racine)?;
            Ok(())
        })
        .await??;
    }
    let _ = fs::remove_file(&zip);
    if !nouveau_dossier.join(paths::NOM_EXE).exists() {
        return Err(MajErreur::ExeAbsent(release.version.clone()));
    }

    reglages::modifier(|r| {
        r.ancien_dossier = racine
            .join(format!("Misty-{}", app_info::VERSION))
            .to_string_lossy()
            .to_string()
    });

    if paths::custom_icon().exists() {
        let _ = fs::copy(paths::custom_icon(), nouveau_dossier.join("Icon.ico"));
    }

    avancement(Etape::Lancement);
    std::process::Command::new(nouveau_dossier.join(paths::NOM_EXE))
        .current_dir(&nouveau_dossier)
        .spawn()?;
    Ok(())
}

/// Après une mise à jour : supprime l'ancien dossier et remet le raccourci sur le nouvel exe.
/// Renvoie true si une mise à jour vient d'être finalisée.
pub async fn finaliser() -> Result<bool, MajErreur> {
    let ancien = reglages::lire().ancien_dossier;
    if ancien.is_empty() {
        return Ok(false);
    }

    tokio::time::sleep(std::time::Duration::from_secs(1)).await; // laisse le temps à l'ancien exe de se fermer

    // le lanceur fixe démarre désormais cette version ; les raccourcis et les épingles sont
    // redirigés vers lui juste après (ui::demarrer)
    crate::systeme::lanceur::installer(true);

    let raccourci = reglages::lire().raccourci;
    if !raccourci.is_empty() {
        let _ = shortcut::creer(Path::new(&raccourci));
    }

    // Installé avec le setup : l'entrée "Applications installées" pointe encore vers l'ancien dossier
    maj_entree_desinstallation();

    if Path::new(&ancien).exists() {
        fs::remove_dir_all(&ancien)?;
    }
    reglages::modifier(|r| r.ancien_dossier = String::new());
    Ok(true)
}

/// ID de l'installateur Inno Setup (installer\Misty.iss).
#[cfg(windows)]
const CLE_DESINSTALLATION: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{8F4C2A61-3B7E-4D19-9A55-6C1E0F2B7D43}_is1";

/// Met à jour la version et l'icône affichées dans "Applications installées".
#[cfg(windows)]
fn maj_entree_desinstallation() {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_SET_VALUE};
    let Ok(cle) = winreg::RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(CLE_DESINSTALLATION, KEY_SET_VALUE)
    else {
        return;
    };
    let _ = cle.set_value("DisplayVersion", &app_info::VERSION);
    let _ = cle.set_value(
        "DisplayIcon",
        &paths::app_exe().to_string_lossy().to_string(),
    );
}

#[cfg(not(windows))]
fn maj_entree_desinstallation() {}

/// Migre l'ancien dossier de données C:\TEXT vers %LOCALAPPDATA%\Misty.
/// Renvoie true si une migration a eu lieu.
pub fn migrer_ancien_dossier() -> Result<bool, MajErreur> {
    let ancien = Path::new(paths::LEGACY_DATA_DIR);
    if !ancien.exists() {
        return Ok(false);
    }
    if !paths::data_dir().exists() {
        copier_dossier(ancien, &paths::data_dir())?;
    }
    fs::remove_dir_all(ancien)?;
    Ok(true)
}

fn copier_dossier(source: &Path, destination: &Path) -> Result<(), MajErreur> {
    fs::create_dir_all(destination)?;
    for entree in fs::read_dir(source)?.flatten() {
        let cible = destination.join(entree.file_name());
        if entree.path().is_dir() {
            copier_dossier(&entree.path(), &cible)?;
        } else {
            fs::copy(entree.path(), cible)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systeme::loc;

    #[test]
    fn ordre_des_versions() {
        assert!(est_plus_recente("0.3.7", "0.3.6"));
        assert!(est_plus_recente("0.2.3.9.5.1", "0.2.3.9.5"));
        assert!(!est_plus_recente("0.3.6", "0.3.6"));
        assert!(est_plus_recente("0.3.7-beta.1", "0.3.6"));
        assert!(est_plus_recente("0.3.7", "0.3.7-beta.2"));
        assert!(est_plus_recente("0.3.7-beta.2", "0.3.7-beta.1"));
        assert!(est_plus_recente("0.3.7-beta.10", "0.3.7-beta.9"));
        assert!(est_plus_recente("0.3.7-rc1", "0.3.7-beta3"));
        assert!(!est_plus_recente("0.3.7-beta.1", "0.3.7"));
        assert!(est_plus_recente("v0.3.8", "0.3.7"));
    }

    /// Une archive signée comme le fait release.yml (clé jetable) : (zip, .sha256, .minisig, clé publique).
    fn archive_signee(octets: &[u8]) -> (Vec<u8>, String, String, String) {
        use sha2::{Digest, Sha256};
        let paire = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
        let signature = minisign::sign(Some(&paire.pk), &paire.sk, octets, None, None)
            .unwrap()
            .into_string();
        let sha256 = format!("{}  Misty-9.9.9.zip\n", hex::encode(Sha256::digest(octets)));
        (octets.to_vec(), sha256, signature, paire.pk.to_base64())
    }

    #[test]
    fn cle_publique_lisible() {
        // ID de la clé : 7F6148B2774655BD (misty.pub)
        minisign_verify::PublicKey::from_base64(app_info::CLE_PUBLIQUE_MAJ).unwrap();
    }

    #[test]
    fn archive_verifiee_acceptee() {
        let (zip, sha256, signature, cle) = archive_signee(b"PK une archive");
        verifier_archive(&zip, &sha256, &signature, &cle).unwrap();
        // empreinte en majuscules, sans nom de fichier : acceptée aussi
        verifier_archive(&zip, &sha256[..64].to_uppercase(), &signature, &cle).unwrap();
    }

    #[test]
    fn mauvaise_empreinte_refusee() {
        let (zip, _, signature, cle) = archive_signee(b"PK une archive");
        let faux = format!("{}  Misty-9.9.9.zip", "0".repeat(64));
        let e = verifier_archive(&zip, &faux, &signature, &cle).unwrap_err();
        assert_eq!(e.to_string(), loc::t("Maj_Verif_Empreinte"));
        let e = verifier_archive(&zip, "", &signature, &cle).unwrap_err();
        assert_eq!(e.to_string(), loc::t("Maj_Verif_Empreinte"));
    }

    #[test]
    fn signature_invalide_refusee() {
        let (zip, sha256, signature, cle) = archive_signee(b"PK une archive");
        let refus = loc::t("Maj_Verif_Signature");
        // signature d'une autre clé (zip signé par quelqu'un d'autre que l'équipe)
        let (_, _, _, autre_cle) = archive_signee(b"PK une archive");
        let e = verifier_archive(&zip, &sha256, &signature, &autre_cle).unwrap_err();
        assert_eq!(e.to_string(), refus);
        // signature d'un autre fichier, avec une empreinte recalculée pour le zip modifié
        let (modifie, sha_modifie, _, _) = archive_signee(b"PK une archive piegee");
        let e = verifier_archive(&modifie, &sha_modifie, &signature, &cle).unwrap_err();
        assert_eq!(e.to_string(), refus);
        // signature illisible, ou clé absente (CLE_PUBLIQUE_MAJ vide)
        let e = verifier_archive(&zip, &sha256, "pas une signature", &cle).unwrap_err();
        assert_eq!(e.to_string(), refus);
        let e = verifier_archive(&zip, &sha256, &signature, "").unwrap_err();
        assert_eq!(e.to_string(), refus);
    }

    /// Secours par le site (réseau) : `cargo test secours_site -- --ignored`.
    #[tokio::test]
    #[ignore]
    async fn secours_site() {
        let doc: serde_json::Value = http::get_json(&format!("{SITE}/api/latest")).await.unwrap();
        let version = doc["version"]
            .as_str()
            .unwrap()
            .trim_start_matches('v')
            .to_string();
        let zip = std::env::temp_dir().join(format!("misty-test-{version}.zip"));
        let max = std::sync::Mutex::new((0u64, 0u64, 0usize));
        let taille = doc["portableSize"].as_u64().unwrap();
        telecharger(&url_site(), &version, taille, &zip, &|e| {
            if let Etape::Telechargement { recu, total, site } = e {
                assert!(site);
                let mut m = max.lock().unwrap();
                *m = (recu, total, m.2 + 1);
            }
        })
        .await
        .unwrap();
        let (recu, total, appels) = *max.lock().unwrap();
        assert_eq!(recu, total);
        assert_eq!(fs::metadata(&zip).unwrap().len(), total);
        zip::ZipArchive::new(fs::File::open(&zip).unwrap()).unwrap();
        let _ = fs::remove_file(&zip);
        // une version qui n'est pas celle du site est refusée
        assert!(telecharger(&url_site(), "0.0.1", 0, &zip, &|_| {})
            .await
            .is_err());
        println!("{version} : {total} octets, {appels} étapes");
    }
}
