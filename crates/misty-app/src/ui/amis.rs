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
//! Page Amis : la liste (ajout par pseudo, retrait), les têtes, et qui joue avec Misty Client.

use super::{lancer, modele, page, App};
use crate::{AppState, FriendCard, FriendRequest, FriendsState};
use misty_core::social::amis::{self, Ajout, Ami, Demandes, Etat as EtatReseau};
use misty_core::systeme::loc;
use slint::{ComponentHandle, Image, SharedString};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::Duration;

/// Le site oublie un joueur après 3 minutes sans signe de vie : on redemande chaque minute tant que
/// la page est ouverte.
const RAFRAICHIR: Duration = Duration::from_secs(60);

#[derive(Default)]
pub struct Etat {
    amis: Vec<Ami>,
    /// demandes d'ami du site (vide sans jeton Mistly)
    demandes: Demandes,
    en_ligne: HashSet<String>,
    tetes: HashMap<String, Image>,
    /// têtes déjà demandées (une fois par lancement)
    demandees: HashSet<String>,
    minuterie: Option<slint::Timer>,
    /// efface le message du dernier ajout après quelques secondes
    effacer: slint::Timer,
}

fn s(app: &App) -> FriendsState<'_> {
    app.ui.global::<FriendsState>()
}

pub fn installer(app: &Rc<App>) {
    let st = s(app);
    let a = app.clone();
    st.on_add(move || {
        let a = a.clone();
        lancer(async move { ajouter(&a).await });
    });
    let a = app.clone();
    st.on_remove(move |uuid| {
        amis::retirer(&uuid);
        a.amis.borrow_mut().amis.retain(|x| x.uuid != uuid.as_str());
        afficher(&a);
        // aussi sur le site (sans jeton : rien à faire)
        let u = uuid.to_string();
        let b = a.clone();
        lancer(async move {
            let _ = b.bg(async move { amis::retirer_du_site(&u).await }).await;
        });
    });
    let a = app.clone();
    st.on_accept(move |uuid| repondre(&a, uuid.to_string(), true));
    let a = app.clone();
    st.on_decline(move |uuid| repondre(&a, uuid.to_string(), false));
    let a = app.clone();
    st.on_refresh(move || {
        let a = a.clone();
        lancer(async move { maj_en_ligne(&a).await });
    });

    let faible = Rc::downgrade(app);
    let minuterie = slint::Timer::default();
    minuterie.start(slint::TimerMode::Repeated, RAFRAICHIR, move || {
        let Some(a) = faible.upgrade() else { return };
        if a.ui.global::<AppState>().get_page() == page::AMIS {
            lancer(async move { maj_en_ligne(&a).await });
        }
    });
    app.amis.borrow_mut().minuterie = Some(minuterie);
}

/// La page s'ouvre : la liste du disque, puis qui est en ligne.
pub fn ouvrir(app: &Rc<App>) {
    message(app, "", false);
    {
        let mut etat = app.amis.borrow_mut();
        etat.amis = amis::lire();
        // les têtes qui n'ont pas pu venir sont redemandées
        let tetes: HashSet<String> = etat.tetes.keys().cloned().collect();
        etat.demandees.retain(|u| tetes.contains(u));
    }
    afficher(app);
    let a = app.clone();
    lancer(async move { maj_en_ligne(&a).await });
}

fn afficher(app: &Rc<App>) {
    let etat = app.amis.borrow();
    let mut liste: Vec<&Ami> = etat.amis.iter().collect();
    liste.sort_by_key(|a| (!etat.en_ligne.contains(&a.uuid), a.nom.to_lowercase()));
    let cartes: Vec<FriendCard> = liste
        .iter()
        .map(|a| {
            let tete = etat.tetes.get(&a.uuid);
            FriendCard {
                uuid: a.uuid.as_str().into(),
                name: a.nom.as_str().into(),
                head: tete.cloned().unwrap_or_default(),
                has_head: tete.is_some(),
                online: etat.en_ligne.contains(&a.uuid),
            }
        })
        .collect();
    let a_charger: Vec<String> = etat
        .amis
        .iter()
        .filter(|a| !etat.demandees.contains(&a.uuid))
        .map(|a| a.uuid.clone())
        .collect();
    let nombre = etat
        .amis
        .iter()
        .filter(|a| etat.en_ligne.contains(&a.uuid))
        .count();
    let demandes: Vec<FriendRequest> = etat
        .demandes
        .recues
        .iter()
        .map(|a| (a, true))
        .chain(etat.demandes.envoyees.iter().map(|a| (a, false)))
        .map(|(a, recue)| {
            let tete = etat.tetes.get(&a.uuid);
            FriendRequest {
                uuid: a.uuid.as_str().into(),
                name: if a.nom.is_empty() {
                    a.uuid[..8].into()
                } else {
                    a.nom.as_str().into()
                },
                head: tete.cloned().unwrap_or_default(),
                has_head: tete.is_some(),
                incoming: recue,
            }
        })
        .collect();
    let a_charger: Vec<String> = a_charger
        .into_iter()
        .chain(
            etat.demandes
                .recues
                .iter()
                .chain(&etat.demandes.envoyees)
                .filter(|a| !etat.demandees.contains(&a.uuid))
                .map(|a| a.uuid.clone()),
        )
        .collect();
    drop(etat);
    s(app).set_requests(modele(demandes));
    s(app).set_list(modele(cartes));
    s(app).set_online_count(nombre as i32);
    if !a_charger.is_empty() {
        charger_tetes(app, a_charger);
    }
}

/// Les têtes manquantes, une par une : le service de sessions de Mojang répond 429 si on lui en
/// demande trop d'un coup. Une tête qui n'a pas pu venir est redemandée à la prochaine ouverture.
fn charger_tetes(app: &Rc<App>, uuids: Vec<String>) {
    app.amis
        .borrow_mut()
        .demandees
        .extend(uuids.iter().cloned());
    let a = app.clone();
    lancer(async move {
        for uuid in uuids {
            let u = uuid.clone();
            // un échec reste « demandé » jusqu'à la prochaine ouverture de la page (pas de boucle)
            if let Some(img) = a.bg(async move { amis::tete(&u).await }).await {
                let image = super::images::rond(&Image::from_rgba8(super::images::tampon(&img)));
                a.amis.borrow_mut().tetes.insert(uuid, image);
                afficher(&a);
            }
        }
    });
}

/// Les demandes d'ami du site ; les amis confirmés qui manquent rejoignent la liste locale.
async fn maj_demandes(app: &Rc<App>) {
    match app.bg(async { amis::demandes().await }).await {
        Ok(Some(d)) => {
            let locaux: HashSet<String> = app
                .amis
                .borrow()
                .amis
                .iter()
                .map(|a| a.uuid.clone())
                .collect();
            let mut nouveaux = false;
            for ami in d
                .amis
                .iter()
                .filter(|a| !locaux.contains(&a.uuid) && !a.nom.is_empty())
            {
                amis::ajouter_ami(ami.clone());
                nouveaux = true;
            }
            let mut etat = app.amis.borrow_mut();
            etat.demandes = d;
            if nouveaux {
                etat.amis = amis::lire();
            }
        }
        Ok(None) => app.amis.borrow_mut().demandes = Demandes::default(),
        // site injoignable : on garde ce qu'on savait
        Err(e) => eprintln!("Amis : demandes indisponibles ({e})"),
    }
    afficher(app);
}

fn repondre(app: &Rc<App>, uuid: String, accepter: bool) {
    let ami = {
        let mut etat = app.amis.borrow_mut();
        let d = &mut etat.demandes;
        let ami = d
            .recues
            .iter()
            .chain(&d.envoyees)
            .find(|a| a.uuid == uuid)
            .cloned();
        d.recues.retain(|a| a.uuid != uuid);
        d.envoyees.retain(|a| a.uuid != uuid);
        ami
    };
    let Some(ami) = ami else { return };
    afficher(app);
    let a = app.clone();
    lancer(async move {
        let nom = ami.nom.clone();
        let resultat = a
            .bg(async move {
                if accepter {
                    amis::repondre(&ami, true).await
                } else {
                    // refuser une demande reçue, ou annuler une demande envoyée
                    amis::retirer_du_site(&ami.uuid).await
                }
            })
            .await;
        match resultat {
            Ok(()) if accepter => {
                message(&a, &loc::t("Amis_Ajoute").replace("{0}", &nom), false);
                a.amis.borrow_mut().amis = amis::lire();
                afficher(&a);
                maj_en_ligne(&a).await;
            }
            Ok(()) => {}
            Err(e) => {
                message(&a, &format!("{} ({e})", loc::t("Amis_Erreur")), true);
                maj_demandes(&a).await;
            }
        }
    });
}

async fn maj_en_ligne(app: &Rc<App>) {
    maj_demandes(app).await;
    let liste = app.amis.borrow().amis.clone();
    if liste.is_empty() || s(app).get_refreshing() {
        return;
    }
    s(app).set_refreshing(true);
    let resultat = app.bg(async move { amis::en_ligne(&liste).await }).await;
    s(app).set_refreshing(false);
    match resultat {
        Ok(EtatReseau::Connu(joueurs)) => {
            s(app).set_status_known(true);
            app.amis.borrow_mut().en_ligne = joueurs;
        }
        Ok(EtatReseau::SansJeton) => {
            s(app).set_status_known(false);
            app.amis.borrow_mut().en_ligne.clear();
        }
        // site injoignable : on garde ce qu'on savait
        Err(e) => eprintln!("Amis : statut indisponible ({e})"),
    }
    afficher(app);
}

async fn ajouter(app: &Rc<App>) {
    let st = s(app);
    let pseudo = st.get_name().trim().to_string();
    if pseudo.is_empty() || st.get_adding() {
        return;
    }
    if !amis::pseudo_valide(&pseudo) {
        message(app, &loc::t("Amis_PseudoInvalide"), true);
        return;
    }
    st.set_adding(true);
    message(app, "", false);
    let p = pseudo.clone();
    let resultat = app.bg(async move { amis::ajouter(&p).await }).await;
    st.set_adding(false);
    match resultat {
        Ok(Ajout::Ajoute(ami)) => {
            // une demande d'ami part aussi sur le site (sans jeton Mistly : liste locale seulement)
            let u = ami.uuid.clone();
            let _ = app.bg(async move { amis::demander(&u).await }).await;
            st.set_name(SharedString::new());
            message(app, &loc::t("Amis_Ajoute").replace("{0}", &ami.nom), false);
            app.amis.borrow_mut().amis = amis::lire();
            afficher(app);
            maj_en_ligne(app).await;
        }
        Ok(Ajout::DejaLa) => message(app, &loc::t("Amis_DejaLa").replace("{0}", &pseudo), true),
        Ok(Ajout::Introuvable) => message(
            app,
            &loc::t("Amis_Introuvable").replace("{0}", &pseudo),
            true,
        ),
        Err(e) => message(app, &format!("{} ({e})", loc::t("Amis_Erreur")), true),
    }
}

/// Durée d'affichage du message d'un ajout.
const MESSAGE: Duration = Duration::from_secs(4);

fn message(app: &Rc<App>, texte: &str, erreur: bool) {
    s(app).set_message(texte.into());
    s(app).set_message_error(erreur);
    let etat = app.amis.borrow();
    if texte.is_empty() {
        etat.effacer.stop();
        return;
    }
    let faible = Rc::downgrade(app);
    etat.effacer
        .start(slint::TimerMode::SingleShot, MESSAGE, move || {
            if let Some(a) = faible.upgrade() {
                s(&a).set_message(SharedString::new());
            }
        });
}
