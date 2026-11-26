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
//! Installation, mise à jour et désinstallation de Mistly Launcher, sans Inno Setup.
//!
//! Même structure qu'avant (et que la mise à jour automatique, voir
//! crates/misty-core/src/systeme/update.rs et lanceur.rs) :
//!   <dossier>\Misty-<version>\Misty.exe   la version installée
//!   <dossier>\Misty.exe                   le lanceur fixe, visé par les raccourcis
//!   <dossier>\uninstall\Mistly-Setup.exe  cet installateur, pour la désinstallation
//! L'entrée d'« Applications installées » garde la clé d'Inno Setup (son AppId) : les
//! installations faites par l'ancien installateur sont reconnues, et le launcher continue d'y
//! écrire sa version après une mise à jour automatique.

use std::path::{Path, PathBuf};

/// Version du launcher contenue dans cet installateur.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Le launcher compressé (vide hors de installer/build.ps1).
const LAUNCHER: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/misty.exe.zst"));

const NOM_EXE: &str = "Misty.exe";

/// Clé de l'entrée d'« Applications installées » (l'AppId d'Inno Setup, à ne jamais changer).
#[cfg(windows)]
const CLE: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{8F4C2A61-3B7E-4D19-9A55-6C1E0F2B7D43}_is1";

/// Une installation trouvée sur ce PC.
#[derive(Clone, Debug)]
pub struct Existante {
    pub dossier: PathBuf,
    pub version: String,
}

/// Dossier par défaut : %LOCALAPPDATA%\Programs\Misty (le même qu'avant, à ne pas renommer).
pub fn dossier_par_defaut() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("Programs")
        .join("Misty")
}

/// Données du launcher (versions de Minecraft, profils, mondes) : %LOCALAPPDATA%\Misty.
pub fn dossier_donnees() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("Misty")
}

/// L'installation de ce PC, si elle existe (entrée d'« Applications installées », sinon le dossier
/// par défaut).
pub fn trouver() -> Option<Existante> {
    #[cfg(windows)]
    {
        use winreg::enums::HKEY_CURRENT_USER;
        if let Ok(cle) = winreg::RegKey::predef(HKEY_CURRENT_USER).open_subkey(CLE) {
            let dossier: Option<String> = cle
                .get_value("InstallLocation")
                .or_else(|_| cle.get_value("Inno Setup: App Path"))
                .ok();
            let version: String = cle.get_value("DisplayVersion").unwrap_or_default();
            if let Some(d) = dossier.filter(|d| !d.is_empty()) {
                let dossier = PathBuf::from(d.trim_end_matches(['\\', '/']));
                if dossier.join(NOM_EXE).exists() {
                    return Some(Existante { dossier, version });
                }
            }
        }
    }
    let dossier = dossier_par_defaut();
    dossier.join(NOM_EXE).exists().then(|| Existante {
        version: version_dans(&dossier).unwrap_or_default(),
        dossier,
    })
}

/// La version la plus récente d'un dossier d'installation (« Misty-<version> »).
fn version_dans(dossier: &Path) -> Option<String> {
    std::fs::read_dir(dossier)
        .ok()?
        .flatten()
        .filter_map(|e| {
            e.file_name()
                .to_string_lossy()
                .strip_prefix("Misty-")
                .map(str::to_string)
        })
        .max()
}

/// Installe (ou met à jour, ou répare) le launcher dans `dossier`.
pub fn installer(dossier: &Path, bureau: bool) -> Result<(), String> {
    if LAUNCHER.is_empty() {
        // développement : rien à installer
        std::thread::sleep(std::time::Duration::from_secs(2));
        return Ok(());
    }
    let exe = zstd::bulk::decompress(LAUNCHER, 512 * 1024 * 1024)
        .map_err(|e| format!("The installer is damaged ({e}). Download it again."))?;
    std::fs::create_dir_all(dossier).map_err(|e| e.to_string())?;

    // le lanceur fixe d'abord : s'il est pris, Mistly est ouvert
    let lanceur = dossier.join(NOM_EXE);
    let temporaire = dossier.join("Misty.exe.new");
    std::fs::write(&temporaire, &exe).map_err(|e| e.to_string())?;
    if std::fs::rename(&temporaire, &lanceur).is_err() {
        let _ = std::fs::remove_file(&temporaire);
        return Err("Mistly Launcher is open: close it, then try again.".into());
    }

    // la version, à côté ; les anciennes versions partent
    let version = dossier.join(format!("Misty-{VERSION}"));
    if let Ok(lecture) = std::fs::read_dir(dossier) {
        for e in lecture.flatten() {
            let nom = e.file_name().to_string_lossy().to_string();
            if nom.starts_with("Misty-") && e.path() != version && e.path().is_dir() {
                let _ = std::fs::remove_dir_all(e.path());
            }
        }
    }
    std::fs::create_dir_all(&version).map_err(|e| e.to_string())?;
    std::fs::write(version.join(NOM_EXE), &exe).map_err(|e| e.to_string())?;
    let _ = std::fs::write(
        version.join("LICENSE.txt"),
        include_str!("../../../LICENSE.txt"),
    );

    // cet installateur, pour la désinstallation (l'ancien désinstallateur d'Inno Setup part)
    let desinstallation = dossier.join("uninstall");
    let _ = std::fs::remove_dir_all(&desinstallation);
    let _ = std::fs::create_dir_all(&desinstallation);
    if let Ok(moi) = std::env::current_exe() {
        let _ = std::fs::copy(moi, desinstallation.join("Mistly-Setup.exe"));
    }

    systeme::raccourcis(&lanceur, bureau);
    systeme::inscrire(dossier, exe.len() as u64);
    Ok(())
}

/// Désinstalle le launcher de `dossier`. `garder` : garde les données (versions, profils,
/// mondes) pour une réinstallation.
pub fn desinstaller(dossier: &Path, garder: bool) -> Result<(), String> {
    let lanceur = dossier.join(NOM_EXE);
    // Mistly ouvert : son exe ne peut pas être renommé
    if lanceur.exists() {
        let essai = dossier.join("Misty.exe.old");
        if std::fs::rename(&lanceur, &essai).is_err() {
            return Err("Mistly Launcher is open: close it, then try again.".into());
        }
        let _ = std::fs::rename(&essai, &lanceur);
    }
    systeme::retirer_raccourcis();
    systeme::desinscrire();
    if !garder {
        let _ = std::fs::remove_dir_all(dossier_donnees());
    }
    // tout le dossier, sauf cet exe s'il tourne depuis lui : supprimé juste après la fermeture
    if let Ok(lecture) = std::fs::read_dir(dossier) {
        for e in lecture.flatten() {
            let p = e.path();
            if p.is_dir() {
                let _ = std::fs::remove_dir_all(&p);
            } else {
                let _ = std::fs::remove_file(&p);
            }
        }
    }
    systeme::supprimer_apres_fermeture(dossier);
    Ok(())
}

/// Démarre le launcher installé.
pub fn lancer(dossier: &Path) {
    let _ = std::process::Command::new(dossier.join(NOM_EXE))
        .current_dir(dossier)
        .spawn();
}

#[cfg(windows)]
mod systeme {
    use std::path::Path;
    use winreg::enums::HKEY_CURRENT_USER;

    fn dossiers_raccourcis() -> Vec<std::path::PathBuf> {
        let mut v = Vec::new();
        if let Some(d) = dirs::data_dir() {
            v.push(d.join(r"Microsoft\Windows\Start Menu\Programs"));
        }
        if let Some(d) = dirs::desktop_dir() {
            v.push(d);
        }
        v
    }

    /// « Mistly » dans le menu Démarrer (et sur le bureau) ; le launcher y ajoute son identifiant
    /// de barre des tâches à son premier démarrage (lanceur.rs : rediriger_raccourcis).
    pub fn raccourcis(lanceur: &Path, bureau: bool) {
        for (i, dossier) in dossiers_raccourcis().into_iter().enumerate() {
            let _ = std::fs::remove_file(dossier.join("Misty.lnk"));
            if i == 1 && !bureau {
                continue;
            }
            let Ok(mut lnk) = mslnk::ShellLink::new(lanceur) else {
                continue;
            };
            lnk.set_working_dir(lanceur.parent().map(|d| d.to_string_lossy().to_string()));
            lnk.set_icon_location(Some(lanceur.to_string_lossy().to_string()));
            let _ = std::fs::create_dir_all(&dossier);
            let _ = lnk.create_lnk(dossier.join("Mistly.lnk"));
        }
    }

    pub fn retirer_raccourcis() {
        for dossier in dossiers_raccourcis() {
            let _ = std::fs::remove_file(dossier.join("Mistly.lnk"));
            let _ = std::fs::remove_file(dossier.join("Misty.lnk"));
        }
    }

    /// L'entrée d'« Applications installées » (la même clé qu'Inno Setup).
    pub fn inscrire(dossier: &Path, taille: u64) {
        let Ok((cle, _)) = winreg::RegKey::predef(HKEY_CURRENT_USER).create_subkey(super::CLE)
        else {
            return;
        };
        let d = dossier.to_string_lossy().to_string();
        let desinstallateur = dossier.join(r"uninstall\Mistly-Setup.exe");
        let d_exe = desinstallateur.to_string_lossy().to_string();
        let _ = cle.set_value("DisplayName", &"Mistly Launcher");
        let _ = cle.set_value("DisplayVersion", &super::VERSION);
        let _ = cle.set_value("Publisher", &"Mistly");
        let _ = cle.set_value("URLInfoAbout", &"https://mistly.cc");
        let _ = cle.set_value(
            "HelpLink",
            &"https://github.com/Mistly-MC/Mistly-Launcher/issues",
        );
        let _ = cle.set_value("InstallLocation", &format!("{d}\\"));
        let _ = cle.set_value("Inno Setup: App Path", &d);
        let _ = cle.set_value(
            "DisplayIcon",
            &dossier.join(super::NOM_EXE).to_string_lossy().to_string(),
        );
        let _ = cle.set_value("UninstallString", &format!("\"{d_exe}\" --uninstall"));
        let _ = cle.set_value(
            "QuietUninstallString",
            &format!("\"{d_exe}\" --uninstall --quiet"),
        );
        let _ = cle.set_value("ModifyPath", &format!("\"{d_exe}\""));
        let _ = cle.set_value("NoRepair", &1u32);
        let _ = cle.set_value("EstimatedSize", &((taille / 1024) as u32));
    }

    pub fn desinscrire() {
        let _ = winreg::RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(super::CLE);
    }

    /// Le dossier (et cet exe s'il est dedans) est supprimé deux secondes après la fermeture.
    pub fn supprimer_apres_fermeture(dossier: &Path) {
        use std::os::windows::process::CommandExt;
        const SANS_FENETRE: u32 = 0x0800_0000;
        let d = dossier.to_string_lossy().to_string();
        let _ = std::process::Command::new("cmd")
            .args([
                "/C",
                &format!("timeout /T 2 /NOBREAK >NUL & rmdir /S /Q \"{d}\""),
            ])
            .creation_flags(SANS_FENETRE)
            .spawn();
    }
}

/// Hors Windows (développement) : pas de raccourcis ni d'entrée système.
#[cfg(not(windows))]
mod systeme {
    use std::path::Path;
    pub fn raccourcis(_lanceur: &Path, _bureau: bool) {}
    pub fn retirer_raccourcis() {}
    pub fn inscrire(_dossier: &Path, _taille: u64) {}
    pub fn desinscrire() {}
    pub fn supprimer_apres_fermeture(dossier: &Path) {
        let _ = std::fs::remove_dir_all(dossier);
    }
}
