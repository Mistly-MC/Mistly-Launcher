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
//! Emotes du site (`mistly.cc/models/emotes.bbmodel`, animations Blockbench au format Bedrock) :
//! saluer, applaudir, danser… jouées sur le personnage du studio, comme sur la boutique.
//! Seules les emotes en solo sont reprises (le câlin à deux et le porté demandent un second
//! joueur).

use super::modele::{appliquer, produit, rotation, Matrice};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Les emotes du studio, dans l'ordre de leurs boutons.
pub const EMOTES: [&str; 6] = ["wave", "clap", "dance", "cheer", "facepalm", "hug"];

/// Une image clé : temps (s), valeur (x, y, z), interpolation.
#[derive(Clone)]
struct Cle {
    t: f32,
    v: [f32; 3],
    lisse: bool,
    palier: bool,
}

#[derive(Default)]
struct Pistes {
    rotation: Vec<Cle>,
    position: Vec<Cle>,
}

struct Emote {
    duree: f32,
    os: HashMap<String, Pistes>,
}

/// Les os du rig de Blockbench et leur pivot (repère de Blockbench).
const OS: [(&str, [f32; 3]); 6] = [
    ("head", [0.0, 24.0, 0.0]),
    ("body", [0.0, 18.0, 0.0]),
    ("right_arm", [5.0, 22.0, 0.0]),
    ("left_arm", [-5.0, 22.0, 0.0]),
    ("right_leg", [1.9, 12.0, 0.0]),
    ("left_leg", [-1.9, 12.0, 0.0]),
];
/// Pivot du joueur entier (« joueur »)
const RACINE: [f32; 3] = [0.0, 16.0, 0.0];

fn emotes() -> &'static HashMap<String, Emote> {
    static E: OnceLock<HashMap<String, Emote>> = OnceLock::new();
    E.get_or_init(|| {
        lire(include_str!("../../../assets/modeles/emotes.bbmodel")).unwrap_or_default()
    })
}

fn lire(json: &str) -> Option<HashMap<String, Emote>> {
    let m: Value = serde_json::from_str(json).ok()?;
    let mut liste = HashMap::new();
    for a in m["animations"].as_array()? {
        let Some(nom) = a["name"].as_str() else {
            continue;
        };
        let mut os: HashMap<String, Pistes> = HashMap::new();
        if let Some(animateurs) = a["animators"].as_object() {
            for an in animateurs.values() {
                let Some(nom_os) = an["name"].as_str() else {
                    continue;
                };
                let Some(cles) = an["keyframes"].as_array() else {
                    continue;
                };
                let pistes = os.entry(nom_os.to_string()).or_default();
                for k in cles {
                    let Some(p) = k["data_points"].as_array().and_then(|d| d.first()) else {
                        continue;
                    };
                    let nombre = |c: &str| -> f32 {
                        match &p[c] {
                            Value::String(s) => s.trim().parse().unwrap_or(0.0),
                            v => v.as_f64().unwrap_or(0.0) as f32,
                        }
                    };
                    let interp = k["interpolation"].as_str().unwrap_or("linear");
                    let cle = Cle {
                        t: k["time"].as_f64().unwrap_or(0.0) as f32,
                        v: [nombre("x"), nombre("y"), nombre("z")],
                        lisse: interp == "catmullrom",
                        palier: interp == "step",
                    };
                    match k["channel"].as_str() {
                        Some("rotation") => pistes.rotation.push(cle),
                        Some("position") => pistes.position.push(cle),
                        _ => {}
                    }
                }
                pistes.rotation.sort_by(|a, b| a.t.total_cmp(&b.t));
                pistes.position.sort_by(|a, b| a.t.total_cmp(&b.t));
            }
        }
        liste.insert(
            nom.to_string(),
            Emote {
                duree: a["length"].as_f64().unwrap_or(1.0).max(0.05) as f32,
                os,
            },
        );
    }
    Some(liste)
}

fn catmull(p0: f32, p1: f32, p2: f32, p3: f32, u: f32) -> f32 {
    0.5 * (2.0 * p1
        + (p2 - p0) * u
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * u * u
        + (3.0 * p1 - p0 - 3.0 * p2 + p3) * u * u * u)
}

/// Valeur d'une piste à l'instant t (linéaire, palier ou Catmull-Rom, comme dans Blockbench).
fn valeur(cles: &[Cle], t: f32) -> [f32; 3] {
    let n = cles.len();
    if n == 0 {
        return [0.0; 3];
    }
    if t <= cles[0].t {
        return cles[0].v;
    }
    if t >= cles[n - 1].t {
        return cles[n - 1].v;
    }
    let i = cles.iter().rposition(|c| c.t <= t).unwrap_or(0).min(n - 2);
    let (a, b) = (&cles[i], &cles[i + 1]);
    let u = (t - a.t) / (b.t - a.t).max(1e-6);
    if a.palier {
        return a.v;
    }
    if a.lisse || b.lisse {
        let p0 = cles.get(i.wrapping_sub(1)).unwrap_or(a).v;
        let p3 = cles.get(i + 2).unwrap_or(b).v;
        return [0, 1, 2].map(|c| catmull(p0[c], a.v[c], b.v[c], p3[c], u));
    }
    [0, 1, 2].map(|c| a.v[c] + (b.v[c] - a.v[c]) * u)
}

/// Repère de Blockbench → studio : un demi-tour autour de y (x et z changent de signe).
fn f(v: [f32; 3]) -> [f32; 3] {
    [-v[0], v[1], -v[2]]
}
fn f_matrice(m: &Matrice) -> Matrice {
    let mut r = *m;
    for (i, ligne) in r.iter_mut().enumerate() {
        for (j, case) in ligne.iter_mut().enumerate() {
            if (i == 1) != (j == 1) {
                *case = -*case;
            }
        }
    }
    r
}

/// Pose d'un os pendant une emote, dans le repère du studio : rotation autour du pivot, puis
/// déplacement (monde = rotation × (x − pivot) + pivot + décalage).
pub(super) struct PoseOs {
    pub rotation: Matrice,
    pub pivot: [f32; 3],
    pub decale: [f32; 3],
}

/// Les 6 os (tête, corps, bras droit, bras gauche, jambe droite, jambe gauche) de l'emote `k`
/// (voir `EMOTES`) à l'instant `t` (secondes, l'emote boucle). None : emote inconnue.
pub(super) fn poser(k: usize, t: f32) -> Option<[PoseOs; 6]> {
    let emote = emotes().get(*EMOTES.get(k)?)?;
    let t = t % emote.duree;
    let racine = emote.os.get("joueur");
    let r_racine = racine
        .map(|p| rotation(valeur(&p.rotation, t)))
        .unwrap_or(rotation([0.0; 3]));
    let p_racine = racine.map(|p| valeur(&p.position, t)).unwrap_or([0.0; 3]);
    let poses = OS.map(|(nom, pivot)| {
        let pistes = emote.os.get(nom);
        let r_os = pistes
            .map(|p| rotation(valeur(&p.rotation, t)))
            .unwrap_or(rotation([0.0; 3]));
        let p_os = pistes.map(|p| valeur(&p.position, t)).unwrap_or([0.0; 3]);
        // Blockbench : monde = Rr·(Ro·(x − po) + po + déplacement de l'os − pr) + pr + déplacement
        // du joueur
        let total = produit(&r_racine, &r_os);
        let dedans = [
            pivot[0] + p_os[0] - RACINE[0],
            pivot[1] + p_os[1] - RACINE[1],
            pivot[2] + p_os[2] - RACINE[2],
        ];
        let tourne = appliquer(&r_racine, dedans);
        let decale = [
            tourne[0] + RACINE[0] + p_racine[0] - pivot[0],
            tourne[1] + RACINE[1] + p_racine[1] - pivot[1],
            tourne[2] + RACINE[2] + p_racine[2] - pivot[2],
        ];
        PoseOs {
            rotation: f_matrice(&total),
            pivot: f(pivot),
            decale: f(decale),
        }
    });
    Some(poses)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_emotes_du_site_se_lisent() {
        for (k, nom) in EMOTES.iter().enumerate() {
            assert!(emotes().contains_key(*nom), "{nom} absente");
            assert!(poser(k, 0.7).is_some());
        }
        assert!(poser(EMOTES.len(), 0.0).is_none());
    }

    #[test]
    fn interpolation_lineaire_et_palier() {
        let cles = vec![
            Cle {
                t: 0.0,
                v: [0.0; 3],
                lisse: false,
                palier: false,
            },
            Cle {
                t: 1.0,
                v: [10.0, 0.0, 0.0],
                lisse: false,
                palier: false,
            },
        ];
        assert_eq!(valeur(&cles, 0.5)[0], 5.0);
        let palier = vec![
            Cle {
                t: 0.0,
                v: [1.0; 3],
                lisse: false,
                palier: true,
            },
            Cle {
                t: 1.0,
                v: [9.0; 3],
                lisse: false,
                palier: false,
            },
        ];
        assert_eq!(valeur(&palier, 0.5)[0], 1.0);
    }

    /// Planche des emotes, de face (MISTY_PLANCHE=<dossier>).
    #[test]
    #[ignore]
    fn planche_des_emotes() {
        let Some(dossier) = std::env::var_os("MISTY_PLANCHE") else {
            return;
        };
        let skin = image::load_from_memory(include_bytes!("../../../assets/steve.png"))
            .unwrap()
            .to_rgba8();
        let l = super::super::LIVE;
        let mut planche = image::RgbaImage::new(l * EMOTES.len() as u32, l * 2);
        for k in 0..EMOTES.len() {
            for (rang, (t, yaw)) in [(0.8f32, 0.35f32), (1.3, 1.2)].iter().enumerate() {
                let (img, _) = super::super::rendre_live_parties(
                    &skin,
                    None,
                    false,
                    false,
                    super::super::Anim::Emote(k as u8),
                    *t,
                    *yaw,
                    0.1,
                    &[],
                )
                .unwrap();
                image::imageops::overlay(
                    &mut planche,
                    &img,
                    (k as u32 * l) as i64,
                    (rang as u32 * l) as i64,
                );
            }
        }
        planche
            .save(std::path::Path::new(&dossier).join("emotes.png"))
            .unwrap();
    }
}
