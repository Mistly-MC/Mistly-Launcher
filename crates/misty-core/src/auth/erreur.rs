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
//! Erreurs de la connexion Microsoft.

use crate::systeme::loc;

/// Une connexion Microsoft qui a échoué. Le texte (`Display`) est celui montré au joueur ;
/// `cle()` donne la clé de traduction du message (ou du titre, pour les erreurs techniques).
#[derive(Debug, thiserror::Error)]
pub enum AuthErreur {
    /// Connexion annulée, ou code de connexion expiré
    #[error("{}", loc::t("Compte_Annule"))]
    Annule,
    /// Refus de login.live.com (réponse brute)
    #[error("Microsoft : {0}")]
    Microsoft(String),
    #[error("Xbox Live : authentification refusée.")]
    XboxLive,
    #[error("{}", loc::t("Compte_PasDeXbox"))]
    PasDeXbox,
    #[error("{}", loc::t("Compte_PaysInterdit"))]
    PaysInterdit,
    #[error("{}", loc::t("Compte_VerifAge"))]
    VerifAge,
    #[error("{}", loc::t("Compte_Enfant"))]
    Enfant,
    /// Autre refus de XSTS (réponse brute)
    #[error("XSTS : {0}")]
    Xsts(String),
    /// Refus de l'API Minecraft (réponse brute)
    #[error("Minecraft : {0}")]
    Minecraft(String),
    /// Le compte ne possède pas Minecraft Java Edition
    #[error("{}", loc::t("Compte_PasDeJeu"))]
    PasDeJeu,
    #[error(transparent)]
    Reseau(#[from] reqwest::Error),
    #[error(transparent)]
    Fichier(#[from] std::io::Error),
}

impl AuthErreur {
    /// Clé de traduction : celle du message pour les erreurs connues, sinon le titre
    /// « Connexion impossible ».
    pub fn cle(&self) -> &'static str {
        match self {
            AuthErreur::Annule => "Compte_Annule",
            AuthErreur::PasDeXbox => "Compte_PasDeXbox",
            AuthErreur::PaysInterdit => "Compte_PaysInterdit",
            AuthErreur::VerifAge => "Compte_VerifAge",
            AuthErreur::Enfant => "Compte_Enfant",
            AuthErreur::PasDeJeu => "Compte_PasDeJeu",
            _ => "Compte_ConnexionImpossible",
        }
    }

    /// Erreur XSTS d'après son code XErr.
    pub fn depuis_xsts(reponse: &serde_json::Value) -> Self {
        match reponse["XErr"].as_i64() {
            Some(2148916233) => AuthErreur::PasDeXbox,
            Some(2148916235) => AuthErreur::PaysInterdit,
            Some(2148916236) | Some(2148916237) => AuthErreur::VerifAge,
            Some(2148916238) => AuthErreur::Enfant,
            _ => AuthErreur::Xsts(reponse.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_xsts() {
        let e = |code: i64| AuthErreur::depuis_xsts(&serde_json::json!({ "XErr": code }));
        assert_eq!(e(2148916233).cle(), "Compte_PasDeXbox");
        assert_eq!(e(2148916235).cle(), "Compte_PaysInterdit");
        assert_eq!(e(2148916236).cle(), "Compte_VerifAge");
        assert_eq!(e(2148916237).cle(), "Compte_VerifAge");
        assert_eq!(e(2148916238).cle(), "Compte_Enfant");
        let autre = e(1);
        assert_eq!(autre.cle(), "Compte_ConnexionImpossible");
        assert_eq!(autre.to_string(), r#"XSTS : {"XErr":1}"#);
        // le texte des erreurs connues est le message traduit
        assert_eq!(AuthErreur::PasDeJeu.to_string(), loc::t("Compte_PasDeJeu"));
    }
}
