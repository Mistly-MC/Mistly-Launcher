//! Création / suppression du raccourci (choisi dans les options) vers Misty : le lanceur fixe une
//! fois installé (voir lanceur.rs), sinon l'exe courant.
//!
//! Windows : « Mistly.lnk ». Linux : « mistly.desktop », plus l'entrée du menu des applications
//! (`~/.local/share/applications/mistly.desktop`).

use std::path::Path;

const NOM_RACCOURCI: &str = if cfg!(windows) {
    "Mistly.lnk"
} else {
    "mistly.desktop"
};
/// Nom d'avant (l'app s'appelait Misty) : retiré avec le nouveau
const ANCIEN_NOM: &str = if cfg!(windows) {
    "Misty.lnk"
} else {
    "misty.desktop"
};

#[cfg(windows)]
pub fn creer(dossier: &Path) -> anyhow::Result<()> {
    if dossier.as_os_str().is_empty() {
        return Ok(());
    }
    supprimer(dossier);

    let mut lnk = mslnk::ShellLink::new(crate::systeme::lanceur::cible())
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    lnk.set_working_dir(
        crate::systeme::lanceur::cible()
            .parent()
            .map(|d| d.to_string_lossy().to_string()),
    );
    lnk.set_icon_location(Some(
        crate::systeme::lanceur::icone()
            .to_string_lossy()
            .to_string(),
    ));
    lnk.create_lnk(dossier.join(NOM_RACCOURCI))
        .map_err(|e| anyhow::anyhow!("{e:?}"))?;
    Ok(())
}

#[cfg(not(windows))]
pub fn creer(dossier: &Path) -> anyhow::Result<()> {
    if dossier.as_os_str().is_empty() {
        return Ok(());
    }
    supprimer(dossier);
    ecrire_entree(&dossier.join(NOM_RACCOURCI))
}

/// Contenu d'un fichier .desktop qui lance Misty (freedesktop.org).
pub fn entree_bureau() -> String {
    let cible = crate::systeme::lanceur::cible();
    // icône livrée à côté de l'exe de la version (remise à jour après chaque mise à jour)
    let icone = crate::systeme::paths::app_dir().join("icone.png");
    format!(
        "[Desktop Entry]\nType=Application\nName=Mistly Launcher\nComment=Minecraft launcher\n\
         Exec=\"{}\"\nPath={}\nIcon={}\nTerminal=false\nCategories=Game;\nStartupWMClass=Misty\n",
        cible.display(),
        cible
            .parent()
            .map(|d| d.display().to_string())
            .unwrap_or_default(),
        icone.display()
    )
}

/// Écrit un fichier .desktop, exécutable (sinon les bureaux refusent de le lancer).
#[cfg(not(windows))]
fn ecrire_entree(chemin: &Path) -> anyhow::Result<()> {
    if let Some(parent) = chemin.parent() {
        std::fs::create_dir_all(parent)?;
    }
    crate::systeme::fichiers::ecrire_atomique(chemin, entree_bureau())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(chemin, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

/// Linux : l'entrée du menu des applications, quand Misty est installé (dossiers « Misty-… »).
pub fn menu_applications() {
    #[cfg(not(windows))]
    if crate::systeme::lanceur::racine().is_some() {
        if let Some(dossier) = dirs::data_local_dir().map(|d| d.join("applications")) {
            let _ = ecrire_entree(&dossier.join(NOM_RACCOURCI));
        }
    }
}

pub fn supprimer(dossier: &Path) {
    if dossier.as_os_str().is_empty() {
        return;
    }
    for nom in [NOM_RACCOURCI, ANCIEN_NOM] {
        let chemin = dossier.join(nom);
        if chemin.exists() {
            let _ = std::fs::remove_file(chemin);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn entree_bureau_valide() {
        let e = super::entree_bureau();
        assert!(e.starts_with("[Desktop Entry]\n"));
        assert!(e.contains("\nType=Application\n") && e.contains("\nExec=\""));
        assert!(e.contains("Name=Mistly Launcher"));
    }
}
