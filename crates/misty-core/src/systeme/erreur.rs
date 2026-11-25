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
//! Erreurs de la mise à jour automatique du launcher.

use crate::systeme::loc;

/// Une mise à jour qui a échoué. Le texte (`Display`) est celui montré au joueur ; `cle()` donne
/// la clé de traduction du message (ou du titre « Mise à jour impossible » pour les erreurs
/// techniques).
#[derive(Debug, thiserror::Error)]
pub enum MajErreur {
    /// Le zip ne correspond pas à son fichier .sha256
    #[error("{}", loc::t("Maj_Verif_Empreinte"))]
    Empreinte,
    /// La signature minisign ne vient pas de la clé de l'équipe
    #[error("{}", loc::t("Maj_Verif_Signature"))]
    Signature,
    /// .sha256 ou .minisig introuvables
    #[error("{}", loc::t("Maj_Verif_Fichiers"))]
    Preuves,
    #[error("{0}/api/latest : version absente")]
    SiteSansVersion(&'static str),
    #[error("GitHub : release sans version")]
    GitHubSansVersion,
    /// Le site sert une autre version que celle annoncée
    #[error("le site propose un autre fichier que {0}")]
    AutreFichier(String),
    #[error("téléchargement incomplet ({recu} octets sur {total})")]
    Incomplet { recu: u64, total: u64 },
    #[error("Misty.exe absent de l'archive (Misty-{0})")]
    ExeAbsent(String),
    /// Échec d'une source de téléchargement (l'adresse, puis la cause)
    #[error("{url}")]
    Source {
        url: String,
        #[source]
        erreur: Box<MajErreur>,
    },
    #[error(transparent)]
    Reseau(#[from] reqwest::Error),
    #[error(transparent)]
    Fichier(#[from] std::io::Error),
    #[error(transparent)]
    Archive(#[from] zip::result::ZipError),
    #[error(transparent)]
    Tache(#[from] tokio::task::JoinError),
    /// Erreur d'un utilitaire partagé (http::get_json…)
    #[error(transparent)]
    Autre(#[from] anyhow::Error),
}

impl MajErreur {
    /// Clé de traduction : celle du message pour les vérifications, sinon le titre
    /// « Mise à jour impossible ». Pour une source qui a échoué, la clé de sa cause.
    pub fn cle(&self) -> &'static str {
        match self {
            MajErreur::Empreinte => "Maj_Verif_Empreinte",
            MajErreur::Signature => "Maj_Verif_Signature",
            MajErreur::Preuves => "Maj_Verif_Fichiers",
            MajErreur::Source { erreur, .. } => erreur.cle(),
            _ => "Maj_Impossible",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cles_et_textes() {
        assert_eq!(MajErreur::Signature.cle(), "Maj_Verif_Signature");
        assert_eq!(
            MajErreur::Signature.to_string(),
            loc::t("Maj_Verif_Signature")
        );
        let source = MajErreur::Source {
            url: "https://github.com/x.zip".into(),
            erreur: Box::new(MajErreur::Incomplet { recu: 1, total: 2 }),
        };
        assert_eq!(source.cle(), "Maj_Impossible");
        // même texte qu'avant (anyhow `{:#}` : l'adresse, puis la cause)
        assert_eq!(
            format!("{:#}", anyhow::Error::from(source)),
            "https://github.com/x.zip: téléchargement incomplet (1 octets sur 2)"
        );
    }
}
