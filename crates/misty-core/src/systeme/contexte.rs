//! Ce que tout le launcher partage : le dossier de données, le client HTTP et le runtime tokio.
//!
//! `main` crée le contexte (`Contexte::nouveau`) puis l'installe (`installer`). Les services le
//! lisent avec `actuel()`, directement ou par `paths::data_dir()` et `http::client()`. Sans
//! contexte installé (fenêtre de connexion, lanceur fixe, tests), le contexte par défaut est
//! créé à la première lecture.
//!
//! Tests : `pour_test(dossier)` remplace le contexte le temps d'un test, sans variable
//! d'environnement ; les tests qui l'utilisent passent un par un.

use crate::systeme::app_info;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, RwLock};
use std::time::Duration;

/// Le contexte du launcher.
pub struct Contexte {
    /// Dossier de données : %LOCALAPPDATA%\Misty (contient aussi le .minecraft)
    pub dossier: PathBuf,
    /// Client HTTP général (Modrinth, Mojang, loaders…), avec le User-Agent du launcher
    pub http: reqwest::Client,
    /// Runtime tokio du launcher (None hors de l'application : outils, tests)
    pub runtime: Option<tokio::runtime::Handle>,
}

impl Contexte {
    pub fn nouveau(dossier: PathBuf, runtime: Option<tokio::runtime::Handle>) -> Self {
        Contexte {
            dossier,
            http: reqwest::Client::builder()
                .user_agent(app_info::user_agent())
                .timeout(Duration::from_secs(300))
                .connect_timeout(Duration::from_secs(20))
                .build()
                .expect("client HTTP"),
            runtime,
        }
    }

    /// Le dossier de données habituel : %LOCALAPPDATA%\Misty.
    ///
    /// Développement : MISTY_DATA_DIR=<dossier> lance le launcher sur un dossier à part (profils
    /// et packs de démonstration pour les captures du site, sans toucher aux vraies données).
    pub fn dossier_par_defaut() -> PathBuf {
        #[cfg(debug_assertions)]
        if let Some(d) = std::env::var_os("MISTY_DATA_DIR") {
            return PathBuf::from(d);
        }
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Misty")
    }
}

static ACTUEL: RwLock<Option<Arc<Contexte>>> = RwLock::new(None);

/// Installe le contexte de l'application (au démarrage, dans `main`).
pub fn installer(contexte: Contexte) -> Arc<Contexte> {
    let contexte = Arc::new(contexte);
    *ACTUEL.write().unwrap_or_else(|e| e.into_inner()) = Some(contexte.clone());
    contexte
}

/// Le contexte actuel (celui par défaut s'il n'a pas été installé).
pub fn actuel() -> Arc<Contexte> {
    if let Some(c) = ACTUEL.read().unwrap_or_else(|e| e.into_inner()).as_ref() {
        return c.clone();
    }
    let mut actuel = ACTUEL.write().unwrap_or_else(|e| e.into_inner());
    actuel
        .get_or_insert_with(|| Arc::new(Contexte::nouveau(Contexte::dossier_par_defaut(), None)))
        .clone()
}

/// Remet le contexte d'avant à la fin d'un test.
pub struct GardeContexte {
    avant: Option<Arc<Contexte>>,
    _un_seul: MutexGuard<'static, ()>,
}

impl Drop for GardeContexte {
    fn drop(&mut self) {
        *ACTUEL.write().unwrap_or_else(|e| e.into_inner()) = self.avant.take();
    }
}

/// Tests : remplace le dossier de données par `dossier` jusqu'à la fin de la garde. Les tests qui
/// l'utilisent passent un par un (les autres ne lisent pas le dossier de données).
pub fn pour_test(dossier: &Path) -> GardeContexte {
    static UN_SEUL: Mutex<()> = Mutex::new(());
    let un_seul = UN_SEUL.lock().unwrap_or_else(|e| e.into_inner());
    let _ = std::fs::create_dir_all(dossier);
    let nouveau = Arc::new(Contexte::nouveau(dossier.to_path_buf(), None));
    let avant = ACTUEL
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .replace(nouveau);
    GardeContexte {
        avant,
        _un_seul: un_seul,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::systeme::{paths, reglages};

    #[test]
    fn dossier_injecte_dans_un_test() {
        let d = std::env::temp_dir().join(format!("misty-contexte-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        {
            let _garde = pour_test(&d);
            assert_eq!(paths::data_dir(), d);
            // les réglages suivent le dossier du contexte
            reglages::modifier(|r| r.ram_mo = 1234);
            assert_eq!(reglages::lire().ram_mo, 1234);
            assert!(d.join("reglages.json").exists());
        }
        // contexte d'avant remis en place
        assert_ne!(paths::data_dir(), d);
        let _ = std::fs::remove_dir_all(&d);
    }
}
