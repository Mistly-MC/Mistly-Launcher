//! Erreurs de l'installation du jeu (versions, bibliothèques, Java, mod loaders).

use std::path::PathBuf;

/// Une installation de Minecraft qui a échoué. Ces messages sont techniques (ils aident surtout à
/// diagnostiquer, dans le journal ou un rapport) : `cle()` donne le titre traduit à afficher
/// au-dessus.
#[derive(Debug, thiserror::Error)]
pub enum InstallErreur {
    #[error("Version {0} introuvable chez Mojang")]
    VersionInconnue(String),
    /// JSON d'une version installée absent ou illisible
    #[error("Version {id} introuvable ({detail})")]
    VersionIllisible { id: String, detail: String },
    #[error("Bibliothèque manquante : {}", .0.display())]
    BibliothequeManquante(PathBuf),
    /// Un fichier du jeu impossible à télécharger (après deux essais)
    #[error("{url} : {detail}")]
    Telechargement { url: String, detail: String },
    #[error("Java {majeure} indisponible pour {plateforme}")]
    JavaIndisponible {
        majeure: u32,
        plateforme: &'static str,
    },
    #[error("Installation de Java incomplète ({0})")]
    JavaIncomplet(String),
    #[error("mainClass manquante")]
    MainClassManquante,
    /// Aucune version du mod loader pour cette version de Minecraft
    #[error("{loader} indisponible pour {version_mc}")]
    LoaderIndisponible { loader: String, version_mc: String },
    /// L'installeur d'un loader (Forge, OptiFine) a échoué : sa sortie d'erreur
    #[error("{0}")]
    InstalleurLoader(String),
    #[error("Mistly Client n'existe pas pour Minecraft {0}")]
    ClientIndisponible(String),
    #[error(transparent)]
    Reseau(#[from] reqwest::Error),
    #[error(transparent)]
    Fichier(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Archive(#[from] zip::result::ZipError),
    /// Erreur d'un utilitaire partagé (téléchargement…)
    #[error(transparent)]
    Autre(#[from] anyhow::Error),
}

impl InstallErreur {
    /// Clé de traduction du titre : « Lancement impossible ».
    pub fn cle(&self) -> &'static str {
        "Home_LancementImpossible"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn textes_inchanges() {
        assert_eq!(
            InstallErreur::LoaderIndisponible {
                loader: "NeoForge".into(),
                version_mc: "1.8.9".into()
            }
            .to_string(),
            "NeoForge indisponible pour 1.8.9"
        );
        assert_eq!(
            InstallErreur::JavaIndisponible {
                majeure: 25,
                plateforme: "windows-x64"
            }
            .to_string(),
            "Java 25 indisponible pour windows-x64"
        );
        assert_eq!(
            InstallErreur::MainClassManquante.cle(),
            "Home_LancementImpossible"
        );
    }
}
