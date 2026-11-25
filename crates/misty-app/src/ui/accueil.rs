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
//! Connexion Microsoft par code d'appareil (la fenêtre du code) et parcours du premier lancement
//! (langue, licence et conditions, télémétrie), avant le choix Premium / Hors ligne.

use super::{lancer, modele, App};
use crate::*;
use misty_core::{
    auth::account,
    systeme::{loc, reglages, system_info, telemetry},
};
use slint::ComponentHandle;
use std::rc::Rc;

/// Pages de mistly.cc ouvertes par « Lire » à l'étape des conditions.
const LICENCE: &str = "https://mistly.cc/license";
const CONFIDENTIALITE: &str = "https://mistly.cc/privacy";
const CONDITIONS: &str = "https://mistly.cc/terms";

pub fn installer(app: &Rc<App>) {
    // ── Fenêtre du code : misty-core l'ouvre (Some) et la ferme (None), depuis le runtime ──
    let faible = app.ui.as_weak();
    account::definir_afficheur(move |code| {
        let _ = faible.upgrade_in_event_loop(move |ui| {
            let etat = ui.global::<ConnexionState>();
            match code {
                Some(c) => {
                    etat.set_code(c.code.into());
                    etat.set_adresse(c.adresse.into());
                    etat.set_copie(false);
                    etat.set_visible(true);
                }
                None => etat.set_visible(false),
            }
        });
    });
    let etat = app.ui.global::<ConnexionState>();
    let faible = app.ui.as_weak();
    // le code est déjà copié par l'interface (champ invisible) : on ouvre la page de Microsoft
    etat.on_copier_ouvrir(move || {
        if let Some(ui) = faible.upgrade() {
            let adresse = ui.global::<ConnexionState>().get_adresse().to_string();
            if !adresse.is_empty() {
                system_info::ouvrir(&adresse);
            }
        }
    });
    let faible = app.ui.as_weak();
    etat.on_annuler(move || {
        account::annuler_connexion();
        if let Some(ui) = faible.upgrade() {
            ui.global::<ConnexionState>().set_visible(false);
        }
    });
    // captures d'écran de développement : la fenêtre du code, avec un faux code
    if cfg!(debug_assertions) && std::env::var_os("MISTY_CAPTURE_CONNEXION").is_some() {
        etat.set_code("ABCD-1234".into());
        etat.set_adresse("https://www.microsoft.com/link".into());
        etat.set_visible(true);
    }

    // ── Premier lancement ──
    let b = app.ui.global::<BienvenueState>();
    let a = app.clone();
    b.on_langue_choisie(move |i| {
        let code = loc::disponibles().get(i as usize).map(|l| l.code.clone());
        let Some(code) = code else { return };
        a.ui.global::<BienvenueState>().set_langue_index(i);
        if code != loc::code() {
            reglages::modifier(|r| r.langue = code.clone());
            loc::charger(&code);
            super::langue_changee(&a);
        }
    });
    b.on_lire(|quoi| {
        let page = match quoi.as_str() {
            "license" => LICENCE,
            "privacy" => CONFIDENTIALITE,
            _ => CONDITIONS,
        };
        system_info::ouvrir(page);
    });
    let a = app.clone();
    b.on_terminer(move || {
        let b = a.ui.global::<BienvenueState>();
        let langue = loc::code();
        reglages::modifier(|r| {
            r.accueil_fait = true;
            r.langue = langue;
        });
        telemetry::activer(b.get_telemetrie());
        b.set_visible(false);
        // puis la question de toujours : Premium ou Hors ligne
        let a = a.clone();
        lancer(async move { super::home::proposer_mode_compte(&a) });
    });
}

/// Le parcours du premier lancement doit-il s'afficher ? Seulement à une vraie première
/// installation : un joueur qui a déjà choisi sa façon de jouer (versions avant 0.4.1) ne le
/// voit pas.
pub fn necessaire() -> bool {
    let r = reglages::lire();
    !r.accueil_fait && r.mode_compte == reglages::ModeCompte::NonChoisi
}

/// Affiche le parcours (langue : celle déjà choisie, sinon l'anglais).
pub fn afficher(app: &Rc<App>) {
    let langues = loc::disponibles();
    let items: Vec<LangItem> = langues
        .iter()
        .map(|l| LangItem {
            name: l.nom.clone().into(),
            english: super::settings::nom_anglais(&l.code).into(),
            flag: super::settings::drapeau(&l.pays),
            code: l.code.to_uppercase().into(),
        })
        .collect();
    let index = langues
        .iter()
        .position(|l| l.code == loc::code())
        .map(|i| i as i32)
        .unwrap_or(0);
    let b = app.ui.global::<BienvenueState>();
    b.set_langues(modele(items));
    b.set_langue_index(index);
    b.set_etape(0);
    // captures d'écran de développement : ouvrir directement une étape
    if cfg!(debug_assertions) {
        if let Some(n) = std::env::var("MISTY_CAPTURE_ETAPE")
            .ok()
            .and_then(|n| n.parse::<i32>().ok())
        {
            b.set_etape(n);
            b.set_accepte(true);
        }
    }
    b.set_accepte(false);
    b.set_telemetrie(true);
    b.set_visible(true);
}
