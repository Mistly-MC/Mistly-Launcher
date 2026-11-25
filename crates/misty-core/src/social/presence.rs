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
//! Compteur de joueurs en ligne du site (`/api/online`) : tant que Misty est ouvert, un signe de
//! vie part toutes les 60 secondes avec l'identifiant de ce PC et s'il y a une partie en cours.
//!
//! L'identifiant est le même pour tous les launchers ouverts sur le PC (deux launchers ouverts =
//! un seul joueur) : un nombre tiré au hasard une fois, gardé dans le dossier de données, mélangé
//! avec la date du jour. Le site ne voit donc jamais deux jours de suite le même identifiant, et
//! rien ne le relie au compte.

use crate::systeme::contexte::Contexte;
use crate::{systeme::http, systeme::journal};
use std::sync::OnceLock;
use std::time::Duration;

const URL: &str = "https://mistly.cc/api/presence";

/// Nombre tiré au hasard une fois pour ce PC (`presence-id` du dossier de données).
fn graine() -> &'static str {
    static S: OnceLock<String> = OnceLock::new();
    S.get_or_init(|| {
        let fichier = crate::systeme::paths::data_dir().join("presence-id");
        if let Ok(texte) = std::fs::read_to_string(&fichier) {
            let texte = texte.trim().to_string();
            if texte.len() == 32 && texte.chars().all(|c| c.is_ascii_hexdigit()) {
                return texte;
            }
        }
        let nouveau = uuid::Uuid::new_v4().simple().to_string();
        let _ = std::fs::create_dir_all(crate::systeme::paths::data_dir());
        let _ = std::fs::write(&fichier, &nouveau);
        nouveau
    })
}

/// Identifiant de ce PC pour aujourd'hui (le même pour tous ses launchers ouverts).
fn session() -> String {
    identifiant(
        graine(),
        &chrono::Local::now().format("%Y-%m-%d").to_string(),
    )
}

fn identifiant(graine: &str, jour: &str) -> String {
    format!(
        "{:x}",
        md5::compute(format!("mistly-presence:{graine}:{jour}"))
    )
}

async fn envoyer(corps: serde_json::Value) {
    let _ = http::client()
        .post(URL)
        .timeout(Duration::from_secs(10))
        .json(&corps)
        .send()
        .await;
}

/// Lance les signes de vie (sur le runtime des tâches de fond du contexte).
pub fn demarrer(contexte: &Contexte) {
    let Some(rt) = &contexte.runtime else {
        return;
    };
    rt.spawn(async {
        loop {
            let en_jeu = journal::parties().iter().any(|p| p.en_cours);
            envoyer(serde_json::json!({ "session": session(), "playing": en_jeu })).await;
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    });
}

/// Joueurs Mistly en ligne en ce moment, et combien sont en jeu (le compteur du site).
pub async fn en_ligne() -> anyhow::Result<(u64, u64)> {
    let v: serde_json::Value = http::get_json("https://mistly.cc/api/online").await?;
    let en_ligne = v["online"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("/api/online : pas de nombre"))?;
    Ok((en_ligne, v["playing"].as_u64().unwrap_or(0)))
}

/// À la fermeture de Misty : il ne compte plus tout de suite (attend au plus 2 secondes).
pub fn quitter(contexte: &Contexte) {
    let Some(rt) = &contexte.runtime else {
        return;
    };
    let _ = rt.block_on(async {
        tokio::time::timeout(
            Duration::from_secs(2),
            envoyer(serde_json::json!({ "session": session(), "bye": true })),
        )
        .await
    });
}

#[cfg(test)]
mod tests {
    use super::identifiant;

    #[test]
    fn meme_pc_meme_jour_meme_identifiant() {
        let g = "0123456789abcdef0123456789abcdef";
        assert_eq!(identifiant(g, "2026-11-25"), identifiant(g, "2026-11-25"));
        assert_ne!(identifiant(g, "2026-11-25"), identifiant(g, "2026-11-26"));
        assert_eq!(identifiant(g, "2026-11-25").len(), 32);
    }
}
