//! Page d'accueil : choix du compte, du profil (ou jeu rapide : version + mod loader) et lancement du jeu.

use super::{dialogs, images, lancer, modele, modele_textes, App};
use crate::{AppState, HomeState};
use misty_core::minecraft::instances::{self, Instance};
use misty_core::minecraft::{install, launch, loaders, FileProgress, McDirs};
use misty_core::{
    auth::account, contenu::content, minecraft::client, social::discord, social::pseudos,
    systeme::journal, systeme::loc, systeme::reglages, systeme::telemetry,
};
use slint::ComponentHandle;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

// Sous-modules de l'accueil : comptes, versions (jeu rapide), lancement, Horizon et Mistly Client
mod compte;
mod horizon;
mod lancement;
mod versions;
pub use compte::*;
use horizon::*;
use lancement::*;
use versions::*;

/// Erreur « téléchargement annulé par le joueur » (pas affichée comme une erreur).
#[derive(Debug)]
struct Annule;

impl std::fmt::Display for Annule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&loc::t("Home_Annule"))
    }
}

impl std::error::Error for Annule {}

fn est_annule(e: &anyhow::Error) -> bool {
    e.chain().any(|c| c.downcast_ref::<Annule>().is_some())
}

/// Téléchargement qu'on peut annuler : chaque tâche de fond lancée par `tache` s'arrête à la
/// demande (les fichiers à moitié téléchargés restent en .part et seront refaits).
#[derive(Clone, Default)]
struct Annulation(Rc<std::cell::RefCell<(Vec<tokio::task::AbortHandle>, bool)>>);

impl Annulation {
    fn annuler(&self) {
        let mut a = self.0.borrow_mut();
        a.1 = true;
        for t in a.0.drain(..) {
            t.abort();
        }
    }

    fn annulee(&self) -> bool {
        self.0.borrow().1
    }

    /// Erreur Annule si le joueur a annulé depuis l'étape précédente.
    fn verifier(&self) -> anyhow::Result<()> {
        if self.annulee() {
            Err(Annule.into())
        } else {
            Ok(())
        }
    }

    /// Lance une tâche de fond annulable et attend son résultat.
    async fn tache<T: Send + 'static>(
        &self,
        app: &App,
        f: impl std::future::Future<Output = anyhow::Result<T>> + Send + 'static,
    ) -> anyhow::Result<T> {
        self.verifier()?;
        let h = app.rt.spawn(f);
        self.0.borrow_mut().0.push(h.abort_handle());
        match h.await {
            Ok(r) => r,
            Err(e) if e.is_cancelled() => Err(Annule.into()),
            Err(e) => Err(anyhow::anyhow!("{e}")),
        }
    }
}

#[derive(Default)]
pub struct Etat {
    /// Téléchargement en cours (annulable par le bouton Annuler)
    annulation: Option<Annulation>,
    version_choisie: String,
    modes_charges: bool,
    installation_en_cours: bool,
    /// Parties lancées depuis Misty qui tournent encore (plusieurs à la fois possible).
    pub jeux_en_cours: u32,
    profil: Option<Instance>,
    /// Profils de la liste déroulante (None = jeu rapide).
    profils: Vec<Option<Instance>>,
    versions: Vec<String>,
    modes: Vec<String>,
    pseudos: Vec<String>,
    /// Toutes les versions de Mojang (type, année), pour la fenêtre de choix
    toutes: Vec<super::picker::Entree>,
    /// Builds d'Horizon publiés sur GitHub
    horizon_builds: Vec<misty_core::social::horizon::Build>,
    /// Mises à jour affichées dans la fenêtre de choix, et celle qui est ouverte
    cles_groupes: Vec<String>,
    groupe_choisi: String,
    /// Mod loader à présélectionner quand ses modes seront chargés (filtre de la fenêtre de choix)
    loader_voulu: Option<String>,
}

fn h(app: &App) -> HomeState<'_> {
    app.ui.global::<HomeState>()
}

/// Début d'un téléchargement annulable.
fn debut_annulable(app: &App) -> Annulation {
    let an = Annulation::default();
    app.home.borrow_mut().annulation = Some(an.clone());
    h(app).set_can_cancel(true);
    an
}

/// Fin du téléchargement (réussi, raté ou annulé) : plus rien à annuler.
fn fin_annulable(app: &App) {
    app.home.borrow_mut().annulation = None;
    h(app).set_can_cancel(false);
}

pub fn installer(app: &Rc<App>) {
    let s = h(app);
    s.set_play_text(loc::t("Home_Bouton_Jouer").into());
    let a = app.clone();
    s.on_cancel_install(move || {
        let an = a.home.borrow().annulation.clone();
        if let Some(an) = an {
            an.annuler();
            fin_annulable(&a);
            statut(&a, &loc::t("Home_Annulation"));
        }
    });
    s.set_status(loc::t("Home_Chargement").into());

    let a = app.clone();
    s.on_account_selected(move |i| {
        let pseudo = a.home.borrow().pseudos.get(i as usize).cloned();
        if let Some(p) = pseudo {
            reglages::modifier(|r| r.pseudo = p.to_string());
        }
        maj_avatar(&a);
    });
    // Premier lancement : Premium (compte Microsoft) ou Hors ligne (un pseudo)
    let a = app.clone();
    a.ui.global::<AppState>().on_choose_premium({
        let a = a.clone();
        move || {
            reglages::modifier(|r| r.mode_compte = reglages::ModeCompte::Premium);
            a.ui.global::<AppState>().set_account_choice(false);
            if account::session().is_none() {
                let a = a.clone();
                lancer(async move { se_connecter(&a).await });
            }
        }
    });
    let a = app.clone();
    a.ui.global::<AppState>().on_choose_offline({
        let a = a.clone();
        move |pseudo| {
            let pseudo = nettoyer_pseudo(&pseudo);
            if pseudo.is_empty() {
                return;
            }
            pseudos::ajouter(&pseudo);
            reglages::modifier(|r| r.pseudo = pseudo.to_string());
            reglages::modifier(|r| r.mode_compte = reglages::ModeCompte::HorsLigne);
            charger_pseudos(&a);
            a.ui.global::<AppState>().set_account_choice(false);
        }
    });
    let a = app.clone();
    s.on_ms_clicked(move || {
        let a = a.clone();
        lancer(async move {
            if account::comptes().is_empty() {
                se_connecter(&a).await;
            } else {
                ajouter_compte_microsoft(&a).await;
            }
        });
    });
    // liste unique : un compte Microsoft (on s'y connecte) ou un pseudo hors ligne
    let a = app.clone();
    s.on_all_selected(move |i| {
        let ms = account::comptes();
        let i = i.max(0) as usize;
        if let Some(c) = ms.get(i) {
            if account::session().is_some_and(|s| s.uuid == c.uuid) {
                return;
            }
            let (a, uuid) = (a.clone(), c.uuid.clone());
            lancer(async move { changer_compte_microsoft(&a, uuid).await });
            return;
        }
        let pseudo = a.home.borrow().pseudos.get(i - ms.len()).cloned();
        if let Some(p) = pseudo {
            reglages::modifier(|r| r.pseudo = p.to_string());
            h(&a).set_account_index((i - ms.len()) as i32);
            // hors ligne : la session Microsoft se ferme (le compte reste enregistré)
            if account::session().is_some() {
                account::deconnecter();
            }
            session_changee(&a);
        }
    });
    // Liste des comptes Microsoft de l'accueil : on change de compte
    let a = app.clone();
    s.on_ms_selected(move |i| {
        let uuid = account::comptes().get(i as usize).map(|c| c.uuid.clone());
        if let Some(uuid) = uuid {
            let a = a.clone();
            lancer(async move { changer_compte_microsoft(&a, uuid).await });
        }
    });
    let a = app.clone();
    s.on_profile_selected(move |i| {
        h(&a).set_horizon_selected(false);
        misty_quitter(&a);
        let profil = a.home.borrow().profils.get(i as usize).cloned().flatten();
        reglages::modifier(|r| {
            r.dernier_profil = profil
                .as_ref()
                .map(|p| p.id.as_str())
                .unwrap_or("")
                .to_string()
        });
        a.home.borrow_mut().profil = profil;
        maj_mode_profil(&a);
    });
    // Section Misty Client : Fabric + Misty Client, sur une des versions de Minecraft du mod
    maj_misty_client(app);
    let a = app.clone();
    s.on_misty_select(move |v| misty_choisir(&a, v.to_string()));
    let a = app.clone();
    s.on_misty_play(move || {
        let v = h(&a).get_misty_mc().to_string();
        misty_choisir(&a, v);
        h(&a).set_picker_open(false);
        let a = a.clone();
        lancer(async move { jouer(&a).await });
    });
    // Horizon : builds GitHub, lancés par Misty
    s.set_horizon_art(super::picker::illustration("horizon"));
    s.set_horizon_build(reglages::lire().horizon_build.into());
    let a = app.clone();
    s.on_horizon_refresh(move || {
        let a = a.clone();
        lancer(async move { horizon_rafraichir(&a).await });
    });
    let a = app.clone();
    s.on_horizon_filter_changed(move || horizon_afficher(&a));
    let a = app.clone();
    s.on_horizon_remove(move |tag| {
        let a = a.clone();
        lancer(async move { horizon_supprimer(&a, tag.to_string()).await });
    });
    let a = app.clone();
    s.on_horizon_select(move |tag| {
        reglages::modifier(|r| r.horizon_build = tag.to_string());
        h(&a).set_horizon_build(tag);
        horizon_choisir(&a);
        horizon_afficher(&a);
    });
    let a = app.clone();
    s.on_horizon_play(move || {
        horizon_choisir(&a);
        h(&a).set_picker_open(false);
        let a = a.clone();
        lancer(async move { horizon_jouer(&a).await });
    });
    s.on_horizon_link(|i| {
        misty_core::systeme::system_info::ouvrir(if i == 0 {
            misty_core::social::horizon::SITE
        } else {
            misty_core::social::horizon::GITHUB
        })
    });

    // Fenêtre de choix de version
    let a = app.clone();
    s.on_picker_changed(move || filtrer_versions(&a));
    let a = app.clone();
    s.on_picker_group_selected(move |i| {
        let cle = a.home.borrow().cles_groupes.get(i as usize).cloned();
        if let Some(cle) = cle {
            a.home.borrow_mut().groupe_choisi = cle;
            filtrer_versions(&a);
        }
    });
    let a = app.clone();
    s.on_version_picked(move |id| {
        let a = a.clone();
        lancer(async move { version_choisie_par_id(&a, id.to_string()).await });
    });
    let a = app.clone();
    s.on_version_selected(move |i| {
        let a = a.clone();
        lancer(async move { version_choisie(&a, i).await });
    });
    let a = app.clone();
    s.on_mode_selected(move |i| {
        if a.home.borrow().profil.is_none() {
            let mode = a
                .home
                .borrow()
                .modes
                .get(i as usize)
                .cloned()
                .unwrap_or_else(|| loaders::VANILLA.into());
            // Misty, c'est Fabric : un autre loader revient à une version normale
            if mode != loaders::FABRIC {
                misty_quitter(&a);
            }
            h(&a).set_card_loader(mode.to_uppercase().into());
        }
    });
    let a = app.clone();
    s.on_reload(move || {
        let a = a.clone();
        lancer(async move { recharger_versions(&a).await });
    });
    let a = app.clone();
    s.on_manage(move || {
        let profil = a.home.borrow().profil.clone();
        if let Some(p) = profil {
            a.ui.global::<AppState>().set_nav(super::page::PROFILS);
            super::profiles::rafraichir(&a);
            super::instance::ouvrir(&a, p);
        }
    });
    let a = app.clone();
    s.on_play(move || {
        if h(&a).get_horizon_selected() {
            let a = a.clone();
            lancer(async move { horizon_jouer(&a).await });
            return;
        }
        let a = a.clone();
        lancer(async move { jouer(&a).await });
    });
}

fn statut(app: &App, texte: &str) {
    h(app).set_status(texte.into());
}

// ───────────────────────── Initialisation ─────────────────────────

/// Appelé une fois au démarrage, après la préparation des fichiers de config.
/// Pseudo Minecraft valide : lettres, chiffres et _ seulement, 16 caractères au plus.
fn nettoyer_pseudo(texte: &str) -> String {
    texte
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .take(16)
        .collect()
}

/// Au démarrage, tant qu'aucune façon de jouer n'a été choisie : l'écran Premium | Hors ligne.
/// Un compte Microsoft déjà connecté vaut le choix « premium » (pas de question).
fn proposer_mode_compte(app: &Rc<App>) {
    if reglages::lire().mode_compte != reglages::ModeCompte::NonChoisi {
        return;
    }
    if account::session().is_some() || reglages::lire().premium {
        reglages::modifier(|r| r.mode_compte = reglages::ModeCompte::Premium);
        return;
    }
    // captures d'écran de développement : seulement si on le demande (il cacherait les pages)
    let capture = cfg!(debug_assertions) && std::env::var_os("MISTY_CAPTURE").is_some();
    if capture && std::env::var_os("MISTY_CAPTURE_CHOIX").is_none() {
        return;
    }
    app.ui.global::<AppState>().set_account_choice(true);
}

/// Joueurs Mistly en ligne (le compteur du site) : lu au démarrage puis chaque minute.
fn suivre_en_ligne(app: &Rc<App>) {
    let a = Rc::downgrade(app);
    let lire = move || {
        let Some(a) = a.upgrade() else { return };
        lancer(async move {
            if let Ok((en_ligne, en_jeu)) = a.bg(misty_core::social::presence::en_ligne()).await {
                h(&a).set_online_count(en_ligne.min(i32::MAX as u64) as i32);
                h(&a).set_online_playing(en_jeu.min(i32::MAX as u64) as i32);
            }
        });
    };
    lire();
    let minuterie = slint::Timer::default();
    minuterie.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_secs(60),
        lire,
    );
    std::mem::forget(minuterie);
}

pub async fn initialiser(app: &Rc<App>) {
    suivre_en_ligne(app);
    charger_pseudos(app);
    maj_compte(app);
    charger_profils(app);
    proposer_mode_compte(app);

    // Connexion automatique en parallèle du chargement des versions
    // (pas pendant les captures d'écran de développement : la fenêtre de connexion s'ouvrirait)
    let capture = cfg!(debug_assertions) && std::env::var_os("MISTY_CAPTURE").is_some();
    if !capture && reglages::lire().premium && account::session().is_none() {
        let a = app.clone();
        lancer(async move { se_connecter(&a).await });
    }

    // Mistly Client : la liste des jars de sa dernière release GitHub, lue pendant le chargement
    // des versions (au plus 10 s : sans Internet, la liste gardée sert)
    let maj_client = app.rt.spawn(async {
        tokio::time::timeout(std::time::Duration::from_secs(10), client::actualiser()).await
    });

    statut(app, &loc::t("Home_ChargementVersions"));
    charger_versions(app).await;
    match maj_client.await {
        Ok(Ok(Ok(true))) => maj_misty_client(app),
        Ok(Ok(Err(e))) => journal::info(&format!("Mistly Client : liste des jars non lue ({e:#})")),
        _ => {}
    }
    // tout premier démarrage (rien de choisi avant) : Misty Client, sur sa version la plus récente
    let premier =
        reglages::lire().derniere_version.is_empty() && reglages::lire().misty_choisi.is_none();
    if (premier || reglages::lire().misty_choisi.unwrap_or(false))
        && client::disponible()
        && app.home.borrow().profil.is_none()
    {
        misty_choisir(app, version_misty());
    } else {
        charger_derniere_version(app);
    }
    if app.home.borrow().profil.is_some() {
        // un profil n'attend pas la liste des versions : « Chargement des versions… » ne doit pas
        // rester affiché (seul le jeu rapide remet le statut à jour une fois ses loaders trouvés)
        maj_mode_profil(app);
    } else if h(app).get_version_index() < 0 {
        statut(app, &loc::t("Home_ChoisisVersion"));
    }
}

/// Appelé quand on revient sur la page (les options ont pu changer).
pub fn rafraichir(app: &Rc<App>, recharger_versions_aussi: bool) {
    charger_pseudos(app);
    charger_profils(app);
    if recharger_versions_aussi {
        let a = app.clone();
        lancer(async move { recharger_versions(&a).await });
    }
}

pub fn langue_changee(app: &Rc<App>) {
    maj_compte(app);
    charger_profils(app); // "Jeu rapide" + résumé du profil
    if !app.home.borrow().installation_en_cours {
        h(app).set_play_text(loc::t("Home_Bouton_Jouer").into());
    }
}

// ───────────────────────── Profils ─────────────────────────

fn charger_profils(app: &Rc<App>) {
    let id_voulu = app
        .home
        .borrow()
        .profil
        .as_ref()
        .map(|p| p.id.clone())
        .unwrap_or_else(|| reglages::lire().dernier_profil);

    let mut profils: Vec<Option<Instance>> = vec![None];
    profils.extend(instances::lister().into_iter().map(Some));
    let noms: Vec<String> = profils
        .iter()
        .map(|p| {
            p.as_ref()
                .map(|i| i.nom.clone())
                .unwrap_or_else(|| loc::t("Home_JeuRapide"))
        })
        .collect();
    let index = profils
        .iter()
        .position(|p| p.as_ref().is_some_and(|i| i.id == id_voulu))
        .unwrap_or(0);

    h(app).set_profiles(modele_textes(&noms));
    h(app).set_profile_index(index as i32);
    // cartes de la fenêtre de choix
    let cartes: Vec<crate::ProfileItem> = profils
        .iter()
        .zip(&noms)
        .map(|(p, nom)| {
            let icone = p
                .as_ref()
                .and_then(|i| std::fs::read(i.chemin_icone()).ok())
                .and_then(|o| super::images::decoder(&o, 96))
                .map(slint::Image::from_rgba8);
            crate::ProfileItem {
                name: nom.clone().into(),
                detail: match p {
                    Some(i) => format!("{} · {}", i.version_mc, i.loader),
                    None => loc::t("Home_JeuRapide_Detail"),
                }
                .into(),
                has_icon: icone.is_some(),
                icon: icone.unwrap_or_default(),
                art: super::picker::illustration(&super::picker::cle_groupe(
                    &p.as_ref()
                        .map(|i| i.version_mc.clone())
                        .unwrap_or_else(|| reglages::lire().derniere_version),
                    0,
                )),
            }
        })
        .collect();
    h(app).set_profile_items(modele(cartes));
    // icônes distantes (modpacks sans icon.png) : chargées ensuite, sans bloquer la liste
    for (i, p) in profils.iter().enumerate() {
        let Some(instance) = p else { continue };
        let Some(url) = instance
            .icone_url
            .clone()
            .filter(|_| !instance.chemin_icone().exists())
        else {
            continue;
        };
        let (a, nom) = (app.clone(), instance.nom.clone());
        lancer(async move {
            use slint::Model;
            let Some(image) = images::depuis_url(&a, &url).await else {
                return;
            };
            let liste = h(&a).get_profile_items();
            // la liste a pu changer entre-temps : on vérifie que c'est toujours ce profil
            if let Some(mut item) = liste.row_data(i).filter(|item| item.name.as_str() == nom) {
                item.icon = image;
                item.has_icon = true;
                liste.set_row_data(i, item);
            }
        });
    }
    {
        let mut e = app.home.borrow_mut();
        e.profil = profils[index].clone();
        e.profils = profils;
    }
    maj_mode_profil(app);
}

/// Bascule l'affichage entre le jeu rapide et le profil sélectionné.
fn maj_mode_profil(app: &Rc<App>) {
    let (profil, version, modes_charges, installation, jeu) = {
        let e = app.home.borrow();
        (
            e.profil.clone(),
            e.version_choisie.clone(),
            e.modes_charges,
            e.installation_en_cours,
            e.jeux_en_cours > 0,
        )
    };
    let s = h(app);
    s.set_quick(profil.is_none());

    match &profil {
        Some(p) => {
            s.set_card_label(
                format!("{} · {}", loc::t("Home_Profil"), p.nom.to_uppercase()).into(),
            );
            s.set_card_version(p.version_mc.clone().into());
            s.set_card_loader(p.loader.to_uppercase().into());
            s.set_summary(resume_contenu(p).into());
            // L'icône du profil remplace le bloc d'herbe
            s.set_has_card_icon(false);
            let (fichier, url) = (p.chemin_icone(), p.icone_url.clone());
            let id = p.id.clone();
            let a = app.clone();
            lancer(async move {
                let image = match images::depuis_fichier(&fichier, 256) {
                    Some(i) => Some(i),
                    None => match url {
                        Some(u) => images::depuis_url(&a, &u).await,
                        None => None,
                    },
                };
                if a.home.borrow().profil.as_ref().map(|p| p.id.as_str()) == Some(id.as_str()) {
                    if let Some(i) = image {
                        h(&a).set_card_icon(i);
                        h(&a).set_has_card_icon(true);
                    }
                }
            });
        }
        None if s.get_misty_selected() => {
            s.set_card_label("MISTY CLIENT".into());
            s.set_card_version(s.get_misty_mc());
            s.set_card_loader("MISTY".into());
            s.set_has_card_icon(false);
        }
        None => {
            s.set_card_label(loc::t("Home_Version_Selected").into());
            s.set_card_version(if version.is_empty() {
                "—".into()
            } else {
                version.into()
            });
            let mode = app
                .home
                .borrow()
                .modes
                .get(s.get_mode_index().max(0) as usize)
                .cloned()
                .unwrap_or_else(|| loaders::VANILLA.into());
            s.set_card_loader(mode.to_uppercase().into());
            s.set_has_card_icon(false);
        }
    }

    if !installation {
        let pret = profil.is_some() || modes_charges;
        s.set_play_enabled(pret);
        if pret {
            statut(app, &loc::t(if jeu { "Home_EnCours" } else { "Home_Pret" }));
        }
    }
}

/// "12 mods · 2 shaders"
fn resume_contenu(instance: &Instance) -> String {
    let mut compte: Vec<(String, usize)> = Vec::new();
    for c in content::lister(instance).into_iter().filter(|c| c.actif()) {
        match compte.iter_mut().find(|(t, _)| *t == c.type_) {
            Some((_, n)) => *n += 1,
            None => compte.push((c.type_.clone(), 1)),
        }
    }
    if compte.is_empty() {
        return loc::t("Home_AucunContenu");
    }
    compte
        .iter()
        .map(|(t, n)| format!("{n} {}", content::libelle_nombre(t, *n)))
        .collect::<Vec<_>>()
        .join(" · ")
}

/// Sélectionne un profil et lance la partie (depuis la page Profils).
pub fn selectionner_et_jouer(app: &Rc<App>, instance: Instance) {
    reglages::modifier(|r| r.dernier_profil = instance.id.to_string());
    app.home.borrow_mut().profil = Some(instance);
    charger_profils(app);
    if !app.home.borrow().installation_en_cours {
        let a = app.clone();
        lancer(async move { jouer(&a).await });
    }
}
