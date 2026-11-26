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
//! Mistly-Setup.exe : l'installateur de Mistly Launcher, écrit en Rust (voir installation.rs).
//!
//! Première installation : bienvenue, dossier, conditions, Install, puis Launch. Si le launcher est
//! déjà installé : mettre à jour (ou réparer), ou désinstaller en gardant ou non les données.
//! `--uninstall` (bouton Désinstaller de Windows) ouvre directement la désinstallation ;
//! `--uninstall --quiet` désinstalle sans fenêtre (en gardant les données).

#![cfg_attr(windows, windows_subsystem = "windows")]

mod installation;

use slint::ComponentHandle;
use std::path::PathBuf;

slint::include_modules!();

/// « 0.4.7-beta.1 » plus récente que « 0.4.6 » ? (une pré-release passe avant sa version finale)
fn plus_recente(a: &str, b: &str) -> bool {
    fn cle(v: &str) -> (Vec<u64>, bool, Vec<u64>) {
        let (base, pre) = v.split_once('-').unwrap_or((v, ""));
        let nombres = |t: &str| -> Vec<u64> {
            t.split(|c: char| !c.is_ascii_digit())
                .filter(|x| !x.is_empty())
                .filter_map(|x| x.parse().ok())
                .collect()
        };
        (nombres(base), pre.is_empty(), nombres(pre))
    }
    cle(a) > cle(b)
}

fn main() -> Result<(), slint::PlatformError> {
    let arguments: Vec<String> = std::env::args().collect();
    let desinstaller = arguments.iter().any(|a| a == "--uninstall");
    let existante = installation::trouver();

    if desinstaller && arguments.iter().any(|a| a == "--quiet") {
        if let Some(e) = &existante {
            let _ = installation::desinstaller(&e.dossier, true);
        }
        return Ok(());
    }

    let fenetre = FenetreSetup::new()?;
    fenetre.set_version(installation::VERSION.into());
    match &existante {
        Some(e) => {
            fenetre.set_dossier(e.dossier.to_string_lossy().to_string().into());
            fenetre.set_version_installee(e.version.as_str().into());
            fenetre.set_plus_recente(
                e.version.is_empty() || plus_recente(installation::VERSION, &e.version),
            );
            fenetre.set_etape(if desinstaller { 5 } else { 4 });
        }
        None => {
            fenetre.set_dossier(
                installation::dossier_par_defaut()
                    .to_string_lossy()
                    .to_string()
                    .into(),
            );
            fenetre.set_etape(0);
        }
    }

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
    let mise_a_jour = existante.is_some();
    fenetre.on_installer(move || {
        let Some(f) = faible.upgrade() else { return };
        f.set_travail(
            if mise_a_jour {
                "Updating Mistly Launcher…"
            } else {
                "Installing Mistly Launcher…"
            }
            .into(),
        );
        f.set_etape(1);
        let dossier = PathBuf::from(f.get_dossier().as_str());
        let bureau = f.get_bureau();
        let faible = faible.clone();
        std::thread::spawn(move || {
            let resultat = installation::installer(&dossier, bureau);
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
    fenetre.on_desinstaller(move || {
        let Some(f) = faible.upgrade() else { return };
        f.set_travail("Uninstalling Mistly Launcher…".into());
        f.set_etape(1);
        let dossier = PathBuf::from(f.get_dossier().as_str());
        let garder = f.get_garder();
        let faible = faible.clone();
        std::thread::spawn(move || {
            let resultat = installation::desinstaller(&dossier, garder);
            let _ = faible.upgrade_in_event_loop(move |f| match resultat {
                Ok(()) => f.set_etape(6),
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
        installation::lancer(&PathBuf::from(f.get_dossier().as_str()));
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

#[cfg(test)]
mod tests {
    use super::plus_recente;

    #[test]
    fn compare_les_versions() {
        assert!(plus_recente("0.4.7-beta.1", "0.4.6"));
        assert!(plus_recente("0.4.7", "0.4.7-beta.2"));
        assert!(plus_recente("0.4.7-beta.2", "0.4.7-beta.1"));
        assert!(!plus_recente("0.4.7-beta.1", "0.4.7-beta.1"));
        assert!(!plus_recente("0.4.6", "0.4.7-beta.1"));
    }
}
