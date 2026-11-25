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
//! Écriture des fichiers du joueur (comptes, réglages, profils…) sans jamais les laisser à moitié
//! écrits : si le PC s'éteint ou si le launcher plante pendant l'écriture, l'ancien fichier reste
//! intact.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Une seule écriture à la fois : deux fils qui enregistrent le même fichier se partageraient
/// sinon le même `.tmp`.
static VERROU: Mutex<()> = Mutex::new(());

/// `<fichier>.tmp`, à côté du fichier (même disque, donc le renommage est atomique).
fn temporaire(chemin: &Path) -> PathBuf {
    let mut nom = chemin.as_os_str().to_owned();
    nom.push(".tmp");
    PathBuf::from(nom)
}

/// Écrit `contenu` dans `<chemin>.tmp`, le force sur le disque, puis le renomme en `chemin`
/// (remplace l'ancien fichier d'un coup). Comme `fs::write`, ne crée pas le dossier parent.
pub fn ecrire_atomique(chemin: impl AsRef<Path>, contenu: impl AsRef<[u8]>) -> std::io::Result<()> {
    let chemin = chemin.as_ref();
    let tmp = temporaire(chemin);
    let _verrou = VERROU.lock().unwrap_or_else(|e| e.into_inner());
    let resultat = (|| {
        let mut fichier = std::fs::File::create(&tmp)?;
        fichier.write_all(contenu.as_ref())?;
        fichier.sync_all()?;
        drop(fichier);
        renommer(&tmp, chemin)
    })();
    if resultat.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    resultat
}

/// Sous Windows, le renommage échoue tant qu'un autre programme (antivirus, indexation, le jeu
/// qui lit `client.json`) garde le fichier ouvert : on réessaie un court instant.
fn renommer(de: &Path, vers: &Path) -> std::io::Result<()> {
    let mut essais = 0;
    loop {
        match std::fs::rename(de, vers) {
            Err(e) if essais < 10 && e.kind() == std::io::ErrorKind::PermissionDenied => {
                essais += 1;
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            r => return r,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dossier_test(nom: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("misty-fichiers-{nom}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn remplace_le_fichier_sans_laisser_de_tmp() {
        let d = dossier_test("remplace");
        let f = d.join("msa.json");
        ecrire_atomique(&f, "ancien").unwrap();
        ecrire_atomique(&f, "nouveau").unwrap();
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "nouveau");
        assert!(!d.join("msa.json.tmp").exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn echec_garde_l_ancien_fichier() {
        let d = dossier_test("echec");
        // le dossier parent n'existe pas : rien n'est écrit, rien n'est cassé
        assert!(ecrire_atomique(d.join("absent").join("x.txt"), "a").is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
