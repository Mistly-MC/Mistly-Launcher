//! Accueil : lancement d'une partie (installation, session, progression).

use super::*;

pub(super) async fn jouer(app: &Rc<App>) {
    let misty =
        h(app).get_misty_selected() && app.home.borrow().profil.is_none() && client::disponible();
    let (instance, version, mode) = if misty {
        (
            None,
            h(app).get_misty_mc().to_string(),
            loaders::FABRIC.to_string(),
        )
    } else {
        let e = app.home.borrow();
        let mode_rapide = e
            .modes
            .get(h(app).get_mode_index().max(0) as usize)
            .cloned()
            .unwrap_or_else(|| loaders::VANILLA.into());
        let instance = e.profil.clone();
        let version = instance
            .as_ref()
            .map(|i| i.version_mc.clone())
            .unwrap_or_else(|| e.version_choisie.clone());
        let mode = instance
            .as_ref()
            .map(|i| i.loader.clone())
            .unwrap_or(mode_rapide);
        (instance, version, mode)
    };
    if version.is_empty() || app.home.borrow().installation_en_cours {
        return;
    }

    app.home.borrow_mut().installation_en_cours = true;
    h(app).set_play_enabled(false);
    h(app).set_selectors_enabled(false);
    // un jeton qui tiendra toute la partie (sinon « Invalid session » en rejoignant un serveur)
    if account::session().is_some() {
        app.bg(account::session_pour_jouer()).await;
    }
    let joueur = joueur_actuel(app);
    let nom = match &instance {
        Some(i) => i.nom.clone(),
        None if misty => format!("Mistly {version}"),
        None => format!("{version} {mode}"),
    };
    afficher_progression(app, 0.0, &loc::tf("Home_Preparation", &[&nom]));

    // Téléchargement de la version et du loader (annulable)
    let an = debut_annulable(app);
    let progression = progression_fichiers(app);
    let (v, m, lv) = (
        version.clone(),
        mode.clone(),
        instance.as_ref().and_then(|i| i.loader_version.clone()),
    );
    let resultat = an
        .tache(app, async move {
            let dirs = McDirs::misty();
            let nom_version =
                loaders::installer(&dirs, &v, &m, lv.as_deref(), progression.clone()).await?;
            Ok(install::installer(&dirs, &nom_version, progression).await?)
        })
        .await;
    fin_annulable(app);

    match resultat {
        Ok(installee) => {
            afficher_progression(app, 100.0, &loc::t("Home_Lancement"));
            lancer_jeu(app, &version, &mode, installee, instance, joueur, misty).await;
        }
        Err(e) if est_annule(&e) => {
            misty_core::systeme::journal::info(&format!("Téléchargement annulé : {nom}"));
            statut(app, &loc::t("Home_Annule"));
            h(app).set_progress(0.0);
        }
        Err(e) => {
            statut(app, &loc::t("Home_EchecTelechargement"));
            dialogs::erreur(
                app,
                &loc::t("Home_TelechargementImpossible"),
                &format!("{e:#}"),
            )
            .await;
        }
    }

    app.home.borrow_mut().installation_en_cours = false;
    let s = h(app);
    s.set_play_text(loc::t("Home_Bouton_Jouer").into());
    s.set_play_enabled(true);
    s.set_selectors_enabled(true);
    s.set_modes_enabled(app.home.borrow().modes_charges);
    s.set_percent_text("".into());
}

/// Le compte choisi maintenant : la session Microsoft, sinon le pseudo hors ligne.
pub(super) fn joueur_actuel(app: &App) -> launch::Joueur {
    match account::session() {
        Some(s) => launch::Joueur {
            pseudo: s.pseudo,
            uuid: s.uuid,
            jeton: s.jeton,
            xuid: s.xuid,
            type_compte: "msa".into(),
        },
        None => {
            let pseudo = pseudo_actuel(app);
            launch::Joueur {
                uuid: account::uuid_hors_ligne(&pseudo),
                pseudo,
                jeton: "0".into(),
                xuid: String::new(),
                type_compte: "legacy".into(),
            }
        }
    }
}

pub(super) async fn lancer_jeu(
    app: &Rc<App>,
    version: &str,
    mode: &str,
    installee: install::VersionInstallee,
    instance: Option<Instance>,
    joueur: launch::Joueur,
    misty: bool,
) {
    let pseudo_joueur = joueur.pseudo.clone();
    // Un profil a son propre dossier de jeu (mods, options, mondes) ; le jeu rapide utilise le dossier principal
    let options = launch::Options {
        joueur,
        ram_mo: reglages::lire().ram_mo,
        dossier_jeu: instance
            .as_ref()
            .map(|i| i.dossier())
            .unwrap_or_else(misty_core::systeme::paths::data_dir),
    };

    let titre = match &instance {
        Some(i) => format!("{} · {version} {mode}", i.nom),
        None if misty => format!("Mistly {version} · Mistly Client"),
        None => format!("Minecraft {version} · {mode}"),
    };
    // Misty Client : posé dans les mods du profil s'il le supporte, retiré sinon (toujours posé
    // quand on lance la version Misty)
    let voulu = misty
        || instance
            .as_ref()
            .map(client::voulu)
            .unwrap_or_else(client::active);
    // le jar de cette version, téléchargé depuis GitHub la première fois
    // une nouvelle release du mod a pu sortir depuis le démarrage du launcher
    if voulu && app.bg(client::actualiser_si_ancien()).await {
        maj_misty_client(app);
    }
    let jar = if voulu && client::compatible(version, mode) {
        statut(app, &loc::t("Home_TelechargementClient"));
        let v = version.to_string();
        match app
            .bg(async move { client::telecharger(&v).await })
            .await
            .and_then(|chemin| Ok(std::fs::read(chemin)?))
        {
            Ok(octets) => Some(octets),
            Err(e) => {
                journal::info(&format!("Mistly Client non téléchargé ({e:#}) : la partie démarre sans le mettre à jour"));
                None
            }
        }
    } else {
        None
    };
    client::preparer(&options.dossier_jeu, version, mode, voulu, jar.as_deref());
    client::ecrire_lien();
    // serveurs et barres d'objets : les mêmes dans toutes les versions et tous les profils
    misty_core::contenu::partage::avant_partie(&options.dossier_jeu);
    let dossier_partie = options.dossier_jeu.clone();
    let partie = journal::nouvelle_partie(&titre);
    if crate::ui::console::auto() {
        crate::ui::console::ouvrir(app);
    }
    let resultat = app
        .bloquant(move || launch::lancer(&McDirs::misty(), &installee, &options))
        .await;
    let mut processus = match resultat {
        Ok(p) => p,
        Err(e) => {
            journal::ajouter(partie, &format!("{e:#}"), journal::ERREUR);
            journal::fin(partie, Some(-1));
            statut(app, &loc::t("Home_EchecLancement"));
            dialogs::erreur(app, &loc::t("Home_LancementImpossible"), &format!("{e:#}")).await;
            return;
        }
    };
    let fils = journal::suivre(partie, &mut processus, None);
    telemetry::evenement(
        "game_start",
        serde_json::json!({ "minecraft": version, "loader": mode, "profile": instance.is_some() }),
    );
    let (v_tel, m_tel, pseudo_tel) = (version.to_string(), mode.to_string(), pseudo_joueur.clone());

    let detail = match &instance {
        Some(i) => format!("{mode} · {}", i.nom),
        None => mode.to_string(),
    };
    discord::definir(discord::Statut::EnJeu {
        version: version.to_string(),
        mode: detail,
        pseudo: pseudo_joueur,
    });
    app.home.borrow_mut().jeux_en_cours += 1;
    statut(
        app,
        &loc::tf(
            "Home_Lance",
            &[&instance
                .as_ref()
                .map(|i| i.nom.clone())
                .unwrap_or_else(|| format!("Minecraft {version}"))],
        ),
    );

    if let Some(mut i) = instance {
        i.dernier_lancement = Some(chrono::Local::now());
        let _ = instances::sauver(&i);
    }

    // Fin du jeu : on revient à l'état « prêt »
    let faible = app.ui.as_weak();
    std::thread::spawn(move || {
        let code = processus.wait().ok().and_then(|s| s.code());
        for f in fils {
            let _ = f.join();
        }
        journal::fin(partie, code);
        misty_core::contenu::partage::apres_partie(&dossier_partie);
        let etat = journal::etat(partie).unwrap_or_default();
        // Stop de la console, Alt+F4 : pas un plantage
        let plante = etat.plantee();
        telemetry::evenement(
            "game_end",
            serde_json::json!({ "minecraft": v_tel, "loader": m_tel, "code": code, "seconds": etat.duree.as_secs(), "crashed": plante, "errors": etat.erreurs }),
        );
        if plante {
            telemetry::plantage(partie, &pseudo_tel);
        }
        let _ = faible.upgrade_in_event_loop(|_| {
            if let Some(a) = crate::APP.with(|a| a.borrow().clone()) {
                let reste = {
                    let mut e = a.home.borrow_mut();
                    e.jeux_en_cours = e.jeux_en_cours.saturating_sub(1);
                    e.jeux_en_cours
                };
                // retour au statut de la page affichée (quand plus aucune partie ne tourne)
                if reste == 0 {
                    crate::ui::maj_discord(&a, true);
                }
                statut(&a, &loc::t("Home_Pret"));
                let profil = a.home.borrow().profil.clone();
                if let Some(p) = profil {
                    h(&a).set_summary(resume_contenu(&p).into());
                }
            }
        });
    });
}

// ───────────────────────── Affichage ─────────────────────────

/// Avancement des téléchargements (limité à ~20 mises à jour par seconde).
pub(super) fn progression_fichiers(app: &Rc<App>) -> FileProgress {
    let faible = app.ui.as_weak();
    let derniere = Arc::new(AtomicU64::new(0));
    Arc::new(move |fait: usize, total: usize, nom: &str| {
        if total == 0 {
            return;
        }
        let maintenant = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        if fait < total && maintenant.saturating_sub(derniere.load(Ordering::Relaxed)) < 50 {
            return;
        }
        derniere.store(maintenant, Ordering::Relaxed);
        let pourcentage = fait as f32 * 100.0 / total as f32;
        let texte = loc::tf("Home_Telechargement", &[&nom]);
        let _ = faible.upgrade_in_event_loop(move |ui| {
            if let Some(a) = crate::APP.with(|a| a.borrow().clone()) {
                if a.home
                    .borrow()
                    .annulation
                    .as_ref()
                    .is_some_and(|an| an.annulee())
                {
                    return;
                }
                afficher_progression(&a, pourcentage, &texte);
            } else {
                let _ = ui;
            }
        });
    })
}

pub(super) fn afficher_progression(app: &App, pourcentage: f32, texte: &str) {
    let s = h(app);
    s.set_status(texte.into());
    s.set_percent_text(format!("{pourcentage:.0}%").into());
    s.set_progress(pourcentage / 100.0);
    if app.home.borrow().installation_en_cours {
        s.set_play_text(format!("{pourcentage:.0}%").into());
    }
}
