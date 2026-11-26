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
//! Cosmétiques 3D du site (modèles Blockbench, `mistly.cc/models/*.bbmodel`) : lus ici et posés
//! sur le personnage du studio, comme sur la boutique du site. Les fichiers sont copiés dans le
//! launcher (`assets/modeles`), pour s'afficher sans Internet.
//!
//! Repère : Blockbench a y vers le haut, la tête de 24 à 32 et le devant au nord (-z). Le studio a
//! le devant vers +z : un point (x, y, z) de Blockbench est (-x, y, -z) ici (un demi-tour autour
//! de y, comme sur le site).

use super::rendu::{Boite, Region};
use base64::Engine;
use image::RgbaImage;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Partie du corps qui porte le modèle (il suit ses mouvements).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Attache {
    Tete,
    Corps,
}

/// Un cosmétique 3D prêt à dessiner.
#[derive(Clone)]
pub struct Modele3d {
    pub(super) boites: Vec<Boite>,
    pub(super) texture: RgbaImage,
    pub attache: Attache,
}

/// Les modèles connus, par clé (celle du site) : « cloudly », « bunny_ears », « mist_wings ».
pub fn modele(cle: &str) -> Option<&'static Modele3d> {
    static MODELES: OnceLock<HashMap<&'static str, Modele3d>> = OnceLock::new();
    MODELES
        .get_or_init(|| {
            let mut m = HashMap::new();
            for (cle, json, attache) in [
                (
                    "cloudly",
                    include_str!("../../../assets/modeles/cloudly.bbmodel"),
                    Attache::Tete,
                ),
                (
                    "bunny_ears",
                    include_str!("../../../assets/modeles/bunny_ears.bbmodel"),
                    Attache::Tete,
                ),
                (
                    "mist_wings",
                    include_str!("../../../assets/modeles/mist_wings.bbmodel"),
                    Attache::Corps,
                ),
            ] {
                if let Some(modele) = charger(json, attache) {
                    m.insert(cle, modele);
                }
            }
            m
        })
        .get(cle)
}

/// Aperçu d'un modèle seul (pour sa tuile) : vu de trois quarts, recadré sur ses pixels.
pub fn apercu(cle: &str) -> Option<RgbaImage> {
    apercu_de(modele(cle)?)
}

/// Aperçu d'un modèle donné (des ailes teintes, par exemple).
pub fn apercu_de(m: &Modele3d) -> Option<RgbaImage> {
    // un skin transparent : seul le modèle est dessiné
    let vide = RgbaImage::new(64, 64);
    let (yaw, pitch) = match m.attache {
        Attache::Tete => (0.5, 0.25),
        Attache::Corps => (3.4, 0.15),
    };
    let (img, _) = super::studio::rendre_live_modeles(
        &vide,
        None,
        false,
        false,
        super::studio::Anim::Repos,
        0.0,
        yaw,
        pitch,
        &[],
        &[m],
    )?;
    let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
    for (x, y, p) in img.enumerate_pixels() {
        if p[3] > 0 {
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
        }
    }
    if x0 > x1 {
        return None;
    }
    Some(image::imageops::crop_imm(&img, x0, y0, x1 - x0 + 1, y1 - y0 + 1).to_image())
}

pub(super) type Matrice = [[f32; 3]; 3];

pub(super) fn produit(a: &Matrice, b: &Matrice) -> Matrice {
    let mut r = [[0.0; 3]; 3];
    for (i, ligne) in r.iter_mut().enumerate() {
        for (j, case) in ligne.iter_mut().enumerate() {
            *case = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    r
}

pub(super) fn appliquer(m: &Matrice, v: [f32; 3]) -> [f32; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

/// Rotation de Blockbench (degrés) : z, puis y, puis x (Rz × Ry × Rx).
pub(super) fn rotation(r: [f32; 3]) -> Matrice {
    let (sx, cx) = r[0].to_radians().sin_cos();
    let (sy, cy) = r[1].to_radians().sin_cos();
    let (sz, cz) = r[2].to_radians().sin_cos();
    let rx = [[1.0, 0.0, 0.0], [0.0, cx, -sx], [0.0, sx, cx]];
    let ry = [[cy, 0.0, sy], [0.0, 1.0, 0.0], [-sy, 0.0, cy]];
    let rz = [[cz, -sz, 0.0], [sz, cz, 0.0], [0.0, 0.0, 1.0]];
    produit(&rz, &produit(&ry, &rx))
}

/// Transformation affine x → m·x + t.
#[derive(Clone, Copy)]
struct Affine {
    m: Matrice,
    t: [f32; 3],
}

impl Affine {
    fn identite() -> Self {
        Affine {
            m: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            t: [0.0; 3],
        }
    }
    /// Cette transformation, puis une rotation `r` autour de `origine` appliquée avant elle
    /// (celle d'un enfant : groupe ou cube).
    fn puis(&self, origine: [f32; 3], r: [f32; 3]) -> Self {
        let rot = rotation(r);
        let ro = appliquer(&rot, origine);
        let local = [origine[0] - ro[0], origine[1] - ro[1], origine[2] - ro[2]];
        let tl = appliquer(&self.m, local);
        Affine {
            m: produit(&self.m, &rot),
            t: [self.t[0] + tl[0], self.t[1] + tl[1], self.t[2] + tl[2]],
        }
    }
}

fn vec3(v: &Value) -> Option<[f32; 3]> {
    let a = v.as_array()?;
    Some([
        a.first()?.as_f64()? as f32,
        a.get(1)?.as_f64()? as f32,
        a.get(2)?.as_f64()? as f32,
    ])
}

/// Lit un .bbmodel (format « Modded Entity », UV par face). None s'il est illisible.
pub(super) fn charger(json: &str, attache: Attache) -> Option<Modele3d> {
    let m: Value = serde_json::from_str(json).ok()?;
    let tex = m["textures"].as_array()?.first()?;
    let source = tex["source"].as_str()?;
    let donnees = source.split_once(',')?.1;
    let octets = base64::engine::general_purpose::STANDARD
        .decode(donnees)
        .ok()?;
    let texture = image::load_from_memory(&octets).ok()?.to_rgba8();
    // les UV des faces sont dans l'espace de la résolution du projet (« resolution », 128 pour
    // les ailes de brume) ; « uv_width » de la texture ne sert qu'en secours (lire les UV en
    // 64 alors que le projet est en 128 décalait les ailes : seule la membrane s'affichait)
    let uv_l = m["resolution"]["width"]
        .as_f64()
        .or_else(|| tex["uv_width"].as_f64())
        .unwrap_or(64.0) as f32;
    let uv_h = m["resolution"]["height"]
        .as_f64()
        .or_else(|| tex["uv_height"].as_f64())
        .unwrap_or(64.0) as f32;
    let (ex, ey) = (
        texture.width() as f32 / uv_l,
        texture.height() as f32 / uv_h,
    );

    let elements: HashMap<&str, &Value> = m["elements"]
        .as_array()?
        .iter()
        .filter_map(|e| Some((e["uuid"].as_str()?, e)))
        .collect();
    let groupes: HashMap<&str, &Value> = m["groups"]
        .as_array()
        .map(|g| {
            g.iter()
                .filter_map(|g| Some((g["uuid"].as_str()?, g)))
                .collect()
        })
        .unwrap_or_default();

    let mut boites = Vec::new();
    // le parcours de l'arbre (groupes et cubes), avec la transformation de leurs parents
    let mut pile: Vec<(&Value, Affine)> = m["outliner"]
        .as_array()?
        .iter()
        .map(|e| (e, Affine::identite()))
        .collect();
    while let Some((entree, parent)) = pile.pop() {
        if let Some(uuid) = entree.as_str() {
            if let Some(e) = elements.get(uuid) {
                if let Some(b) = boite(e, &parent, ex, ey) {
                    boites.push(b);
                }
            }
            continue;
        }
        let uuid = entree["uuid"].as_str().unwrap_or_default();
        let g = groupes.get(uuid).copied().unwrap_or(entree);
        // la tête et le corps de référence ne servent qu'à modéliser
        if g["name"].as_str() == Some("ref") || g["visibility"].as_bool() == Some(false) {
            continue;
        }
        let origine = vec3(&g["origin"]).unwrap_or([0.0; 3]);
        let r = vec3(&g["rotation"]).unwrap_or([0.0; 3]);
        let t = parent.puis(origine, r);
        if let Some(enfants) = entree["children"].as_array() {
            pile.extend(enfants.iter().map(|c| (c, t)));
        }
    }
    (!boites.is_empty()).then_some(Modele3d {
        boites,
        texture,
        attache,
    })
}

/// Un cube de Blockbench en boîte du studio.
fn boite(e: &Value, parent: &Affine, ex: f32, ey: f32) -> Option<Boite> {
    if e["visibility"].as_bool() == Some(false) {
        return None;
    }
    let de = vec3(&e["from"])?;
    let a = vec3(&e["to"])?;
    let gonfle = e["inflate"].as_f64().unwrap_or(0.0) as f32;
    let origine = vec3(&e["origin"]).unwrap_or([0.0; 3]);
    let r = vec3(&e["rotation"]).unwrap_or([0.0; 3]);
    let t = parent.puis(origine, r);
    // vers le repère du studio : F = diag(-1, 1, -1), monde = (F·M·F)·local + F·t
    let f = |v: [f32; 3]| [-v[0], v[1], -v[2]];
    let mut m = t.m;
    for (i, ligne) in m.iter_mut().enumerate() {
        for (j, case) in ligne.iter_mut().enumerate() {
            let signe = if (i == 1) == (j == 1) { 1.0 } else { -1.0 };
            *case *= signe;
        }
    }
    let min = [
        -a[0].max(de[0]) - gonfle,
        de[1].min(a[1]) - gonfle,
        -a[2].max(de[2]) - gonfle,
    ];
    let mut max = [
        -a[0].min(de[0]) + gonfle,
        de[1].max(a[1]) + gonfle,
        -a[2].min(de[2]) + gonfle,
    ];
    let mut min = min;
    // un plan (épaisseur nulle, comme la membrane des ailes) : le rayon le traverserait sans le
    // toucher ; on lui donne une épaisseur à peine visible
    for i in 0..3 {
        if max[i] - min[i] < 0.02 {
            min[i] -= 0.01;
            max[i] += 0.01;
        }
    }
    // UV par face, dans l'ordre du studio : +z (nord), -z (sud), -x (est), +x (ouest), dessus,
    // dessous ; une face sans texture reste vide (NaN)
    let face = |nom: &str| -> [f32; 4] {
        let f = &e["faces"][nom];
        if f.is_null() || f["texture"].is_null() {
            return [f32::NAN; 4];
        }
        match f["uv"].as_array() {
            Some(uv) if uv.len() == 4 => {
                let v = |i: usize| uv[i].as_f64().unwrap_or(0.0) as f32;
                [v(0) * ex, v(1) * ey, v(2) * ex, v(3) * ey]
            }
            _ => [f32::NAN; 4],
        }
    };
    Some(Boite {
        min,
        max,
        faces: [Region(0, 0, 0, 0); 4],
        miroir: false,
        incline: 0.0,
        roulis: 0.0,
        pivot: [0.0; 3],
        decale: [0.0; 3],
        dessus: None,
        transformation: Some((m, f(t.t))),
        rotation_membre: None,
        uv: Some([
            face("north"),
            face("south"),
            face("east"),
            face("west"),
            face("up"),
            face("down"),
        ]),
    })
}

// ───────────────────────── Couleurs des ailes de brume ─────────────────────────
// Comme sur le site : dans la texture, la membrane occupe les lignes du haut (avant ZONE_OS) et
// les os les suivantes, jusqu'à ZONE_FIN. Chaque pixel prend la teinte voulue à la même
// luminosité, du plus sombre au plus clair de ses 4 tons.

const ZONE_OS: u32 = 32;
const ZONE_FIN: u32 = 96;

/// Préréglages du site : (clé, tons de la membrane, tons des os, part verticale du dégradé).
/// None : la couleur d'origine du modèle (membrane cramoisie, os beiges).
/// Tons d'une partie des ailes (du plus sombre au plus clair), None : la couleur d'origine.
pub type Tons = Option<[u32; 4]>;

pub const TEINTES_AILES: [(&str, Tons, Tons, f32); 7] = [
    (
        "misty",
        Some([0x04050c, 0x1d3a8a, 0x5b8cf0, 0xffffff]),
        Some([0x030305, 0x151824, 0x2e3448, 0x9bb8f0]),
        0.85,
    ),
    ("crimson", None, None, 0.0),
    (
        "amethyst",
        Some([0x1d0833, 0x4a1680, 0x8a3fd0, 0xd2a6ff]),
        Some([0x120a1c, 0x3a2a4a, 0x8a78a0, 0xefe6ff]),
        0.0,
    ),
    (
        "jade",
        Some([0x04241a, 0x0d5c3c, 0x1f9e66, 0x8be8b4]),
        None,
        0.0,
    ),
    (
        "gold",
        Some([0x2e1a04, 0x7a4c0c, 0xd0951f, 0xffe08a]),
        Some([0x1a0f05, 0x5a3a18, 0xb98a4a, 0xfff0c8]),
        0.0,
    ),
    (
        "shadow",
        Some([0x07070b, 0x1a1a24, 0x34344a, 0x7a7a96]),
        Some([0x030305, 0x1e1e26, 0x4a4a5a, 0xc8c8d8]),
        0.0,
    ),
    (
        "snow",
        Some([0x4a5468, 0x9aa6bd, 0xd6deec, 0xffffff]),
        Some([0x2a3346, 0x7a8aa6, 0xc9d6ee, 0xffffff]),
        0.0,
    ),
];

fn rgb(c: u32) -> [f32; 3] {
    [
        ((c >> 16) & 255) as f32,
        ((c >> 8) & 255) as f32,
        (c & 255) as f32,
    ]
}

/// « #3a7bd5 » → 0x3a7bd5
pub fn couleur_hex(texte: &str) -> Option<u32> {
    let t = texte.trim().strip_prefix('#')?;
    (t.len() == 6).then(|| u32::from_str_radix(t, 16).ok())?
}

/// Une couleur libre en 4 tons, de presque noir à presque blanc (comme le site).
fn degrade(c: u32) -> [[f32; 3]; 4] {
    let c = rgb(c);
    let melange = |b: [f32; 3], f: f32| [0, 1, 2].map(|i| c[i] + (b[i] - c[i]) * f);
    [
        melange([0.0; 3], 0.88),
        melange([0.0; 3], 0.55),
        c,
        melange([255.0; 3], 0.7),
    ]
}

/// Les ailes de brume teintes : un préréglage (`TEINTES_AILES`), et éventuellement une couleur
/// libre pour la membrane et pour les os (elles priment sur le préréglage).
pub fn ailes_teintes(preset: &str, membrane: Option<u32>, os: Option<u32>) -> Option<Modele3d> {
    let base = modele("mist_wings")?;
    let (_, p_membrane, p_os, vertical) = TEINTES_AILES
        .iter()
        .find(|t| t.0 == preset)
        .copied()
        .unwrap_or(TEINTES_AILES[0]);
    let tons = |libre: Option<u32>, preset: Option<[u32; 4]>| -> Option<[[f32; 3]; 4]> {
        libre.map(degrade).or_else(|| preset.map(|p| p.map(rgb)))
    };
    let parties = [
        (
            tons(membrane, p_membrane),
            0,
            ZONE_OS,
            if membrane.is_some() { 0.0 } else { vertical },
        ),
        (tons(os, p_os), ZONE_OS, ZONE_FIN, 0.0),
    ];
    let src = &base.texture;
    let mut img = src.clone();
    let l = src.width();
    let lum = |p: &image::Rgba<u8>| 0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32;
    for (tons, debut, fin, vertical) in parties {
        let Some(tons) = tons else { continue };
        let fin = fin.min(src.height());
        // bornes de luminosité, et dernière ligne de chaque colonne (le bord découpé, le plus clair)
        let (mut min, mut max) = (255.0f32, 0.0f32);
        let mut bas = vec![debut; l as usize];
        for y in debut..fin {
            for x in 0..l {
                let p = src.get_pixel(x, y);
                if p[3] > 0 {
                    let v = lum(p);
                    min = min.min(v);
                    max = max.max(v);
                    bas[x as usize] = y;
                }
            }
        }
        for y in debut..fin {
            for x in 0..l {
                let p = *src.get_pixel(x, y);
                if p[3] == 0 {
                    continue;
                }
                let hauteur = (y - debut) as f32 / ((bas[x as usize] - debut).max(1)) as f32;
                let t = ((1.0 - vertical) * (lum(&p) - min) / (max - min).max(1.0)
                    + vertical * hauteur)
                    * 3.0;
                let k = (t.floor() as usize).min(2);
                let f = t - k as f32;
                let c = [0, 1, 2]
                    .map(|i| (tons[k][i] + (tons[k + 1][i] - tons[k][i]) * f).round() as u8);
                img.put_pixel(x, y, image::Rgba([c[0], c[1], c[2], p[3]]));
            }
        }
    }
    let mut teint = base.clone();
    teint.texture = img;
    Some(teint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_modeles_du_site_se_lisent() {
        for cle in ["cloudly", "bunny_ears", "mist_wings"] {
            let m = modele(cle).unwrap_or_else(|| panic!("{cle} illisible"));
            assert!(!m.boites.is_empty(), "{cle} sans cube");
        }
        assert_eq!(modele("cloudly").unwrap().attache, Attache::Tete);
        assert_eq!(modele("mist_wings").unwrap().attache, Attache::Corps);
        // la tête de référence (groupe « ref ») n'est pas dessinée : Cloudly est au-dessus de la tête
        let cloudly = modele("cloudly").unwrap();
        assert!(cloudly.boites.iter().all(|b| b.max[1] > 31.0));
    }

    #[test]
    fn ailes_teintes_comme_sur_le_site() {
        assert_eq!(couleur_hex("#3a7bd5"), Some(0x3a7bd5));
        assert_eq!(couleur_hex("3a7bd5"), None);
        let origine = &modele("mist_wings").unwrap().texture;
        // « crimson » : la couleur d'origine
        assert_eq!(
            &ailes_teintes("crimson", None, None).unwrap().texture,
            origine
        );
        // Mistly, ou une couleur libre : la texture change
        assert_ne!(
            &ailes_teintes("misty", None, None).unwrap().texture,
            origine
        );
        assert_ne!(
            &ailes_teintes("crimson", Some(0x00ff00), None)
                .unwrap()
                .texture,
            origine
        );
    }

    #[test]
    fn rotation_d_un_quart_de_tour() {
        let r = rotation([0.0, 90.0, 0.0]);
        let v = appliquer(&r, [1.0, 0.0, 0.0]);
        assert!((v[0]).abs() < 1e-5 && (v[2] + 1.0).abs() < 1e-5);
    }

    /// Planche du personnage avec ses cosmétiques 3D, sous 4 angles (MISTY_PLANCHE=<dossier>).
    #[test]
    #[ignore]
    fn planche_des_cosmetiques() {
        let Some(dossier) = std::env::var_os("MISTY_PLANCHE") else {
            return;
        };
        let skin = image::load_from_memory(include_bytes!("../../../assets/steve.png"))
            .unwrap()
            .to_rgba8();
        for (nom, cles) in [
            ("chapeaux", vec!["cloudly"]),
            ("lapin", vec!["bunny_ears"]),
            ("ailes", vec!["mist_wings"]),
        ] {
            let modeles: Vec<&Modele3d> = cles.iter().filter_map(|c| modele(c)).collect();
            let mut planche = RgbaImage::new(super::super::LIVE * 4, super::super::LIVE);
            for (k, yaw) in [0.4f32, 1.6, 3.2, 4.7].iter().enumerate() {
                let (img, _) = super::super::rendre_live_modeles(
                    &skin,
                    None,
                    false,
                    false,
                    super::super::Anim::Repos,
                    0.0,
                    *yaw,
                    0.2,
                    &[],
                    &modeles,
                )
                .unwrap();
                image::imageops::overlay(
                    &mut planche,
                    &img,
                    (k as u32 * super::super::LIVE) as i64,
                    0,
                );
            }
            planche
                .save(std::path::Path::new(&dossier).join(format!("{nom}.png")))
                .unwrap();
        }
    }
}
