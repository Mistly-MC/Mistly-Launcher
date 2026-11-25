//! Accueil : comptes (Microsoft et hors ligne), pseudo et avatar.

use super::*;

/// Liste des comptes hors ligne de l'accueil, à recharger quand les paramètres en ajoutent ou en suppriment.
pub fn comptes_hors_ligne_changes(app: &Rc<App>) {
    charger_pseudos(app);
}

pub(super) fn charger_pseudos(app: &Rc<App>) {
    let liste = pseudos::lire();
    let dernier = reglages::lire().pseudo;
    let index = liste
        .iter()
        .position(|p| *p == dernier)
        .map(|i| i as i32)
        .unwrap_or(if liste.is_empty() { -1 } else { 0 });
    h(app).set_accounts(modele_textes(&liste));
    h(app).set_account_index(index);
    app.home.borrow_mut().pseudos = liste;
    maj_liste_comptes(app);
    maj_avatar(app);
}

/// La liste unique de l'accueil : comptes Microsoft, puis pseudos hors ligne (un pseudo qui existe
/// aussi en compte Microsoft est marqué « hors ligne » pour les distinguer).
pub(super) fn maj_liste_comptes(app: &Rc<App>) {
    let ms = account::comptes();
    let hors_ligne = app.home.borrow().pseudos.clone();
    let mut noms: Vec<String> = ms.iter().map(|c| c.pseudo.clone()).collect();
    for p in &hors_ligne {
        if ms.iter().any(|c| c.pseudo.eq_ignore_ascii_case(p)) {
            noms.push(format!(
                "{p} ({})",
                loc::t("Home_Compte_HorsLigne").to_lowercase()
            ));
        } else {
            noms.push(p.clone());
        }
    }
    let index = match account::session() {
        Some(s) => ms
            .iter()
            .position(|c| c.uuid == s.uuid)
            .map(|i| i as i32)
            .unwrap_or(-1),
        None if hors_ligne.is_empty() => -1,
        None => ms.len() as i32 + h(app).get_account_index().max(0),
    };
    h(app).set_all_accounts(modele_textes(&noms));
    h(app).set_all_index(index);
}

/// Ajoute un compte Microsoft de plus (fenêtre de connexion), qui devient le compte utilisé.
pub(super) async fn ajouter_compte_microsoft(app: &Rc<App>) {
    h(app).set_ms_busy(true);
    crate::ui::settings::occupe_microsoft(app, true);
    if let Err(e) = app.bg(account::ajouter()).await {
        dialogs::erreur(app, &loc::t("Compte_ConnexionImpossible"), &e.to_string()).await;
    }
    h(app).set_ms_busy(false);
    crate::ui::settings::occupe_microsoft(app, false);
    session_changee(app);
}

pub(super) async fn se_connecter(app: &Rc<App>) {
    h(app).set_ms_busy(true);
    crate::ui::settings::occupe_microsoft(app, true);
    let ancien = h(app).get_status().to_string();
    statut(app, &loc::t("Home_ConnexionMicrosoft"));
    if let Err(e) = app.bg(account::connecter()).await {
        dialogs::erreur(app, &loc::t("Compte_ConnexionImpossible"), &e.to_string()).await;
    }
    h(app).set_ms_busy(false);
    crate::ui::settings::occupe_microsoft(app, false);
    statut(app, &ancien);
    session_changee(app);
}

/// Passe sur un autre compte Microsoft déjà ajouté.
pub async fn changer_compte_microsoft(app: &Rc<App>, uuid: String) {
    h(app).set_ms_busy(true);
    crate::ui::settings::occupe_microsoft(app, true);
    if let Err(e) = app.bg(async move { account::activer(&uuid).await }).await {
        dialogs::erreur(app, &loc::t("Compte_ConnexionImpossible"), &e.to_string()).await;
    }
    h(app).set_ms_busy(false);
    crate::ui::settings::occupe_microsoft(app, false);
    session_changee(app);
}

/// Connexion / déconnexion : met à jour l'accueil et les options.
pub fn session_changee(app: &Rc<App>) {
    maj_compte(app);
    crate::ui::settings::maj_microsoft(app);
    crate::ui::skin::session_changee(app);
}

pub(super) fn maj_compte(app: &Rc<App>) {
    let session = account::session();
    h(app).set_premium(session.is_some());
    let courant = session.as_ref().map(|s| s.uuid.clone()).unwrap_or_default();
    h(app).set_premium_name(session.map(|s| s.pseudo).unwrap_or_default().into());
    let comptes = account::comptes();
    h(app).set_ms_index(
        comptes
            .iter()
            .position(|c| c.uuid == courant)
            .map(|i| i as i32)
            .unwrap_or(-1),
    );
    let noms: Vec<String> = comptes.into_iter().map(|c| c.pseudo).collect();
    h(app).set_ms_names(modele_textes(&noms));
    maj_liste_comptes(app);
    maj_avatar(app);
}

pub fn pseudo_actuel(app: &App) -> String {
    if let Some(s) = account::session() {
        return s.pseudo;
    }
    let i = h(app).get_account_index();
    app.home
        .borrow()
        .pseudos
        .get(i.max(0) as usize)
        .filter(|p| !p.is_empty())
        .cloned()
        .unwrap_or_else(|| "Mistly".into())
}

/// Tête du skin (mc-heads.net). Si le téléchargement échoue, le bloc d'herbe reste visible.
pub(super) fn maj_avatar(app: &Rc<App>) {
    let pseudo = pseudo_actuel(app);
    let a = app.clone();
    let premium = account::session().is_some();
    // petite image du statut Discord : la tête de ce compte
    let uuid = account::session().map(|s| s.uuid);
    discord::definir_joueur(&pseudo, uuid.as_deref());
    lancer(async move {
        // Compte Microsoft : tête tirée du dernier skin connu (disque), sans interroger Mojang.
        // L'onglet Skin la met à jour dès qu'il a reçu le vrai skin ; mc-heads renverrait Steve.
        if premium {
            let uuid = account::session().map(|s| s.uuid).unwrap_or_default();
            let tete = a
                .bg(async move {
                    let p = misty_core::media::skins::profil_en_cache(&uuid)?;
                    if p.skin_url.is_empty() {
                        return None;
                    }
                    let png = misty_core::media::skins::telecharger(&p.skin_url)
                        .await
                        .ok()?;
                    misty_core::media::skins::rendre_tete(&png)
                })
                .await;
            if pseudo_actuel(&a) != pseudo {
                return;
            }
            if let Some(tete) = tete {
                h(&a).set_has_avatar(true);
                h(&a).set_avatar(images::rond(&slint::Image::from_rgba8(images::tampon(
                    &tete,
                ))));
            }
            return;
        }
        let url = format!(
            "https://mc-heads.net/avatar/{}/64",
            urlencoding::encode(&pseudo)
        );
        let image = images::depuis_url(&a, &url).await;
        if pseudo_actuel(&a) != pseudo {
            return; // le compte a changé entre-temps
        }
        h(&a).set_has_avatar(image.is_some());
        h(&a).set_avatar(image.map(|i| images::rond(&i)).unwrap_or_default());
    });
}

/// Option « Demander quel compte utiliser au lancement » : avant la partie, la liste des comptes
/// (Microsoft puis hors ligne). Renvoie false si le joueur annule.
pub(super) async fn choisir_compte_avant_partie(app: &Rc<App>) -> bool {
    if !reglages::lire().demander_compte {
        return true;
    }
    let ms = account::comptes();
    let hors_ligne = app.home.borrow().pseudos.clone();
    if ms.len() + hors_ligne.len() <= 1 {
        return true;
    }
    let mut noms: Vec<String> = ms
        .iter()
        .map(|c| format!("{} · Microsoft", c.pseudo))
        .collect();
    noms.extend(
        hors_ligne
            .iter()
            .map(|p| format!("{p} · {}", loc::t("Home_Compte_HorsLigne").to_lowercase())),
    );
    let Some(i) = dialogs::choisir(
        app,
        &loc::t("Compte_Choisir_Titre"),
        &loc::t("Compte_Choisir_Texte"),
        &noms,
    )
    .await
    else {
        return false;
    };
    if let Some(c) = ms.get(i) {
        if !account::session().is_some_and(|s| s.uuid == c.uuid) {
            changer_compte_microsoft(app, c.uuid.clone()).await;
        }
        return account::session().is_some_and(|s| s.uuid == c.uuid);
    }
    if let Some(p) = hors_ligne.get(i - ms.len()) {
        reglages::modifier(|r| r.pseudo = p.to_string());
        h(app).set_account_index((i - ms.len()) as i32);
        if account::session().is_some() {
            account::deconnecter();
        }
        session_changee(app);
    }
    true
}
