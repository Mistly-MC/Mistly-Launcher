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
//! Accueil : versions de Minecraft du jeu rapide (liste, filtre, choix).

use super::*;

pub(super) async fn charger_versions(app: &Rc<App>) {
    match app.bg(install::versions_proposees()).await {
        Ok(v) => {
            h(app).set_versions(modele_textes(&v));
            app.home.borrow_mut().versions = v;
            // la fenêtre de choix propose toutes les versions, filtrées par type
            if let Ok(m) = app.bg(install::manifeste()).await {
                let toutes = m
                    .into_iter()
                    .filter_map(|v| {
                        let kind = match v.type_.as_str() {
                            "release" => 0,
                            "snapshot" => 1,
                            "old_beta" => 2,
                            "old_alpha" => 3,
                            _ => return None,
                        };
                        let annee = v.release_time.get(..4).unwrap_or("").to_string();
                        Some(crate::ui::picker::Entree {
                            id: v.id,
                            kind,
                            annee,
                        })
                    })
                    .collect();
                app.home.borrow_mut().toutes = toutes;
                filtrer_versions(app);
            }
        }
        Err(e) => {
            statut(app, &loc::t("Home_VersionsImpossible"));
            dialogs::erreur(
                app,
                &loc::t("Home_VersionsIndisponibles"),
                &loc::tf("Home_VersionsErreur", &[&e]),
            )
            .await;
        }
    }
}

/// Fenêtre de choix : versions filtrées (types, mod loader compatible, recherche), regroupées par
/// mise à jour ; la mise à jour ouverte montre ses versions à droite.
pub(super) fn filtrer_versions(app: &App) {
    use crate::ui::picker;
    let s = h(app);
    let mut types = [
        s.get_picker_release(),
        s.get_picker_snapshot(),
        s.get_picker_beta(),
        s.get_picker_alpha(),
    ];
    // aucun type coché : les releases reviennent (une liste vide n'aurait rien à proposer)
    if !types.iter().any(|&t| t) {
        s.set_picker_release(true);
        types[0] = true;
    }
    let loader = s.get_picker_loader();
    let recherche = s.get_picker_search().trim().to_lowercase();
    let (groupes, choisi) = {
        let e = app.home.borrow();
        let mut groupes: Vec<(String, Vec<picker::Entree>)> = Vec::new();
        for v in e.toutes.iter().filter(|v| {
            types[v.kind as usize]
                && picker::compatible(&v.id, loader)
                && (recherche.is_empty() || v.id.to_lowercase().contains(&recherche))
        }) {
            let cle = picker::cle_groupe(&v.id, v.kind);
            match groupes.iter_mut().find(|(c, _)| *c == cle) {
                Some((_, liste)) => liste.push(v.clone()),
                None => groupes.push((cle, vec![v.clone()])),
            }
        }
        // mise à jour ouverte : celle choisie, sinon celle de la version actuelle, sinon la première
        let actuelle = picker::cle_groupe(&e.version_choisie, 0);
        let choisi = groupes
            .iter()
            .position(|(c, _)| *c == e.groupe_choisi)
            .or_else(|| groupes.iter().position(|(c, _)| *c == actuelle))
            .unwrap_or(0);
        (groupes, choisi)
    };
    let cartes: Vec<crate::VersionGroup> = groupes
        .iter()
        .map(|(cle, liste)| {
            let (titre, nom) = picker::titre_groupe(cle);
            let annees: Vec<&str> = liste
                .iter()
                .map(|v| v.annee.as_str())
                .filter(|a| !a.is_empty())
                .collect();
            let (min, max) = (
                annees.iter().min().copied().unwrap_or(""),
                annees.iter().max().copied().unwrap_or(""),
            );
            crate::VersionGroup {
                key: cle.clone().into(),
                title: titre.into(),
                name: nom.into(),
                art: picker::illustration(cle),
                count: if liste.len() == 1 {
                    loc::t("Ver_Nombre1")
                } else {
                    loc::tf("Ver_Nombre", &[&liste.len()])
                }
                .into(),
                years: if min == max {
                    min.to_string()
                } else {
                    format!("{min} – {max}")
                }
                .into(),
            }
        })
        .collect();
    let sous: Vec<crate::VersionItem> = groupes
        .get(choisi)
        .map(|(_, liste)| {
            liste
                .iter()
                .map(|v| crate::VersionItem {
                    id: v.id.clone().into(),
                    kind: v.kind,
                })
                .collect()
        })
        .unwrap_or_default();
    {
        let mut e = app.home.borrow_mut();
        e.cles_groupes = groupes.iter().map(|(c, _)| c.clone()).collect();
        if let Some((c, _)) = groupes.get(choisi) {
            e.groupe_choisi = c.clone();
        }
    }
    s.set_picker_groups(modele(cartes));
    s.set_picker_group(if groupes.is_empty() {
        -1
    } else {
        choisi as i32
    });
    s.set_picker_sub(modele(sous));
}

/// Une version choisie dans la grande fenêtre : on passe en jeu rapide sur cette version (même si
/// son type est masqué dans les options).
pub(super) async fn version_choisie_par_id(app: &Rc<App>, id: String) {
    h(app).set_horizon_selected(false);
    misty_quitter(app);
    app.home.borrow_mut().loader_voulu =
        crate::ui::picker::nom_loader(h(app).get_picker_loader()).map(str::to_string);
    aller_a_version(app, id).await;
}

/// Jeu rapide sur la version `id` (le loader voulu est déjà noté).
pub(super) async fn aller_a_version(app: &Rc<App>, id: String) {
    if app.home.borrow().profil.is_some() {
        reglages::modifier(|r| r.dernier_profil = String::new());
        app.home.borrow_mut().profil = None;
        h(app).set_profile_index(0);
        maj_mode_profil(app);
    }
    let index = {
        let mut e = app.home.borrow_mut();
        match e.versions.iter().position(|v| *v == id) {
            Some(i) => i,
            None => {
                e.versions.insert(0, id.clone());
                0
            }
        }
    };
    let versions = app.home.borrow().versions.clone();
    h(app).set_versions(modele_textes(&versions));
    h(app).set_version_index(index as i32);
    version_choisie(app, index as i32).await;
}

pub(super) fn charger_derniere_version(app: &Rc<App>) {
    let derniere = reglages::lire().derniere_version;
    let index = {
        let e = app.home.borrow();
        e.versions
            .iter()
            .position(|v| *v == derniere)
            .or(if e.versions.is_empty() { None } else { Some(0) }) // la plus récente
    };
    if let Some(i) = index {
        h(app).set_version_index(i as i32);
        let a = app.clone();
        lancer(async move { version_choisie(&a, i as i32).await });
    }
}

pub(super) async fn recharger_versions(app: &Rc<App>) {
    let actuelle = app.home.borrow().version_choisie.clone();
    h(app).set_reload_enabled(false);
    if app.home.borrow().profil.is_none() {
        statut(app, &loc::t("Home_ChargementVersions"));
    }
    charger_versions(app).await;

    let index = app
        .home
        .borrow()
        .versions
        .iter()
        .position(|v| *v == actuelle);
    match index {
        Some(i) => h(app).set_version_index(i as i32),
        None => {
            h(app).set_version_index(-1);
            if app.home.borrow().profil.is_none() {
                statut(app, &loc::t("Home_ChoisisVersion"));
            }
        }
    }
    h(app).set_reload_enabled(true);
}

pub(super) async fn version_choisie(app: &Rc<App>, i: i32) {
    let Some(version) = app.home.borrow().versions.get(i.max(0) as usize).cloned() else {
        return;
    };
    {
        let mut e = app.home.borrow_mut();
        e.modes_charges = false;
        e.modes.clear();
        e.version_choisie = version.clone();
    }
    h(app).set_modes_enabled(false);
    h(app).set_modes(modele_textes(&[]));
    h(app).set_mode_index(-1);
    reglages::modifier(|r| r.derniere_version = version.to_string());

    let rapide = app.home.borrow().profil.is_none();
    if rapide {
        h(app).set_play_enabled(false);
        h(app).set_card_version(version.clone().into());
        statut(app, &loc::tf("Commun_RechercheLoaders", &[&version]));
    }

    let v = version.clone();
    let modes = app
        .bg(async move { loaders::modes_disponibles(&v).await })
        .await;

    // L'utilisateur a changé de version pendant le chargement : on laisse la nouvelle sélection gérer
    if app.home.borrow().version_choisie != version {
        return;
    }
    h(app).set_modes(modele_textes(&modes));
    // loader filtré dans la fenêtre de choix : présélectionné s'il existe pour cette version
    let voulu = app.home.borrow_mut().loader_voulu.take();
    let index = voulu
        .and_then(|l| modes.iter().position(|m| *m == l))
        .unwrap_or(0);
    h(app).set_mode_index(index as i32);
    if index > 0 && app.home.borrow().profil.is_none() {
        h(app).set_card_loader(modes[index].to_uppercase().into());
    }
    let installation = app.home.borrow().installation_en_cours;
    h(app).set_modes_enabled(!installation);
    {
        let mut e = app.home.borrow_mut();
        e.modes = modes;
        e.modes_charges = true;
    }
    if app.home.borrow().profil.is_none() {
        maj_mode_profil(app);
    }
}
