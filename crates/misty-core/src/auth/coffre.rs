//! Coffre des jetons Microsoft : le coffre du système, une entrée par compte
//! (« msa-<uuid>.Misty ») : le Gestionnaire d'identifiants sous Windows, le Secret Service
//! (GNOME Keyring, KWallet) sous Linux. msa.json ne garde que ce qui n'est pas secret (pseudo,
//! uuid, compte actif). Si le coffre refuse une entrée, les jetons restent dans msa.json.
//!
//! Sur les autres systèmes (tests, développement), les jetons vont dans un fichier à part,
//! `msa-secrets.json`.

use serde::{Deserialize, Serialize};

/// Les jetons d'un compte.
#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Debug)]
pub struct Secrets {
    #[serde(default, rename = "r", skip_serializing_if = "String::is_empty")]
    pub refresh_token: String,
    #[serde(default, rename = "m", skip_serializing_if = "String::is_empty")]
    pub mc_token: String,
}

impl Secrets {
    pub fn est_vide(&self) -> bool {
        self.refresh_token.is_empty() && self.mc_token.is_empty()
    }
}

/// Un endroit où ranger les jetons, un jeu de jetons par compte (uuid).
pub trait Coffre: Send + Sync {
    fn lire(&self, uuid: &str) -> Option<Secrets>;
    fn ecrire(&self, uuid: &str, secrets: &Secrets) -> anyhow::Result<()>;
    fn supprimer(&self, uuid: &str);
}

/// Le coffre de cette machine.
pub fn coffre() -> &'static dyn Coffre {
    #[cfg(any(windows, target_os = "linux"))]
    {
        &coffre_systeme::CoffreSysteme
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        static COFFRE: std::sync::OnceLock<CoffreFichier> = std::sync::OnceLock::new();
        COFFRE.get_or_init(|| CoffreFichier {
            chemin: crate::systeme::paths::data_dir().join("msa-secrets.json"),
        })
    }
}

#[cfg(any(windows, target_os = "linux"))]
mod coffre_systeme {
    use super::{Coffre, Secrets};

    /// Nom du service dans le Gestionnaire d'identifiants (garde « Misty », comme le dossier de
    /// données et l'AUMID).
    const SERVICE: &str = "Misty";
    /// Taille maximale d'un secret dans le Gestionnaire d'identifiants (CRED_MAX_CREDENTIAL_BLOB_SIZE).
    const TAILLE_MAX: usize = 2560;

    pub struct CoffreSysteme;

    fn entree(uuid: &str) -> keyring::Result<keyring::Entry> {
        keyring::Entry::new(SERVICE, &format!("msa-{uuid}"))
    }

    impl Coffre for CoffreSysteme {
        fn lire(&self, uuid: &str) -> Option<Secrets> {
            let octets = entree(uuid).ok()?.get_secret().ok()?;
            serde_json::from_slice(&octets).ok()
        }

        fn ecrire(&self, uuid: &str, secrets: &Secrets) -> anyhow::Result<()> {
            let mut octets = serde_json::to_vec(secrets)?;
            if octets.len() > TAILLE_MAX {
                // trop long pour une entrée : le jeton Minecraft sera redemandé avec le jeton de
                // renouvellement à la prochaine connexion
                let reduit = Secrets {
                    mc_token: String::new(),
                    ..secrets.clone()
                };
                octets = serde_json::to_vec(&reduit)?;
            }
            entree(uuid)?.set_secret(&octets)?;
            Ok(())
        }

        fn supprimer(&self, uuid: &str) {
            if let Ok(e) = entree(uuid) {
                let _ = e.delete_credential();
            }
        }
    }
}

/// Repli sans coffre du système : tous les jetons dans un fichier JSON à part.
#[cfg_attr(any(windows, target_os = "linux"), allow(dead_code))]
pub struct CoffreFichier {
    pub chemin: std::path::PathBuf,
}

#[cfg_attr(any(windows, target_os = "linux"), allow(dead_code))]
impl CoffreFichier {
    fn tout(&self) -> std::collections::BTreeMap<String, Secrets> {
        std::fs::read_to_string(&self.chemin)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    fn sauver(&self, tout: &std::collections::BTreeMap<String, Secrets>) -> anyhow::Result<()> {
        if tout.is_empty() {
            let _ = std::fs::remove_file(&self.chemin);
            return Ok(());
        }
        crate::systeme::fichiers::ecrire_atomique(
            &self.chemin,
            serde_json::to_string_pretty(tout)?,
        )?;
        Ok(())
    }
}

impl Coffre for CoffreFichier {
    fn lire(&self, uuid: &str) -> Option<Secrets> {
        self.tout().remove(uuid)
    }

    fn ecrire(&self, uuid: &str, secrets: &Secrets) -> anyhow::Result<()> {
        let mut tout = self.tout();
        tout.insert(uuid.to_string(), secrets.clone());
        self.sauver(&tout)
    }

    fn supprimer(&self, uuid: &str) {
        let mut tout = self.tout();
        if tout.remove(uuid).is_some() {
            let _ = self.sauver(&tout);
        }
    }
}

/// Coffre en mémoire, pour les tests (ne touche jamais au vrai Gestionnaire d'identifiants).
#[cfg(test)]
#[derive(Default)]
pub struct CoffreMemoire {
    pub entrees: std::sync::Mutex<std::collections::BTreeMap<String, Secrets>>,
    /// Simule un Gestionnaire d'identifiants indisponible
    pub en_panne: bool,
}

#[cfg(test)]
impl Coffre for CoffreMemoire {
    fn lire(&self, uuid: &str) -> Option<Secrets> {
        self.entrees.lock().unwrap().get(uuid).cloned()
    }

    fn ecrire(&self, uuid: &str, secrets: &Secrets) -> anyhow::Result<()> {
        anyhow::ensure!(!self.en_panne, "coffre indisponible");
        self.entrees
            .lock()
            .unwrap()
            .insert(uuid.to_string(), secrets.clone());
        Ok(())
    }

    fn supprimer(&self, uuid: &str) {
        self.entrees.lock().unwrap().remove(uuid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coffre_fichier_garde_un_jeu_par_compte() {
        let chemin = std::env::temp_dir().join(format!("misty-coffre-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&chemin);
        let c = CoffreFichier {
            chemin: chemin.clone(),
        };
        let s = |r: &str| Secrets {
            refresh_token: r.into(),
            mc_token: "mc".into(),
        };
        c.ecrire("a", &s("ra")).unwrap();
        c.ecrire("b", &s("rb")).unwrap();
        assert_eq!(c.lire("a"), Some(s("ra")));
        c.supprimer("a");
        assert_eq!(c.lire("a"), None);
        assert_eq!(c.lire("b"), Some(s("rb")));
        c.supprimer("b");
        assert!(!chemin.exists());
    }

    /// Vrai Gestionnaire d'identifiants : crée puis supprime une entrée « msa-test-coffre.Misty ».
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn gestionnaire_d_identifiants_windows() {
        let c = coffre_systeme::CoffreSysteme;
        let s = Secrets {
            refresh_token: "r".repeat(900),
            mc_token: "m".repeat(2000),
        };
        c.ecrire("test-coffre", &s).unwrap();
        // trop long pour une entrée : seul le jeton de renouvellement est gardé
        let relu = c.lire("test-coffre").unwrap();
        assert_eq!(relu.refresh_token, s.refresh_token);
        assert!(relu.mc_token.is_empty());
        c.supprimer("test-coffre");
        assert_eq!(c.lire("test-coffre"), None);
    }
}
