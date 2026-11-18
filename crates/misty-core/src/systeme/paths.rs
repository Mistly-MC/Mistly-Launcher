//! Tous les chemins utilisés par le launcher.

use std::path::PathBuf;

/// Dossier de données : %LOCALAPPDATA%\Misty (contient aussi le .minecraft), celui du contexte
/// (voir `systeme::contexte`, et `Contexte::dossier_par_defaut` pour MISTY_DATA_DIR).
pub fn data_dir() -> PathBuf {
    crate::systeme::contexte::actuel().dossier.clone()
}

pub fn profiles_file() -> PathBuf {
    data_dir().join("listpseudo.txt")
}

/// Ancien dossier de données (avant 0.2.3.9.5), migré au démarrage.
pub const LEGACY_DATA_DIR: &str = r"C:\TEXT\";

/// Chemin de l'exécutable en cours.
pub fn app_exe() -> PathBuf {
    std::env::current_exe().unwrap_or_else(|_| PathBuf::from("Misty.exe"))
}

/// Dossier de l'exécutable.
pub fn app_dir() -> PathBuf {
    app_exe()
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Icône personnalisée optionnelle posée à côté de l'exe.
pub fn custom_icon() -> PathBuf {
    app_dir().join("Icon.ico")
}
