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
//! Accueil : Horizon (builds, téléchargement, lancement) et choix de Mistly Client.

use super::*;

pub(super) fn taille_mo(octets: u64) -> String {
    format!("{:.0} {}", octets as f64 / 1_048_576.0, loc::t("Unite_Mo"))
}

/// Liste des builds : ceux de GitHub, plus ceux installés qui n'y sont plus (les nightlies ne restent
/// que quelques jours sur GitHub).
pub(super) fn horizon_afficher(app: &App) {
    let installes = misty_core::social::horizon::installes();
    let mut lignes: Vec<crate::HorizonBuild> = app
        .home
        .borrow()
        .horizon_builds
        .iter()
        .map(|b| crate::HorizonBuild {
            tag: b.tag.clone().into(),
            date: b.date.clone().into(),
            size: taille_mo(b.taille).into(),
            nightly: b.nightly,
            installed: installes.contains(&b.tag),
        })
        .collect();
    for tag in &installes {
        if !lignes.iter().any(|l| l.tag == tag.as_str()) {
            lignes.push(crate::HorizonBuild {
                tag: tag.clone().into(),
                date: "".into(),
                size: "".into(),
                nightly: tag.starts_with("nightly"),
                installed: true,
            });
        }
    }
    let s = h(app);
    // build choisi : installé, ou dans la liste de GitHub (il sera téléchargé au lancement) ; la
    // liste n'est pas encore chargée : on le garde
    let choisi = s.get_horizon_build().to_string();
    let charge = !app.home.borrow().horizon_builds.is_empty();
    if choisi.is_empty() || (charge && !lignes.iter().any(|l| l.tag == choisi.as_str())) {
        let nouveau = installes.first().cloned().unwrap_or_default();
        reglages::modifier(|r| r.horizon_build = nouveau.to_string());
        s.set_horizon_build(nouveau.into());
    }
    s.set_horizon_build_installed(installes.contains(&s.get_horizon_build().to_string()));
    // place de chaque build dans la grille selon le filtre (Tous, Nightly, Stables)
    let filtre = s.get_horizon_filter();
    let mut n = 0;
    let rangs: Vec<i32> = lignes
        .iter()
        .map(|l| {
            if filtre == 0 || (filtre == 1) == l.nightly {
                n += 1;
                n - 1
            } else {
                -1
            }
        })
        .collect();
    s.set_horizon_builds(modele(lignes));
    s.set_horizon_ranks(modele(rangs));
    s.set_horizon_visibles(n);
}

pub(super) async fn horizon_rafraichir(app: &Rc<App>) {
    let s = h(app);
    horizon_afficher(app);
    s.set_horizon_loading(true);
    match app
        .bg(misty_core::social::horizon::builds_disponibles())
        .await
    {
        Ok(b) => {
            app.home.borrow_mut().horizon_builds = b;
            s.set_horizon_message("".into());
        }
        Err(_) => s.set_horizon_message(loc::t("Hz_ListeImpossible").into()),
    }
    s.set_horizon_loading(false);
    horizon_afficher(app);
}

/// Télécharge le build choisi s'il n'est pas encore là (au lancement, comme une version de
/// Minecraft : choisir un build ne télécharge rien). L'avancement s'affiche dans la barre du bas et
/// sur la puce du build. Renvoie son dossier.
pub(super) async fn horizon_telecharger(
    app: &Rc<App>,
    an: &Annulation,
    tag: &str,
) -> anyhow::Result<std::path::PathBuf> {
    if let Some(dossier) = misty_core::social::horizon::dossier_build(tag) {
        return Ok(dossier);
    }
    // la liste des builds n'est pas encore chargée (build choisi lors d'un lancement précédent)
    if !app
        .home
        .borrow()
        .horizon_builds
        .iter()
        .any(|b| b.tag == tag)
    {
        horizon_rafraichir(app).await;
    }
    let build = app
        .home
        .borrow()
        .horizon_builds
        .iter()
        .find(|b| b.tag == tag)
        .cloned();
    let build = build.ok_or_else(|| anyhow::anyhow!(loc::tf("Hz_BuildIntrouvable", &[&tag])))?;
    let s = h(app);
    s.set_horizon_busy(true);
    s.set_horizon_busy_tag(tag.into());
    s.set_horizon_progress(0.0);
    afficher_progression(app, 0.0, &loc::tf("Hz_TelechargementBuild", &[&tag]));
    let fenetre = app.ui.as_weak();
    let dernier = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let resultat = an
        .tache(app, async move {
            misty_core::social::horizon::installer_build(&build, move |f| {
                let pourcent = (f * 100.0) as u32;
                if dernier.swap(pourcent, std::sync::atomic::Ordering::SeqCst) != pourcent {
                    let _ = fenetre.upgrade_in_event_loop(move |ui| {
                        let s = ui.global::<HomeState>();
                        s.set_horizon_progress(f);
                        s.set_progress(f);
                        s.set_percent_text(format!("{pourcent}%").into());
                        s.set_play_text(format!("{pourcent}%").into());
                    });
                }
            })
            .await
        })
        .await;
    s.set_horizon_busy(false);
    s.set_horizon_busy_tag("".into());
    horizon_afficher(app);
    resultat?;
    misty_core::social::horizon::dossier_build(tag)
        .ok_or_else(|| anyhow::anyhow!(loc::tf("Hz_BuildIntrouvable", &[&tag])))
}

pub(super) async fn horizon_supprimer(app: &Rc<App>, tag: String) {
    let ok = dialogs::confirmer(
        app,
        &loc::t("Hz_Supprimer"),
        &tag,
        Some(&loc::t("Commun_Supprimer")),
        None,
        true,
    )
    .await;
    if !ok {
        return;
    }
    if let Err(e) = app
        .bloquant(move || misty_core::social::horizon::supprimer_build(&tag))
        .await
    {
        h(app).set_horizon_message(e.to_string().into());
    }
    horizon_afficher(app);
}

/// La liste des jars de Mistly Client (sa release GitHub) est connue ou a changé : le choix de
/// version, les options et l'onglet Skin suivent.
pub(super) fn maj_misty_client(app: &App) {
    let s = h(app);
    s.set_misty_available(client::disponible());
    let versions: Vec<String> = client::versions().into_iter().rev().collect();
    s.set_misty_versions(modele_textes(&versions));
    let misty = version_misty();
    s.set_misty_mc(misty.as_str().into());
    s.set_misty_art(crate::ui::picker::illustration(
        &crate::ui::picker::cle_groupe(&misty, 0),
    ));
    let reglages = app.ui.global::<crate::SettingsState>();
    reglages.set_misty_client_available(client::disponible());
    reglages.set_misty_client_mc(client::versions_texte().into());
    let skin = app.ui.global::<crate::SkinState>();
    skin.set_misty_in_game(client::disponible());
    skin.set_misty_mc(client::versions_texte().into());
}

/// La version Misty choisie avant (gardée dans les réglages), sinon la plus récente.
pub(super) fn version_misty() -> String {
    let choisie = reglages::lire().misty_version;
    if client::existe(&choisie) {
        choisie
    } else {
        client::derniere()
    }
}

/// Cette version Misty devient le jeu lancé par JOUER : jeu rapide sur cette version, en Fabric,
/// avec Misty Client (même si le réglage « Misty Client en jeu » est coupé).
pub(super) fn misty_choisir(app: &Rc<App>, version: String) {
    if !client::existe(&version) {
        return;
    }
    let s = h(app);
    s.set_misty_mc(version.as_str().into());
    s.set_misty_art(crate::ui::picker::illustration(
        &crate::ui::picker::cle_groupe(&version, 0),
    ));
    reglages::modifier(|r| r.misty_version = version.to_string());
    s.set_horizon_selected(false);
    s.set_misty_selected(true);
    s.set_picker_misty(true);
    super::marquer_edition(app, 0);
    reglages::modifier(|r| r.misty_choisi = Some(true));
    let a = app.clone();
    lancer(async move {
        if a.home.borrow().profil.is_some() {
            reglages::modifier(|r| r.dernier_profil = String::new());
            a.home.borrow_mut().profil = None;
            h(&a).set_profile_index(0);
        }
        a.home.borrow_mut().loader_voulu = Some(loaders::FABRIC.to_string());
        aller_a_version(&a, version).await;
        maj_mode_profil(&a);
    });
    maj_mode_profil(app);
}

/// Une autre version, un profil ou Horizon est choisi : JOUER ne lance plus Misty.
pub(super) fn misty_quitter(app: &App) {
    let s = h(app);
    if s.get_misty_selected() {
        s.set_misty_selected(false);
        reglages::modifier(|r| r.misty_choisi = Some(false));
    }
}

/// Horizon devient le jeu lancé par JOUER (barre du bas et carte de l'accueil).
pub(super) fn horizon_choisir(app: &App) {
    let s = h(app);
    misty_quitter(app);
    s.set_horizon_selected(true);
    super::marquer_edition(app, 3);
    s.set_card_label(loc::t("Hz_Etiquette").into());
    s.set_card_version("26.3".into());
    s.set_card_loader("HORIZON".into());
}

pub(super) fn horizon_ouvrir_panneau(app: &App) {
    let s = h(app);
    s.set_picker_tab(0);
    s.set_picker_horizon(true);
    s.set_picker_open(true);
    s.invoke_horizon_refresh();
}

/// Lance le build choisi. La première fois : Minecraft (la version qu'Horizon prépare) et son Java
/// sont installés par Misty, le server.jar est téléchargé, puis horizon-prepare prépare le build.
pub(super) async fn horizon_jouer(app: &Rc<App>) {
    // compte Microsoft : session en ligne (avec un jeton qui tiendra la partie) ; sinon hors ligne,
    // sous le pseudo choisi
    let compte = if account::session().is_some() {
        app.bg(account::session_pour_jouer()).await
    } else {
        None
    };
    let pseudo = compte
        .as_ref()
        .map(|c| c.pseudo.clone())
        .unwrap_or_else(|| pseudo_actuel(app));
    let tag = h(app).get_horizon_build().to_string();
    if tag.is_empty() {
        horizon_ouvrir_panneau(app);
        return;
    }
    if app.home.borrow().installation_en_cours {
        return;
    }
    let s = h(app);
    app.home.borrow_mut().installation_en_cours = true;
    s.set_play_text("0%".into());
    s.set_play_enabled(false);
    s.set_selectors_enabled(false);
    let an = debut_annulable(app);
    let resultat: anyhow::Result<()> = async {
        let build = horizon_telecharger(app, &an, &tag).await?;
        let b = build.clone();
        let exigences = app.bloquant(move || misty_core::social::horizon::exigences(&b)).await?;
        // Minecraft et son Java, installés comme pour n'importe quelle version
        afficher_progression(app, 0.0, &loc::tf("Hz_InstallationMinecraft", &[&exigences.minecraft]));
        let progression = progression_fichiers(app);
        let mc = exigences.minecraft.clone();
        let installee = an.tache(app, async move { Ok(install::installer(&McDirs::misty(), &mc, progression).await?) }).await?;
        let b = build.clone();
        let ex = exigences.clone();
        if !app.bloquant(move || misty_core::social::horizon::est_prepare(&b, &ex)).await {
            let json = installee.json.clone();
            let mc = exigences.minecraft.clone();
            let jar = an.tache(app, async move { misty_core::social::horizon::server_jar(&json, &mc).await }).await?;
            let (b, ex) = (build.clone(), exigences.clone());
            an.tache(app, async move { misty_core::social::horizon::manifeste_epingle(&b, &ex).await }).await?;
            an.verifier()?;
            let java_home = installee.java.parent().and_then(|bin| bin.parent()).map(|p| p.to_path_buf()).unwrap_or_default();
            afficher_progression(app, 0.0, &loc::t("Hz_Preparation"));
            let fenetre = app.ui.as_weak();
            let b = build.clone();
            let ex = exigences.clone();
            app.bloquant(move || {
                misty_core::social::horizon::preparer(&b, &ex, &McDirs::misty().root, &jar, &java_home, move |f, message| {
                    let texte = if message.is_empty() { String::new() } else { message.to_string() };
                    let _ = fenetre.upgrade_in_event_loop(move |_| {
                        if let Some(a) = crate::APP.with(|a| a.borrow().clone()) {
                            let libelle = if texte.is_empty() { loc::t("Hz_Preparation") } else { texte };
                            afficher_progression(&a, f * 100.0, &libelle);
                        }
                    });
                })
            })
            .await?;
            an.verifier()?;
        }
        // la partie : plus rien à annuler
        an.verifier()?;
        fin_annulable(app);
        afficher_progression(app, 100.0, &loc::t("Hz_Lancement"));
        discord::definir(discord::Statut::EnJeu { version: exigences.minecraft.clone(), mode: format!("Horizon {tag}"), pseudo: pseudo.clone() });
        app.home.borrow_mut().jeux_en_cours += 1;
        let session = compte.as_ref().map(|c| misty_core::social::horizon::Session { pseudo: c.pseudo.clone(), uuid: c.uuid.clone(), jeton: c.jeton.clone(), xuid: c.xuid.clone() });
        let b = build.clone();
        let ex = exigences.clone();
        let p = pseudo.clone();
        let partie = journal::nouvelle_partie(&format!("Horizon {tag} · Minecraft {}", exigences.minecraft));
        if crate::ui::console::auto() {
            crate::ui::console::ouvrir(app);
        }
        telemetry::evenement("game_start", serde_json::json!({ "minecraft": exigences.minecraft, "loader": "horizon", "build": tag }));
        // une autre partie peut être lancée pendant que celle-ci tourne
        app.home.borrow_mut().installation_en_cours = false;
        s.set_play_text(loc::t("Home_Bouton_Jouer").into());
        s.set_play_enabled(true);
        s.set_selectors_enabled(true);
        let fin = app.bloquant(move || misty_core::social::horizon::jouer(partie, &b, &ex, session.as_ref(), &p)).await;
        let etat = journal::etat(partie).unwrap_or_default();
        let plante = etat.plantee();
        telemetry::evenement(
            "game_end",
            serde_json::json!({ "minecraft": exigences.minecraft, "loader": "horizon", "build": tag, "code": etat.code, "seconds": etat.duree.as_secs(), "crashed": plante, "stopped": etat.arretee, "errors": etat.erreurs }),
        );
        if plante {
            telemetry::plantage(partie, &pseudo);
        }
        let reste = {
            let mut e = app.home.borrow_mut();
            e.jeux_en_cours = e.jeux_en_cours.saturating_sub(1);
            e.jeux_en_cours
        };
        if reste == 0 {
            crate::ui::maj_discord(app, true);
        }
        fin
    }
    .await;
    fin_annulable(app);
    app.home.borrow_mut().installation_en_cours = false;
    s.set_play_text(loc::t("Home_Bouton_Jouer").into());
    s.set_play_enabled(true);
    s.set_selectors_enabled(true);
    match resultat {
        Ok(()) => afficher_progression(app, 0.0, &loc::t("Hz_Ferme")),
        Err(e) if est_annule(&e) => {
            afficher_progression(app, 0.0, &loc::t("Home_Annule"));
            h(app).set_percent_text("".into());
        }
        Err(e) => {
            afficher_progression(app, 0.0, &loc::t("Hz_Erreur"));
            dialogs::erreur(app, &loc::t("Hz_Erreur"), &format!("{e:#}")).await;
        }
    }
}
