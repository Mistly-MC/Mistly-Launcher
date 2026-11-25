//! Liste des pseudos hors-ligne, stockée dans listpseudo.txt ("listpseudo=a,b,c").

use crate::systeme::paths;
use std::fs;

const CLE: &str = "listpseudo=";

/// Pseudo que les versions d'avant 0.4.3 ajoutaient d'office à la liste.
const ANCIEN_PSEUDO_DEFAUT: &str = "Mistly";

/// Crée la liste (vide : le premier lancement fait choisir un compte). `compte_microsoft` : le
/// joueur a un compte Microsoft ; le pseudo « Mistly » que les anciennes versions ajoutaient
/// d'office est alors retiré s'il est seul dans la liste et n'est pas le pseudo utilisé.
pub fn initialiser(compte_microsoft: bool, pseudo_utilise: &str) {
    let _ = fs::create_dir_all(paths::data_dir());
    if !paths::profiles_file().exists() {
        let _ =
            crate::systeme::fichiers::ecrire_atomique(paths::profiles_file(), format!("{CLE}\r\n"));
        return;
    }
    if compte_microsoft
        && pseudo_utilise != ANCIEN_PSEUDO_DEFAUT
        && lire() == [ANCIEN_PSEUDO_DEFAUT]
    {
        supprimer(ANCIEN_PSEUDO_DEFAUT);
    }
}

pub fn lire() -> Vec<String> {
    let Ok(texte) = fs::read_to_string(paths::profiles_file()) else {
        return Vec::new();
    };
    let mut resultat = Vec::new();
    for ligne in texte.lines() {
        if let Some(valeur) = ligne.strip_prefix(CLE) {
            if !valeur.is_empty() {
                resultat.extend(valeur.split(',').map(str::to_string));
            }
        }
    }
    resultat
}

pub fn ajouter(pseudo: &str) {
    if pseudo.trim().is_empty() {
        return;
    }
    let mut pseudos = lire();
    if pseudos.iter().any(|p| p == pseudo) {
        return;
    }
    pseudos.push(pseudo.to_string());
    ecrire(&pseudos);
}

pub fn supprimer(pseudo: &str) {
    let mut pseudos = lire();
    let avant = pseudos.len();
    pseudos.retain(|p| p != pseudo);
    if pseudos.len() != avant {
        ecrire(&pseudos);
    }
}

fn ecrire(pseudos: &[String]) {
    let Ok(texte) = fs::read_to_string(paths::profiles_file()) else {
        return;
    };
    let nouvelle = format!("{CLE}{}", pseudos.join(","));
    let mut lignes: Vec<String> = texte.lines().map(str::to_string).collect();
    match lignes.iter().position(|l| l.starts_with(CLE)) {
        Some(i) => lignes[i] = nouvelle,
        None => lignes.push(nouvelle),
    }
    let _ = crate::systeme::fichiers::ecrire_atomique(
        paths::profiles_file(),
        lignes.join("\r\n") + "\r\n",
    );
}
