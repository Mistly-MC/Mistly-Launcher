//! Interface : relie les pages Slint (ui/*.slint) aux services.
//!
//! Principe :
//! - toute la logique d'interface tourne sur le thread de l'interface, dans des tâches `slint::spawn_local`
//!   (l'équivalent des `async void` du C#) ;
//! - le travail lourd (réseau, fichiers) part sur le runtime tokio avec `bg(...)`, et on attend le résultat ;
//! - chaque page a son module, avec son état dans un `RefCell`.

pub mod accueil;
pub mod amis;
pub mod browser;
pub mod changelog;
pub mod console;
pub mod dialogs;
pub mod editor;
pub mod home;
pub mod images;
pub mod instance;
pub mod news;
pub mod packs;
pub mod picker;
pub mod profiles;
pub mod scene;
pub mod settings;
pub mod skin;
pub mod studio;

use crate::*;
use misty_core::{
    social::discord, systeme::journal, systeme::loc, systeme::paths, systeme::reglages,
    systeme::system_info, Progress,
};
use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;
use std::sync::Arc;

/// Pages (doivent correspondre à AppState.page dans state.slint).
pub mod page {
    pub const ACCUEIL: i32 = 0;
    pub const PROFILS: i32 = 1;
    pub const OPTIONS: i32 = 2;
    pub const NOTES: i32 = 3;
    pub const PACKS: i32 = 4;
    pub const INSTANCE: i32 = 5;
    pub const EDITEUR: i32 = 6;
    /// Onglet de la barre latérale (pas une page) : l'accueil, sur le studio du skin
    pub const SKIN: i32 = 7;
    /// Amis : la liste et qui joue avec Misty Client
    pub const AMIS: i32 = 8;
    /// Bouton de la barre du haut (pas une page) : les profils, sur l'explorateur de modpacks
    pub const DECOUVRIR: i32 = 9;
}

/// L'application : la fenêtre, le contexte (dont le runtime) et l'état de chaque page.
pub struct App {
    pub ui: MainWindow,
    /// Contexte partagé avec misty-core (dossier de données, client HTTP, runtime)
    pub contexte: std::sync::Arc<misty_core::systeme::contexte::Contexte>,
    /// Runtime tokio du contexte, pour `bg(...)`
    pub rt: tokio::runtime::Handle,
    pub dialog: RefCell<Option<futures::channel::oneshot::Sender<Option<i32>>>>,
    pub images: images::Cache,
    pub home: RefCell<home::Etat>,
    pub profiles: RefCell<profiles::Etat>,
    pub instance: RefCell<instance::Etat>,
    pub browser: RefCell<browser::Etat>,
    pub settings: RefCell<settings::Etat>,
    pub packs: RefCell<packs::Etat>,
    pub amis: RefCell<amis::Etat>,
    pub editor: RefCell<editor::Etat>,
    pub skin: RefCell<skin::Etat>,
    /// Liens des news du carrousel de l'accueil (vide = notes de version du launcher)
    pub news_urls: RefCell<Vec<String>>,
}

impl App {
    pub fn new(
        ui: MainWindow,
        contexte: std::sync::Arc<misty_core::systeme::contexte::Contexte>,
    ) -> Rc<App> {
        let rt = contexte
            .runtime
            .clone()
            .expect("le contexte de l'application a un runtime");
        Rc::new(App {
            ui,
            contexte,
            rt,
            dialog: RefCell::new(None),
            images: images::Cache::default(),
            home: RefCell::default(),
            profiles: RefCell::default(),
            instance: RefCell::default(),
            browser: RefCell::default(),
            settings: RefCell::default(),
            packs: RefCell::default(),
            amis: RefCell::default(),
            editor: RefCell::default(),
            news_urls: RefCell::default(),
            skin: RefCell::default(),
        })
    }

    /// Lance un travail sur le runtime tokio et attend son résultat.
    pub fn bg<T, F>(&self, f: F) -> impl Future<Output = T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let tache = self.rt.spawn(f);
        async move { tache.await.expect("tâche de fond interrompue") }
    }

    /// Lance un calcul bloquant (fichiers, images…) hors du thread de l'interface.
    pub fn bloquant<T, F>(&self, f: F) -> impl Future<Output = T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let tache = self.rt.spawn_blocking(f);
        async move { tache.await.expect("tâche de fond interrompue") }
    }

    /// Fonction de progression qui affiche le texte avec `afficher` (sur le thread de l'interface).
    pub fn progression(
        &self,
        afficher: impl Fn(&MainWindow, String) + Send + Sync + Clone + 'static,
    ) -> Progress {
        let faible = self.ui.as_weak();
        Arc::new(move |texte: String| {
            let afficher = afficher.clone();
            let _ = faible.upgrade_in_event_loop(move |ui| afficher(&ui, texte));
        })
    }
}

/// Lance une tâche asynchrone sur le thread de l'interface.
pub fn lancer(f: impl Future<Output = ()> + 'static) {
    let _ = slint::spawn_local(f);
}

/// Vec<String> → modèle Slint de textes.
pub fn modele_textes(textes: &[String]) -> ModelRc<SharedString> {
    ModelRc::new(VecModel::from(
        textes.iter().map(SharedString::from).collect::<Vec<_>>(),
    ))
}

/// Vec<T> → modèle Slint.
pub fn modele<T: Clone + 'static>(v: Vec<T>) -> ModelRc<T> {
    ModelRc::new(VecModel::from(v))
}

/// Modifie une ligne d'un modèle (s'il a encore cette ligne).
pub fn maj_ligne<T: Clone + 'static>(m: &ModelRc<T>, i: usize, f: impl FnOnce(&mut T)) {
    if let Some(mut ligne) = m.row_data(i) {
        f(&mut ligne);
        m.set_row_data(i, ligne);
    }
}

// ───────────────────────── Démarrage ─────────────────────────

pub fn installer(app: &Rc<App>) {
    let ui = &app.ui;

    // Traductions
    ui.global::<Tr>().on_lookup(|cle| loc::t(&cle).into());
    ui.global::<Assets>().set_logo(images::logo_nuage());
    // version sous la barre latérale : « v0.3.8 » puis « beta.3 » sur sa propre ligne (la barre est étroite)
    ui.global::<AppState>().set_version_text(
        format!("v{}", misty_core::systeme::app_info::VERSION)
            .replacen('-', "\n", 1)
            .into(),
    );

    // Paysage du site en fond (net tout de suite, puis flouté si l'option est réglée)
    afficher_paysage(app, scene::generer(0));
    // l'écran de chargement garde cette scène nette (le fond peut être flouté ensuite)
    {
        let etat = ui.global::<AppState>();
        etat.set_loading_ridges(etat.get_scene_ridges());
        etat.set_loading_stars(etat.get_scene_stars());
        etat.set_loading_twinkle(etat.get_scene_twinkle());
        etat.set_loading_logo(images::logo_chargement());
    }
    let flou = reglages::lire().fond_flou;
    ui.global::<crate::Theme>().set_bg_blur(flou as i32);
    if flou > 0 {
        appliquer_flou_fond(app, flou);
    }

    // Fenêtre
    let a = app.clone();
    ui.global::<AppState>().on_drag(move || {
        fenetre(&a, |w| {
            let _ = w.drag_window();
        })
    });
    let a = app.clone();
    ui.global::<AppState>()
        .on_minimize(move || fenetre(&a, |w| w.set_minimized(true)));
    let a = app.clone();
    ui.global::<AppState>().on_toggle_maximize(move || {
        fenetre(&a, |w| w.set_maximized(!w.is_maximized()));
        suivre_agrandissement(&a);
    });
    // La fenêtre peut aussi être agrandie ou restaurée par Windows (Win+flèches, glisser en haut de l’écran) :
    // on suit son état pour arrondir ou non les coins.
    let a = Rc::downgrade(app);
    let passages = std::cell::Cell::new(0u32);
    let minuterie = slint::Timer::default();
    minuterie.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(300),
        move || {
            if let Some(a) = a.upgrade() {
                suivre_agrandissement(&a);
                suivre_decor(&a);
                maj_discord(&a, false);
                // thème « Comme Windows » : suit Windows quand on le change pendant que Misty est
                // ouvert. Lu dans le registre : toutes les 3 secondes, pas à chaque passage.
                passages.set(passages.get().wrapping_add(1));
                if passages.get().is_multiple_of(10) {
                    let sombre = system_info::windows_sombre();
                    let theme = a.ui.global::<crate::Theme>();
                    if theme.get_system_dark() != sombre {
                        theme.set_system_dark(sombre);
                    }
                }
            }
        },
    );
    std::mem::forget(minuterie);

    // Apparence enregistrée
    let theme = ui.global::<crate::Theme>();
    theme.set_system_dark(system_info::windows_sombre());
    theme.set_mode(reglages::lire().theme);
    theme.set_blur(reglages::lire().flou);
    appliquer_saison(app);
    ui.global::<AppState>().on_close(|| {
        let _ = slint::quit_event_loop();
    });
    ui.global::<AppState>()
        .on_open_data_folder(|| system_info::ouvrir_dossier(&paths::data_dir()));
    ui.global::<AppState>()
        .on_open_discord(|| system_info::ouvrir(misty_core::systeme::app_info::DISCORD_INVITE));
    let a = app.clone();
    ui.global::<AppState>()
        .on_nav_clicked(move |i| nav_clicked(&a, i));

    dialogs::installer(app);
    accueil::installer(app);
    home::installer(app);
    profiles::installer(app);
    instance::installer(app);
    browser::installer(app);
    settings::installer(app);
    changelog::installer(app);
    packs::installer(app);
    amis::installer(app);
    editor::installer(app);
    skin::installer(app);
    studio::installer(app);
    news::installer(app);
    console::installer(app);
}

fn afficher_paysage(app: &App, paysage: scene::Scene) {
    let p = paysage.vers_slint();
    let etat = app.ui.global::<AppState>();
    let n_cretes = p.cretes.len();
    etat.set_scene_ridges(modele(p.cretes));
    etat.set_scene_stars(p.etoiles);
    etat.set_scene_twinkle(p.etoiles_scintillantes);
    etat.set_scene_moon(p.lune);
    etat.set_moon_x(p.lune_x);
    etat.set_moon_y(p.lune_y);
    etat.set_moon_size(p.lune_taille);
    // une liste de nappes par crête : l'interface ne crée que celles qu'elle dessine
    let par_crete: Vec<ModelRc<crate::ScenePuff>> = (0..n_cretes)
        .map(|i| {
            modele(
                p.nappes
                    .iter()
                    .filter(|n| n.layer == i as i32)
                    .cloned()
                    .collect(),
            )
        })
        .collect();
    etat.set_scene_puffs(modele(par_crete));
}

thread_local! {
    /// Dernier niveau de flou demandé : un calcul dépassé (curseur bougé entre-temps) est jeté
    static FLOU_DEMANDE: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// Recalcule le paysage flouté (en arrière-plan) et l'affiche s'il correspond toujours au réglage.
pub fn appliquer_flou_fond(app: &Rc<App>, niveau: u32) {
    FLOU_DEMANDE.with(|f| f.set(niveau));
    let a = app.clone();
    lancer(async move {
        let paysage = a.bloquant(move || scene::generer(niveau)).await;
        if FLOU_DEMANDE.with(|f| f.get()) == niveau {
            afficher_paysage(&a, paysage);
            a.ui.global::<crate::Theme>().set_bg_blur(niveau as i32);
        }
    });
}

/// Thème de saison (Halloween, Noël) si on est à ±3 jours de la fête et que l'option est active.
/// En développement, MISTY_EVENT=1 ou 2 force une saison pour la tester.
pub fn appliquer_saison(app: &App) {
    #[cfg_attr(not(debug_assertions), allow(unused_mut))]
    let mut saison = if reglages::lire().saison {
        reglages::saison_du_jour()
    } else {
        0
    };
    #[cfg(debug_assertions)]
    if let Ok(force) = std::env::var("MISTY_EVENT") {
        saison = force.trim().parse().unwrap_or(saison);
    }
    app.ui.global::<crate::Theme>().set_event(saison);
}

thread_local! {
    /// Dernière page envoyée à Discord (on n'envoie qu'un changement)
    static PAGE_DISCORD: std::cell::Cell<Option<discord::Page>> = const { std::cell::Cell::new(None) };
}

/// Statut Discord de la page affichée (accueil, skin, profils, notes, packs, éditeur, options).
/// Rien n'est envoyé pendant une partie : le statut « en jeu » reste. `force` : renvoyer même sans changement.
pub fn maj_discord(app: &App, force: bool) {
    if app.home.borrow().jeux_en_cours > 0 {
        PAGE_DISCORD.with(|p| p.set(None));
        return;
    }
    let page = if app.ui.global::<SettingsState>().get_open() {
        discord::Page::Options
    } else {
        match app.ui.global::<AppState>().get_page() {
            page::ACCUEIL if app.ui.global::<SkinState>().get_tab() == 1 => discord::Page::Skin,
            page::ACCUEIL => discord::Page::Accueil,
            page::PROFILS | page::INSTANCE => discord::Page::Profils,
            page::NOTES => discord::Page::Notes,
            page::PACKS => discord::Page::Packs,
            page::EDITEUR => discord::Page::Editeur,
            page::AMIS => discord::Page::Amis,
            _ => discord::Page::Accueil,
        }
    };
    if force || PAGE_DISCORD.with(|p| p.get()) != Some(page) {
        PAGE_DISCORD.with(|p| p.set(Some(page)));
        discord::definir(discord::Statut::Launcher(page));
    }
}

/// Met à jour l’indicateur « fenêtre agrandie » (coins carrés, icône du bouton).
fn suivre_agrandissement(app: &App) {
    let mut agrandie = false;
    fenetre(app, |w| agrandie = w.is_maximized());
    let etat = app.ui.global::<AppState>();
    if etat.get_maximized() != agrandie {
        etat.set_maximized(agrandie);
    }
}

/// Le décor (paysage, brume) ne bouge que quand on regarde le launcher : fenêtre au premier plan,
/// pas réduite, et aucune partie en cours. Sinon il s'arrête, et la fenêtre n'est plus redessinée
/// à chaque image (le jeu garde le processeur et la carte graphique). Pas pendant les captures
/// d'écran de développement (fenêtre hors de l'écran, jamais au premier plan).
fn suivre_decor(app: &App) {
    let capture = cfg!(debug_assertions) && std::env::var_os("MISTY_CAPTURE").is_some();
    let mut regarde = true;
    fenetre(app, |w| {
        regarde = w.has_focus() && !w.is_minimized().unwrap_or(false)
    });
    let en_jeu = journal::partie_en_cours();
    let anime = capture || (regarde && !en_jeu);
    let etat = app.ui.global::<AppState>();
    if etat.get_anime() != anime {
        etat.set_anime(anime);
    }
    // Une partie tourne et on ne regarde pas le launcher : il passe en mode éco (le processeur
    // reste au jeu). Il en sort dès qu'on revient dessus, ou quand la partie se termine.
    system_info::mode_eco(en_jeu && !regarde);
    // Une partie vient de démarrer : le launcher rend sa mémoire inutilisée à Windows
    if EN_JEU.with(|j| j.replace(en_jeu)) != en_jeu && en_jeu {
        system_info::liberer_memoire();
    }
}

thread_local! {
    /// Une partie tournait au dernier passage de `suivre_decor`
    static EN_JEU: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Accès à la fenêtre winit (déplacement, réduction).
fn fenetre(app: &App, f: impl FnOnce(&slint::winit_030::winit::window::Window)) {
    use slint::winit_030::WinitWindowAccessor;
    app.ui.window().with_winit_window(f);
}

/// Démarrage (après l'affichage de la fenêtre) : migration, mises à jour, accueil.
pub async fn demarrer(app: Rc<App>) {
    // Ancien dossier C:\TEXT
    match app
        .bloquant(misty_core::systeme::update::migrer_ancien_dossier)
        .await
    {
        Ok(true) => {
            dialogs::info(
                &app,
                &loc::t("Demarrage_Migre"),
                &loc::t("Demarrage_Migre_Texte"),
            )
            .await
        }
        Ok(false) => {}
        Err(e) => {
            dialogs::erreur(
                &app,
                &loc::t("Demarrage_MigrationImpossible"),
                &loc::tf("Demarrage_MigrationImpossible_Texte", &[&e]),
            )
            .await
        }
    }

    reglages::initialiser();
    misty_core::social::pseudos::initialiser();
    news::charger(&app);

    verifier_mise_a_jour(&app, false).await;
    match app.bg(misty_core::systeme::update::finaliser()).await {
        Ok(true) => {
            dialogs::info(
                &app,
                &loc::t("Maj_Terminee"),
                &loc::tf(
                    "Maj_Terminee_Texte",
                    &[&misty_core::systeme::app_info::VERSION],
                ),
            )
            .await
        }
        Ok(false) => {}
        Err(e) => {
            dialogs::erreur(
                &app,
                &loc::t("Maj_Nettoyage"),
                &loc::tf("Maj_Nettoyage_Texte", &[&e]),
            )
            .await
        }
    }
    // lanceur fixe, et raccourcis (épingles de la barre des tâches comprises) redirigés vers lui
    std::thread::spawn(|| {
        misty_core::systeme::lanceur::installer(false);
        misty_core::systeme::lanceur::rediriger_raccourcis();
    });

    home::initialiser(&app).await;
    app.ui.global::<AppState>().set_ready(true);

    // télémétrie : on demande une fois, puis l'interrupteur des options décide
    // (au premier lancement, c'est la dernière étape de l'accueil qui pose la question)
    if !misty_core::systeme::telemetry::demandee()
        && !app.ui.global::<BienvenueState>().get_visible()
    {
        let oui = dialogs::confirmer(
            &app,
            &loc::t("Telemetrie_Question"),
            &loc::t("Telemetrie_Question_Texte"),
            Some(&loc::t("Telemetrie_Accepter")),
            Some(&loc::t("Telemetrie_Refuser")),
            false,
        )
        .await;
        misty_core::systeme::telemetry::activer(oui);
    }
    misty_core::systeme::telemetry::evenement(
        "launcher_start",
        serde_json::json!({ "theme": reglages::lire().theme, "profiles": misty_core::minecraft::instances::lister().len() }),
    );
}

/// `manuel` : lancé depuis les options (on affiche aussi « à jour » et les erreurs).
async fn verifier_mise_a_jour(app: &Rc<App>, manuel: bool) {
    // « Mettre en pause les téléchargements pendant une partie » : pas de téléchargement en fond
    // pendant que le jeu tourne (la vérification manuelle reste possible)
    if !manuel && reglages::lire().pause_telechargements && journal::partie_en_cours() {
        return;
    }
    if manuel {
        app.ui.global::<SettingsState>().set_checking_update(true);
    }
    let resultat = app.bg(misty_core::systeme::update::verifier()).await;
    if manuel {
        app.ui.global::<SettingsState>().set_checking_update(false);
    }
    let release = match resultat {
        Ok(Some(r)) => r,
        Ok(None) => {
            if manuel {
                dialogs::info(
                    app,
                    &loc::t("Maj_AJour"),
                    &loc::tf(
                        "Maj_AJour_Texte",
                        &[&misty_core::systeme::app_info::VERSION],
                    ),
                )
                .await;
            }
            return;
        }
        Err(e) => {
            if manuel {
                dialogs::erreur(app, &loc::t("Maj_VerifImpossible"), &e.to_string()).await;
            }
            return;
        }
    };
    let installer = dialogs::confirmer(
        app,
        &loc::t("Maj_Disponible"),
        &if release.prerelease {
            format!(
                "{}\n\n{}",
                loc::tf(
                    "Maj_Disponible_Texte",
                    &[&release.version, &misty_core::systeme::app_info::VERSION]
                ),
                loc::t("Maj_PreRelease_Texte")
            )
        } else {
            loc::tf(
                "Maj_Disponible_Texte",
                &[&release.version, &misty_core::systeme::app_info::VERSION],
            )
        },
        Some(&loc::t("Maj_Installer")),
        Some(&loc::t("Commun_PlusTard")),
        false,
    )
    .await;
    if !installer {
        return;
    }
    {
        let st = app.ui.global::<AppState>();
        st.set_update_title(loc::tf("Maj_EnCours", &[&release.version]).into());
        st.set_update_status(loc::t("Maj_Connexion").into());
        st.set_update_progress(-1.0);
        st.set_updating(true);
    }
    let faible = app.ui.as_weak();
    // au plus un rafraîchissement par point de pourcentage (le flux arrive par petits morceaux)
    let dernier = std::sync::Arc::new(std::sync::atomic::AtomicI64::new(-2));
    let avancement = move |etape: misty_core::systeme::update::Etape| {
        use misty_core::systeme::update::Etape;
        let (progres, texte) = match etape {
            Etape::Telechargement { recu, total, site } => {
                let mo = |o: u64| format!("{:.1}", o as f64 / 1_048_576.0);
                let source = loc::t(if site {
                    "Maj_Source_Site"
                } else {
                    "Maj_Source_GitHub"
                });
                if total > 0 {
                    (
                        recu as f32 / total as f32,
                        loc::tf("Maj_Telechargement", &[&mo(recu), &mo(total), &source]),
                    )
                } else {
                    (
                        -1.0,
                        loc::tf("Maj_Telechargement_Inconnu", &[&mo(recu), &source]),
                    )
                }
            }
            Etape::Extraction => (1.0, loc::t("Maj_Extraction")),
            Etape::Lancement => (1.0, loc::t("Maj_Lancement")),
        };
        let cle = if progres < 0.0 {
            -1
        } else {
            (progres * 100.0) as i64
        };
        if (0..100).contains(&cle) && dernier.swap(cle, std::sync::atomic::Ordering::Relaxed) == cle
        {
            return;
        }
        let _ = faible.upgrade_in_event_loop(move |ui| {
            let st = ui.global::<AppState>();
            st.set_update_progress(progres);
            st.set_update_status(texte.into());
        });
    };
    let cible = release.version.clone();
    misty_core::systeme::journal::info(&format!("Mise à jour vers {cible}"));
    match app
        .bg(async move { misty_core::systeme::update::installer(&release, avancement).await })
        .await
    {
        Ok(()) => {
            misty_core::systeme::telemetry::evenement(
                "update",
                serde_json::json!({ "to": cible, "ok": true }),
            );
            let _ = slint::quit_event_loop();
        }
        Err(e) => {
            misty_core::systeme::telemetry::evenement(
                "update",
                serde_json::json!({ "to": cible, "ok": false, "error": format!("{e:#}") }),
            );
            app.ui.global::<AppState>().set_updating(false);
            dialogs::erreur(app, &loc::t("Maj_Impossible"), &format!("{e:#}")).await
        }
    }
}

// ───────────────────────── Navigation ─────────────────────────

fn nav_clicked(app: &Rc<App>, i: i32) {
    // Les options s’ouvrent dans une fenêtre par-dessus la page en cours
    if i == page::OPTIONS {
        settings::ouvrir(app);
        return;
    }
    // un onglet (ou l'avatar) pendant que les options, le choix de version ou la console sont
    // ouverts : ils se ferment, et la page demandée s'affiche
    settings::fermer(app);
    app.ui.global::<crate::HomeState>().set_picker_open(false);
    app.ui.global::<crate::ConsoleState>().set_open(false);
    // Skin (barre latérale) : l'accueil, sur le studio ; Jouer : l'accueil, sur la carte de version
    if i == page::SKIN || i == page::ACCUEIL {
        let skin = app.ui.global::<SkinState>();
        let voulu = if i == page::SKIN { 1 } else { 0 };
        if skin.get_tab() != voulu {
            skin.set_tab(voulu);
            if voulu == 1 {
                skin.invoke_open();
            }
        }
        if i == page::SKIN {
            app.ui.global::<AppState>().set_nav(page::SKIN);
            if app.ui.global::<AppState>().get_page() != page::ACCUEIL {
                naviguer(app, page::ACCUEIL);
            }
            app.ui.global::<AppState>().set_nav(page::SKIN);
            maj_discord(app, false);
            return;
        }
    }

    // Découvrir : la page des profils, ouverte sur l'explorateur (modpacks, mods, shaders…)
    if i == page::DECOUVRIR {
        app.ui.global::<AppState>().set_nav(page::DECOUVRIR);
        if app.ui.global::<AppState>().get_page() != page::PROFILS {
            naviguer(app, page::PROFILS);
        }
        app.ui.global::<AppState>().set_nav(page::DECOUVRIR);
        app.ui.global::<crate::ProfilesState>().invoke_modpacks();
        return;
    }

    let etat = app.ui.global::<AppState>();
    let page = etat.get_page();

    // Re-cliquer sur l'onglet actif depuis une sous-page (profil, éditeur) ramène à sa page principale
    if i == etat.get_nav() {
        if i == page::PROFILS && page == page::INSTANCE {
            profiles::rafraichir(app);
            naviguer(app, page::PROFILS);
        } else if i == page::PACKS && page == page::EDITEUR {
            naviguer(app, page::PACKS);
            packs::rafraichir(app);
        } else if i == page::PACKS && app.ui.global::<crate::PacksState>().get_view() != 0 {
            // depuis l'explorateur : retour à la galerie
            app.ui.global::<crate::PacksState>().invoke_back();
        }
        return;
    }

    etat.set_nav(i);
    match i {
        page::ACCUEIL => {
            home::rafraichir(app, false);
        }
        page::OPTIONS => settings::rafraichir(app),
        page::PROFILS => {
            // depuis Découvrir : retour à la liste des profils
            if app.ui.global::<crate::ProfilesState>().get_view() != 0 {
                app.ui.global::<crate::ProfilesState>().invoke_back();
            }
            profiles::rafraichir(app);
        }
        _ => {}
    }
    naviguer(app, i);
    if i == page::AMIS {
        amis::ouvrir(app);
    }

    if i == page::PACKS {
        packs::rafraichir(app); // après naviguer : l'éditeur a pu enregistrer une texture
    }
    maj_discord(app, false);
}

/// Affiche une page (et la barre latérale correspondante).
pub fn naviguer(app: &Rc<App>, p: i32) {
    let etat = app.ui.global::<AppState>();
    if etat.get_page() == p {
        return;
    }
    // On quitte l'éditeur : la texture en cours ne doit pas être perdue
    if etat.get_page() == page::EDITEUR {
        editor::quitter(app);
    }
    etat.set_nav(match p {
        page::INSTANCE => page::PROFILS,
        page::EDITEUR => page::PACKS,
        autre => autre,
    });
    etat.set_page(p);
}

/// Langue changée : recalcule les textes posés depuis Rust.
pub fn langue_changee(app: &Rc<App>) {
    let tr = app.ui.global::<Tr>();
    tr.set_rev(tr.get_rev() + 1);
    discord::reappliquer();
    home::langue_changee(app);
    profiles::rafraichir_liste(app);
    instance::langue_changee(app);
    browser::langue_changee(app);
    settings::maj_textes(app);
    editor::langue_changee(app);
    skin::langue_changee(app);
}
