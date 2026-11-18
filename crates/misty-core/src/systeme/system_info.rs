//! Informations système : RAM, taille de dossier, ouverture de dossiers et de liens.

use std::path::Path;

/// RAM physique totale, en Mo.
#[cfg(windows)]
pub fn ram_totale_mo() -> u32 {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    // SAFETY : structure initialisée à zéro avec sa taille, comme l'exige l'API Windows.
    unsafe {
        let mut status: MEMORYSTATUSEX = std::mem::zeroed();
        status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
        if GlobalMemoryStatusEx(&mut status) == 0 {
            return 8192;
        }
        (status.ullTotalPhys / 1024 / 1024) as u32
    }
}

#[cfg(not(windows))]
pub fn ram_totale_mo() -> u32 {
    8192
}

/// RAM maximale proposée : la RAM totale arrondie au Go supérieur, moins 1 Go pour le système.
pub fn ram_max_proposee_mo() -> u32 {
    let total_go = (ram_totale_mo() as f64 / 1024.0).ceil() as u32;
    total_go.saturating_sub(1).max(1) * 1024
}

/// Le launcher est-il en mode éco ? (pour ne rien demander à Windows quand rien ne change)
static ECO: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Mode éco de Windows 11 (« mode efficacité » du gestionnaire des tâches) pour le launcher
/// lui-même : Windows le fait tourner à basse fréquence, sur les cœurs économes s'il y en a.
/// Demandé pendant une partie, quand on ne regarde pas le launcher : le processeur reste au jeu.
/// Sans effet sur les versions de Windows qui ne le connaissent pas.
pub fn mode_eco(actif: bool) {
    use std::sync::atomic::Ordering;
    if ECO.swap(actif, Ordering::SeqCst) == actif {
        return;
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Threading::{
            GetCurrentProcess, ProcessPowerThrottling, SetProcessInformation,
            PROCESS_POWER_THROTTLING_CURRENT_VERSION, PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
            PROCESS_POWER_THROTTLING_STATE,
        };
        let etat = PROCESS_POWER_THROTTLING_STATE {
            Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
            ControlMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
            StateMask: if actif {
                PROCESS_POWER_THROTTLING_EXECUTION_SPEED
            } else {
                0
            },
        };
        // SAFETY : pseudo-handle du processus courant (toujours valide) et pointeur vers une
        // structure de la taille annoncée, comme l'exige l'API ; un refus de Windows est ignoré.
        unsafe {
            SetProcessInformation(
                GetCurrentProcess(),
                ProcessPowerThrottling,
                &etat as *const _ as *const core::ffi::c_void,
                std::mem::size_of::<PROCESS_POWER_THROTTLING_STATE>() as u32,
            );
        }
    }
}

/// À appeler juste avant de lancer un autre programme (le jeu, un éditeur…) : le launcher quitte
/// le mode éco, pour que le nouveau processus ne risque pas d'en hériter.
pub fn avant_processus() {
    mode_eco(false);
}

/// Rend à Windows la mémoire que le launcher n'utilise pas en ce moment (elle revient d'elle-même
/// quand il en a besoin). Appelé quand une partie démarre : la mémoire va au jeu.
pub fn liberer_memoire() {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Threading::{GetCurrentProcess, SetProcessWorkingSetSize};
        // SAFETY : pseudo-handle du processus courant ; (-1, -1) demande à Windows de retirer de
        // la mémoire vive les pages inutilisées, sans rien libérer ni invalider.
        unsafe {
            SetProcessWorkingSetSize(GetCurrentProcess(), usize::MAX, usize::MAX);
        }
    }
}

/// Taille totale d'un dossier en octets (0 s'il n'existe pas).
pub fn taille_dossier(dossier: &Path) -> u64 {
    let Ok(entrees) = std::fs::read_dir(dossier) else {
        return 0;
    };
    let mut total = 0;
    for entree in entrees.flatten() {
        let Ok(meta) = entree.metadata() else {
            continue;
        };
        if meta.is_dir() {
            total += taille_dossier(&entree.path());
        } else {
            total += meta.len();
        }
    }
    total
}

/// Ouvre un dossier dans l'explorateur (le crée s'il n'existe pas).
pub fn ouvrir_dossier(dossier: &Path) {
    let _ = std::fs::create_dir_all(dossier);
    let _ = std::process::Command::new("explorer.exe")
        .arg(dossier)
        .spawn();
}

/// Ouvre l'explorateur avec un fichier sélectionné.
pub fn montrer_fichier(fichier: &Path) {
    let _ = std::process::Command::new("explorer.exe")
        .arg(format!("/select,{}", fichier.display()))
        .spawn();
}

/// Ouvre une URL (ou un fichier) avec l'application par défaut.
pub fn ouvrir(cible: &str) {
    avant_processus();
    let _ = open::that_detached(cible);
}

/// Windows est en mode sombre pour les applications (Paramètres → Personnalisation → Couleurs).
#[cfg(windows)]
pub fn windows_sombre() -> bool {
    use winreg::enums::HKEY_CURRENT_USER;
    winreg::RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .and_then(|c| c.get_value::<u32, _>("AppsUseLightTheme"))
        .is_ok_and(|clair| clair == 0)
}

#[cfg(not(windows))]
pub fn windows_sombre() -> bool {
    false
}

/// Version de Windows, comme « Windows 10.0.19045 » (lue dans le registre).
#[cfg(windows)]
pub fn version_windows() -> String {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    let Ok(cle) = winreg::RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion")
    else {
        return "Windows".into();
    };
    let majeure: u32 = cle.get_value("CurrentMajorVersionNumber").unwrap_or(10);
    let mineure: u32 = cle.get_value("CurrentMinorVersionNumber").unwrap_or(0);
    let build: String = cle.get_value("CurrentBuild").unwrap_or_default();
    if build.is_empty() {
        format!("Windows {majeure}.{mineure}")
    } else {
        format!("Windows {majeure}.{mineure}.{build}")
    }
}

#[cfg(not(windows))]
pub fn version_windows() -> String {
    std::env::consts::OS.to_string()
}
