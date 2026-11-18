//! Erreurs du contenu : mods, packs, modpacks (Modrinth, CurseForge).

use crate::systeme::loc;

/// Une installation de contenu qui a échoué. Le texte (`Display`) est celui montré au joueur ;
/// `cle()` donne la clé de traduction du message (ou du titre « Erreur inattendue » pour les
/// erreurs techniques).
#[derive(Debug, thiserror::Error)]
pub enum ContenuErreur {
    /// Aucune version compatible avec le profil (`cle` : Erreur_PasDeVersion ou
    /// Erreur_PasDeVersionCF, selon la source)
    #[error("{}", loc::tf(.cle, &[.titre, .version_mc, .loader]))]
    PasDeVersion {
        cle: &'static str,
        titre: String,
        version_mc: String,
        loader: String,
    },
    /// L'auteur interdit le téléchargement hors du site (CurseForge) : lien vers la page du fichier
    #[error("{}", loc::tf("Erreur_CFInterdit", &[.titre, .page]))]
    TelechargementInterdit { titre: String, page: String },
    #[error("{}", loc::tf("Erreur_ProjetCF", &[.0]))]
    ProjetCF(i64),
    #[error("{}", loc::t("Erreur_AucunFichier"))]
    AucunFichier,
    #[error("{}", loc::t("Erreur_CleCF"))]
    CleCF,
    #[error("{}", loc::t("Export_JamaisLance"))]
    JamaisLance,
    // ── Modpacks ──
    #[error("{}", loc::t("Erreur_ModpackIllisible"))]
    ModpackIllisible,
    #[error("{}", loc::t("Erreur_ModpackFormat"))]
    ModpackFormat,
    #[error("{}", loc::t("Erreur_PasUnModpack"))]
    PasUnModpack,
    #[error("{}", loc::t("Erreur_ModpackSansVersion"))]
    ModpackSansVersion,
    #[error("{}", loc::t("Erreur_ModpackCFInterdit"))]
    ModpackInterdit,
    #[error("{}", loc::t("Erreur_SansMrpack"))]
    SansMrpack,
    #[error("{}", loc::t("Erreur_ModpackSansMc"))]
    ModpackSansMc,
    #[error("{}", loc::tf("Erreur_LoaderNonPris", &[.0]))]
    LoaderNonPris(String),
    #[error("{}", loc::tf("Erreur_Manquant", &[.0]))]
    Manquant(String),
    /// Fichier d'un modpack impossible à télécharger (chemin, puis la dernière erreur)
    #[error("{}{}", loc::tf("Erreur_Telechargement", &[.chemin]), .detail)]
    Telechargement { chemin: String, detail: String },
    // ── Packs de textures ──
    #[error("{}", loc::tf("Pack_VersionInconnue", &[.0]))]
    PackVersionInconnue(String),
    #[error("{}", loc::tf("Pack_VersionTropAncienne", &[.0]))]
    PackVersionTropAncienne(String),
    #[error("{}", loc::t("Pack_NomInvalide"))]
    PackNomInvalide,
    #[error("{}", loc::t("Pack_ExisteDeja"))]
    PackExisteDeja,
    #[error("{}", loc::t("Pack_ImportPasUnPack"))]
    PackPasUnPack,
    #[error("{}", loc::t("Pack_DistantAucunFichier"))]
    PackAucunFichier,
    // ── Techniques ──
    #[error("Monde requis pour un datapack")]
    MondeRequis,
    #[error("Type inconnu : {0}")]
    TypeInconnu(String),
    #[error(transparent)]
    Reseau(#[from] reqwest::Error),
    #[error(transparent)]
    Fichier(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// Erreur d'un utilitaire partagé (téléchargement, source de contenu…)
    #[error(transparent)]
    Autre(#[from] anyhow::Error),
}

impl ContenuErreur {
    /// Clé de traduction du message, ou « Erreur_Inattendue » pour les erreurs techniques. Une
    /// erreur typée enveloppée dans `Autre` donne sa propre clé.
    pub fn cle(&self) -> &'static str {
        use ContenuErreur::*;
        match self {
            PasDeVersion { cle, .. } => cle,
            TelechargementInterdit { .. } => "Erreur_CFInterdit",
            ProjetCF(_) => "Erreur_ProjetCF",
            AucunFichier => "Erreur_AucunFichier",
            CleCF => "Erreur_CleCF",
            JamaisLance => "Export_JamaisLance",
            ModpackIllisible => "Erreur_ModpackIllisible",
            ModpackFormat => "Erreur_ModpackFormat",
            PasUnModpack => "Erreur_PasUnModpack",
            ModpackSansVersion => "Erreur_ModpackSansVersion",
            ModpackInterdit => "Erreur_ModpackCFInterdit",
            SansMrpack => "Erreur_SansMrpack",
            ModpackSansMc => "Erreur_ModpackSansMc",
            LoaderNonPris(_) => "Erreur_LoaderNonPris",
            Manquant(_) => "Erreur_Manquant",
            Telechargement { .. } => "Erreur_Telechargement",
            PackVersionInconnue(_) => "Pack_VersionInconnue",
            PackVersionTropAncienne(_) => "Pack_VersionTropAncienne",
            PackNomInvalide => "Pack_NomInvalide",
            PackExisteDeja => "Pack_ExisteDeja",
            PackPasUnPack => "Pack_ImportPasUnPack",
            PackAucunFichier => "Pack_DistantAucunFichier",
            Autre(e) => e
                .downcast_ref::<ContenuErreur>()
                .map(ContenuErreur::cle)
                .unwrap_or("Erreur_Inattendue"),
            MondeRequis | TypeInconnu(_) | Reseau(_) | Fichier(_) | Json(_) => "Erreur_Inattendue",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn textes_traduits_et_cles() {
        let e = ContenuErreur::PasDeVersion {
            cle: "Erreur_PasDeVersionCF",
            titre: "JEI".into(),
            version_mc: "1.21.1".into(),
            loader: "Forge".into(),
        };
        assert_eq!(e.cle(), "Erreur_PasDeVersionCF");
        assert_eq!(
            e.to_string(),
            loc::tf("Erreur_PasDeVersionCF", &[&"JEI", &"1.21.1", &"Forge"])
        );
        // une erreur typée qui a traversé anyhow garde sa clé
        let enveloppee = ContenuErreur::Autre(ContenuErreur::ProjetCF(42).into());
        assert_eq!(enveloppee.cle(), "Erreur_ProjetCF");
        assert_eq!(enveloppee.to_string(), loc::tf("Erreur_ProjetCF", &[&42]));
    }
}
