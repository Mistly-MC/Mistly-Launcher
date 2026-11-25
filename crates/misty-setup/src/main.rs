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
//! Mistly-Setup.exe : la fenêtre d'installation de Mistly Launcher (dossier, conditions,
//! Install, puis Launch), dans le style du launcher.
//!
//! L'installation elle-même reste celle d'Inno Setup (installer/Misty.iss : même AppId, même
//! désinstallation, mêmes raccourcis) : son installateur est intégré à cet exe (build.rs) et lancé
//! sans fenêtre, avec le dossier choisi. Sans lui (développement), l'installation est simulée.

#![cfg_attr(windows, windows_subsystem = "windows")]

use slint::ComponentHandle;
use std::path::PathBuf;

slint::include_modules!();

/// L'installateur Inno Setup (vide hors de installer/build.ps1).
const INTERIEUR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/interieur.exe"));

/// Dossier par défaut : celui d'Inno Setup ({localappdata}\Programs\Misty, à ne pas changer).
fn dossier_par_defaut() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("Programs")
        .join("Misty")
}

/// Lance l'installateur Inno Setup sans fenêtre dans `dossier` et attend sa fin.
fn installer(dossier: &str) -> Result<(), String> {
    if INTERIEUR.is_empty() {
        // développement : rien à installer
        std::thread::sleep(std::time::Duration::from_secs(2));
        return Ok(());
    }
    let temporaire = std::env::temp_dir().join(format!("mistly-setup-{}.exe", std::process::id()));
    std::fs::write(&temporaire, INTERIEUR).map_err(|e| e.to_string())?;
    let statut = std::process::Command::new(&temporaire)
        .args([
            "/VERYSILENT",
            "/SUPPRESSMSGBOXES",
            "/NORESTART",
            "/SP-",
            "/TASKS=desktopicon",
        ])
        .arg(format!("/DIR={dossier}"))
        .status();
    let _ = std::fs::remove_file(&temporaire);
    match statut {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(format!(
            "The installer stopped with code {}.",
            s.code().unwrap_or(-1)
        )),
        Err(e) => Err(e.to_string()),
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let fenetre = FenetreSetup::new()?;
    fenetre.set_dossier(dossier_par_defaut().to_string_lossy().to_string().into());

    let faible = fenetre.as_weak();
    fenetre.on_parcourir(move || {
        let Some(f) = faible.upgrade() else { return };
        if let Some(d) = rfd::FileDialog::new()
            .set_directory(f.get_dossier().as_str())
            .pick_folder()
        {
            f.set_dossier(d.join("Misty").to_string_lossy().to_string().into());
        }
    });

    let faible = fenetre.as_weak();
    fenetre.on_installer(move || {
        let Some(f) = faible.upgrade() else { return };
        f.set_etape(1);
        let dossier = f.get_dossier().to_string();
        let faible = faible.clone();
        std::thread::spawn(move || {
            let resultat = installer(&dossier);
            let _ = faible.upgrade_in_event_loop(move |f| match resultat {
                Ok(()) => f.set_etape(2),
                Err(e) => {
                    f.set_erreur(e.into());
                    f.set_etape(3);
                }
            });
        });
    });

    let faible = fenetre.as_weak();
    fenetre.on_lancer(move || {
        let Some(f) = faible.upgrade() else { return };
        // le lanceur fixe démarre la version installée
        let exe = PathBuf::from(f.get_dossier().as_str()).join("Misty.exe");
        let _ = std::process::Command::new(exe).spawn();
        let _ = f.hide();
    });

    fenetre.on_ouvrir_lien(|adresse| {
        let _ = open::that_detached(adresse.as_str());
    });

    let faible = fenetre.as_weak();
    fenetre.on_deplacer(move || {
        use slint::winit_030::WinitWindowAccessor;
        if let Some(f) = faible.upgrade() {
            f.window().with_winit_window(|w| {
                let _ = w.drag_window();
            });
        }
    });
    let faible = fenetre.as_weak();
    fenetre.on_reduire(move || {
        use slint::winit_030::WinitWindowAccessor;
        if let Some(f) = faible.upgrade() {
            f.window().with_winit_window(|w| w.set_minimized(true));
        }
    });
    let faible = fenetre.as_weak();
    fenetre.on_fermer(move || {
        if let Some(f) = faible.upgrade() {
            let _ = f.hide();
        }
    });

    fenetre.run()
}
