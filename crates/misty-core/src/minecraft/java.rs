//! Java fourni par Mojang (le même que le launcher officiel), téléchargé à la demande.
//! Chaque version de Minecraft indique le composant voulu ("jre-legacy" = Java 8, "java-runtime-delta" = Java 21…).

use super::install::{telecharger_tout, Tache};
use super::{FileProgress, McDirs};
use crate::systeme::http;
use serde_json::Value;
use std::path::PathBuf;

const RUNTIMES_URL: &str =
    "https://piston-meta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

fn plateforme() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "windows-arm64"
    } else if cfg!(target_arch = "x86") {
        "windows-x86"
    } else {
        "windows-x64"
    }
}

/// Chemin de javaw.exe pour ce composant (téléchargé si besoin).
pub async fn assurer(
    dirs: &McDirs,
    composant: &str,
    majeure: u32,
    progression: &FileProgress,
) -> anyhow::Result<PathBuf> {
    // Même organisation que la version C# : runtime\windows-x64\<composant>
    let dossier = dirs.runtime().join(plateforme()).join(composant);
    let javaw = dossier.join("bin").join("javaw.exe");
    let marqueur = dossier.join(".misty-version");

    let tous: Value = match http::get_json(RUNTIMES_URL).await {
        Ok(v) => v,
        // Hors-ligne : on garde le Java déjà installé
        Err(e) if javaw.exists() => {
            let _ = e;
            return Ok(javaw);
        }
        Err(e) => return Err(e),
    };

    // Composant indisponible pour cette plateforme : on prend un Java de version proche
    let mut entree = tous[plateforme()][composant]
        .as_array()
        .and_then(|a| a.first())
        .cloned();
    if entree.is_none() {
        let secours = match majeure {
            0..=8 => "jre-legacy",
            9..=16 => "java-runtime-alpha",
            17 => "java-runtime-gamma",
            _ => "java-runtime-delta",
        };
        entree = tous[plateforme()][secours]
            .as_array()
            .and_then(|a| a.first())
            .cloned();
    }
    let Some(entree) = entree else {
        anyhow::bail!(crate::minecraft::InstallErreur::JavaIndisponible {
            majeure,
            plateforme: plateforme(),
        })
    };
    let url_manifeste = entree["manifest"]["url"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let sha1_manifeste = entree["manifest"]["sha1"]
        .as_str()
        .unwrap_or_default()
        .to_string();

    // Déjà installé et à jour. Le marqueur dit que chaque fichier a été vérifié (SHA-1) : un ancien
    // marqueur (le SHA-1 du manifeste seul, avant 0.3.9) fait revérifier le runtime, car une mise à
    // jour de Mojang ne remplaçait alors que les fichiers qui avaient changé de taille
    let attendu = marqueur_verifie(&sha1_manifeste);
    if javaw.exists()
        && std::fs::read_to_string(&marqueur).ok().as_deref() == Some(attendu.as_str())
    {
        return Ok(javaw);
    }

    let manifeste: Value = http::get_json(&url_manifeste).await?;
    let mut taches = Vec::new();
    for (chemin, fichier) in manifeste["files"].as_object().into_iter().flatten() {
        let destination = dossier.join(chemin.replace('/', "\\"));
        match fichier["type"].as_str() {
            Some("directory") => {
                std::fs::create_dir_all(&destination)?;
            }
            Some("file") => {
                let raw = &fichier["downloads"]["raw"];
                if let Some(url) = raw["url"].as_str() {
                    taches.push(Tache {
                        chemin: destination,
                        url: url.to_string(),
                        sha1: raw["sha1"].as_str().map(str::to_string),
                        taille: raw["size"].as_u64(),
                    });
                }
            }
            _ => {} // liens symboliques : inutiles sous Windows
        }
    }
    // Mise à jour du runtime : un fichier déjà là mais dont le contenu a changé (même taille) est
    // supprimé pour être retéléchargé. Sinon, l'ancien management.dll restait à côté du nouveau
    // jvm.dll, et Java ne pouvait plus le charger.
    let (taches, tout_remplace) = tokio::task::spawn_blocking(move || {
        let mut tout_remplace = true;
        for t in &taches {
            let Some(sha1) = &t.sha1 else { continue };
            if t.chemin.exists()
                && !crate::systeme::download::sha1_fichier(&t.chemin)
                    .is_ok_and(|h| h.eq_ignore_ascii_case(sha1))
            {
                // un fichier utilisé par une partie en cours ne peut pas être remplacé : on revérifiera
                tout_remplace &= std::fs::remove_file(&t.chemin).is_ok();
            }
        }
        (taches, tout_remplace)
    })
    .await?;
    telecharger_tout(taches, progression).await?;
    if tout_remplace {
        std::fs::write(&marqueur, &attendu)?;
    }

    if !javaw.exists() {
        anyhow::bail!(crate::minecraft::InstallErreur::JavaIncomplet(
            composant.to_string()
        ));
    }
    Ok(javaw)
}

/// Contenu du marqueur d'un runtime vérifié fichier par fichier, pour ce manifeste.
fn marqueur_verifie(sha1_manifeste: &str) -> String {
    format!("{sha1_manifeste} sha1")
}

/// java.exe à côté de javaw.exe (pour les outils en ligne de commande comme les installateurs).
pub fn java_console(javaw: &std::path::Path) -> PathBuf {
    let java = javaw.with_file_name("java.exe");
    if java.exists() {
        java
    } else {
        javaw.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// Internet et le dossier Misty : installe (ou répare) Java 25, puis vérifie chaque fichier
    /// avec le manifeste de Mojang.
    #[tokio::test]
    #[ignore]
    async fn java_25_complet() {
        let dirs = McDirs::misty();
        let progression: FileProgress = Arc::new(|_, _, _| {});
        let javaw = assurer(&dirs, "java-runtime-epsilon", 25, &progression)
            .await
            .unwrap();
        let dossier = javaw.parent().unwrap().parent().unwrap().to_path_buf();
        let tous: Value = http::get_json(RUNTIMES_URL).await.unwrap();
        let manifeste: Value = http::get_json(
            tous[plateforme()]["java-runtime-epsilon"][0]["manifest"]["url"]
                .as_str()
                .unwrap(),
        )
        .await
        .unwrap();
        let mut differents = Vec::new();
        for (chemin, fichier) in manifeste["files"].as_object().unwrap() {
            if let Some(sha1) = fichier["downloads"]["raw"]["sha1"].as_str() {
                let local = dossier.join(chemin.replace('/', "\\"));
                if !crate::systeme::download::sha1_fichier(&local)
                    .is_ok_and(|h| h.eq_ignore_ascii_case(sha1))
                {
                    differents.push(chemin.clone());
                }
            }
        }
        assert!(
            differents.is_empty(),
            "fichiers différents du manifeste : {differents:?}"
        );
    }
}
