//! Fenêtre de connexion Microsoft hors Windows (mode `Misty --ms-login`).
//!
//! Sous Windows, la page de Microsoft s'affiche dans une WebView (ms_login.rs). Ailleurs, elle
//! s'ouvre dans le navigateur : une fois connecté, le joueur colle ici l'adresse de la page de fin
//! (`https://login.live.com/oauth20_desktop.srf?code=…`). Le code est écrit sur la sortie standard
//! ("CODE:..."), comme sous Windows, et lu par le processus principal (misty_core::auth::account).
//! Fenêtre fermée sans code : connexion annulée.

use misty_core::{auth::account, systeme::loc, systeme::system_info};
use slint::ComponentHandle;
use std::io::Write;

slint::slint! {
    import { Button, LineEdit, VerticalBox } from "std-widgets.slint";

    export component FenetreConnexion inherits Window {
        in property <string> titre;
        in property <string> texte;
        in property <string> ouvrir;
        in property <string> indice;
        in property <string> valider;
        in property <string> erreur;
        in-out property <string> adresse;
        callback ouvrir-page();
        callback valider-adresse(string);

        title: root.titre;
        preferred-width: 560px;

        VerticalBox {
            Text {
                text: root.texte;
                wrap: word-wrap;
            }
            Button {
                text: root.ouvrir;
                clicked => { root.ouvrir-page(); }
            }
            LineEdit {
                placeholder-text: root.indice;
                text <=> root.adresse;
                accepted(t) => { root.valider-adresse(t); }
            }
            if root.erreur != "": Text {
                text: root.erreur;
                color: #d33;
                wrap: word-wrap;
            }
            Button {
                text: root.valider;
                primary: true;
                clicked => { root.valider-adresse(root.adresse); }
            }
        }
    }
}

fn ecrire(ligne: &str) {
    let mut sortie = std::io::stdout();
    let _ = writeln!(sortie, "{ligne}");
    let _ = sortie.flush();
}

pub fn executer() {
    loc::initialiser();
    let Ok(fenetre) = FenetreConnexion::new() else {
        ecrire("ERREUR:Fenêtre de connexion indisponible");
        return;
    };
    fenetre.set_titre(loc::t("Connexion_Linux_Titre").into());
    fenetre.set_texte(loc::t("Connexion_Linux_Texte").into());
    fenetre.set_ouvrir(loc::t("Connexion_Linux_Ouvrir").into());
    fenetre.set_indice(loc::t("Connexion_Linux_Indice").into());
    fenetre.set_valider(loc::t("Connexion_Linux_Valider").into());
    fenetre
        .window()
        .set_size(slint::LogicalSize::new(560.0, 300.0));

    let ouvrir = || system_info::ouvrir(&account::url_connexion());
    fenetre.on_ouvrir_page(ouvrir);

    let faible = fenetre.as_weak();
    fenetre.on_valider_adresse(move |adresse| {
        let Some(fenetre) = faible.upgrade() else {
            return;
        };
        if let Some(code) = account::parametre_url(&adresse, "code").filter(|c| !c.is_empty()) {
            ecrire(&format!("CODE:{code}"));
            let _ = slint::quit_event_loop();
        } else if let Some(erreur) = account::parametre_url(&adresse, "error_description") {
            ecrire(&format!("ERREUR:{erreur}"));
            let _ = slint::quit_event_loop();
        } else {
            fenetre.set_erreur(loc::t("Connexion_Linux_Invalide").into());
        }
    });

    // la page de connexion s'ouvre tout de suite, le bouton sert à la rouvrir
    ouvrir();
    if fenetre.show().is_ok() {
        let _ = slint::run_event_loop();
    }
}
