//! Skins : aperçu en temps réel du studio (poses, animations, rendu sur plusieurs fils).

use super::*;

// ───────────────────────── Aperçu en temps réel ─────────────────────────
// Le studio de l'onglet Skin dessine le personnage à chaque image (et non plus des vues
// pré-calculées) : rotation et inclinaison de la caméra continues, animations, cape qui bouge.

/// Animations du studio (mêmes que la boutique du site).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Anim {
    Repos,
    Marche,
    Course,
    Vol,
    Accroupi,
}

impl Anim {
    pub fn depuis(i: i32) -> Anim {
        match i {
            1 => Anim::Marche,
            2 => Anim::Course,
            3 => Anim::Vol,
            4 => Anim::Accroupi,
            _ => Anim::Repos,
        }
    }
}

/// Taille de l'image du rendu en direct (carrée : en vol, le personnage est à l'horizontale).
pub const LIVE: u32 = 320;

/// Orientation d'une partie du corps : bascule avant/arrière (x) et écartement (z) autour d'un
/// pivot, puis déplacement.
#[derive(Clone, Copy, Default)]
pub(super) struct Membre {
    incline: f32,
    roulis: f32,
    pivot: [f32; 3],
    decale: [f32; 3],
}

/// Pose complète à l'instant `t` (secondes).
pub(super) struct Pose {
    tete: Membre,
    corps: Membre,
    bras_d: Membre,
    bras_g: Membre,
    jambe_d: Membre,
    jambe_g: Membre,
    /// inclinaison de la cape (elle flotte vers l'arrière)
    cape: f32,
    /// élytres : inclinaison et écartement
    ailes: (f32, f32),
    /// tout le personnage basculé (vol, course), autour de la taille
    global: f32,
}

pub(super) fn pose(anim: Anim, t: f32, b: f32) -> Pose {
    use std::f32::consts::TAU;
    let m = |pivot: [f32; 3]| Membre {
        pivot,
        ..Default::default()
    };
    let mut p = Pose {
        tete: m([0.0, 24.0, 0.0]),
        corps: m([0.0, 24.0, 0.0]),
        bras_d: m([-4.0 - b / 2.0, 22.0, 0.0]),
        bras_g: m([4.0 + b / 2.0, 22.0, 0.0]),
        jambe_d: m([-2.0, 12.0, 0.0]),
        jambe_g: m([2.0, 12.0, 0.0]),
        cape: CAPE_INCLINAISON,
        ailes: (0.26, 0.38),
        global: 0.0,
    };
    // respiration : les bras s'écartent à peine, la cape ondule doucement
    let souffle = (t * TAU / 3.2).sin();
    p.bras_d.roulis = -0.03 - 0.025 * souffle;
    p.bras_g.roulis = 0.03 + 0.025 * souffle;
    p.cape += 0.05 * (t * TAU / 2.7).sin();
    match anim {
        Anim::Repos => {}
        Anim::Marche | Anim::Course => {
            let (amp, freq) = if anim == Anim::Course {
                (0.95, 1.55)
            } else {
                (0.55, 0.95)
            };
            let s = (t * TAU * freq).sin();
            p.bras_d.incline = amp * s;
            p.bras_g.incline = -amp * s;
            p.jambe_d.incline = -amp * s;
            p.jambe_g.incline = amp * s;
            // la cape se soulève avec la vitesse et bat au rythme des pas
            let vitesse = if anim == Anim::Course { 0.85 } else { 0.4 };
            p.cape = CAPE_INCLINAISON + vitesse + 0.08 * (t * TAU * freq * 2.0).sin();
        }
        Anim::Vol => {
            // à l'horizontale, bras le long du corps, jambes serrées, ailes grandes ouvertes
            p.global = 1.25;
            p.bras_d.incline = -0.25;
            p.bras_g.incline = -0.25;
            p.bras_d.roulis = -0.12;
            p.bras_g.roulis = 0.12;
            p.tete.incline = -0.9;
            p.cape = 1.2 + 0.1 * (t * TAU * 1.8).sin();
            p.ailes = (0.45, 0.7 + 0.08 * (t * TAU * 0.9).sin());
        }
        Anim::Accroupi => {
            // k : 0 debout, 1 accroupi ; on reste un peu dans chaque position (cycle de 2,4 s)
            let k = ((t * TAU / 2.4).sin() * 0.7 + 0.5).clamp(0.0, 1.0);
            let k = k * k * (3.0 - 2.0 * k);
            p.corps.incline = 0.5 * k;
            p.bras_d.incline = 0.4 * k;
            p.bras_g.incline = 0.4 * k;
            for haut in [&mut p.tete, &mut p.corps, &mut p.bras_d, &mut p.bras_g] {
                haut.decale = [0.0, -2.6 * k, 0.0];
            }
            p.jambe_d.decale = [0.0, 0.0, -3.6 * k];
            p.jambe_g.decale = [0.0, 0.0, -3.6 * k];
            p.cape += 0.5 * k;
            // ailes plus écartées accroupi, comme dans le jeu
            p.ailes = (0.26 + 0.04 * k, 0.38 + 0.52 * k);
        }
    }
    p
}

/// Une boîte prête à être touchée par les rayons : la caméra est ramenée dans son repère une fois
/// par image (et non à chaque pixel).
pub(super) struct BoitePosee<'a> {
    boite: &'a Boite,
    texture: &'a RgbaImage,
    origine: [f32; 3],
    droite: [f32; 3],
    haut: [f32; 3],
    regard: [f32; 3],
}

pub(super) fn tourner_inverse(v: [f32; 3], incline: f32, roulis: f32) -> [f32; 3] {
    let (sx, cx) = incline.sin_cos();
    let (sz, cz) = roulis.sin_cos();
    let (x, y, z) = (v[0] * cz + v[1] * sz, -v[0] * sz + v[1] * cz, v[2]);
    [x, y * cx + z * sx, -y * sx + z * cx]
}

/// Partie du corps visible en un pixel du rendu en direct (`rendre_live_parties`).
#[allow(dead_code)] // les bras et les jambes ne mènent à aucun emplacement (pour l'instant)
pub mod partie {
    pub const TETE: u8 = 0;
    pub const CORPS: u8 = 1;
    pub const BRAS_D: u8 = 2;
    pub const BRAS_G: u8 = 3;
    pub const JAMBE_D: u8 = 4;
    pub const JAMBE_G: u8 = 5;
    pub const CAPE: u8 = 6;
    pub const ELYTRES: u8 = 7;
    /// rien (le fond)
    pub const AUCUNE: u8 = 255;
}

/// Le personnage dans sa pose, vu sous l'angle `yaw` (autour de lui) et `pitch` (de haut ou de bas),
/// en radians. `t` : temps de l'animation, en secondes. Avec l'image, la partie du corps visible en
/// chaque pixel (une valeur de `partie` par pixel, ligne par ligne) : ce qui dit sur quoi on clique.
/// `surbrillance` : parties teintées de bleu (celles que la souris survole dans le studio).
#[allow(clippy::too_many_arguments)]
/// Fils du rendu en direct : créés au premier dessin, puis réutilisés (8 au plus).
pub(super) fn fils_du_rendu() -> &'static rayon::ThreadPool {
    static FILS: std::sync::OnceLock<rayon::ThreadPool> = std::sync::OnceLock::new();
    FILS.get_or_init(|| {
        let n = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
            .clamp(1, 8);
        rayon::ThreadPoolBuilder::new()
            .num_threads(n)
            .thread_name(|i| format!("studio-{i}"))
            .build()
            .expect("fils du rendu du studio")
    })
}

// Les paramètres d'une image du rendu (skin, pose, caméra) : 7 appels, une structure n'apporterait rien
#[allow(clippy::too_many_arguments)]
pub fn rendre_live_parties(
    skin: &RgbaImage,
    cape: Option<&RgbaImage>,
    fin: bool,
    elytres: bool,
    anim: Anim,
    t: f32,
    yaw: f32,
    pitch: f32,
    surbrillance: &[u8],
) -> Option<(RgbaImage, Vec<u8>)> {
    rendre_live_modeles(
        skin,
        cape,
        fin,
        elytres,
        anim,
        t,
        yaw,
        pitch,
        surbrillance,
        &[],
    )
}

/// Comme `rendre_live_parties`, avec des cosmétiques 3D du site (chapeaux, ailes) portés par le
/// personnage.
#[allow(clippy::too_many_arguments)]
pub fn rendre_live_modeles(
    skin: &RgbaImage,
    cape: Option<&RgbaImage>,
    fin: bool,
    elytres: bool,
    anim: Anim,
    t: f32,
    yaw: f32,
    pitch: f32,
    surbrillance: &[u8],
    modeles: &[&Modele3d],
) -> Option<(RgbaImage, Vec<u8>)> {
    if skin.width() != 64 || skin.height() < 32 {
        return None;
    }
    let cape_img: Option<RgbaImage> = match (elytres, cape) {
        (true, Some(c)) if c.width() != 64 || c.height() != 32 => {
            Some(imageops::resize(c, 64, 32, imageops::FilterType::Nearest))
        }
        (true, Some(c)) => Some(c.clone()),
        (true, None) => Some(elytres_par_defaut()),
        (false, c) => c.cloned(),
    };
    let moderne = skin.height() >= 64;
    let bras: u32 = if fin { 3 } else { 4 };
    let b = bras as f32;
    let p = pose(anim, t, b);

    // (boîte, partie du corps) ; 0 tête, 1 corps, 2 bras d, 3 bras g, 4 jambe d, 5 jambe g
    let mut boites: Vec<(Boite, usize)> = vec![
        (
            boite_skin((0, 0), (8, 8, 8), [-4.0, 24.0, -4.0], 0.0, false),
            0,
        ),
        (
            boite_skin((16, 16), (8, 12, 4), [-4.0, 12.0, -2.0], 0.0, false),
            1,
        ),
        (
            boite_skin((40, 16), (bras, 12, 4), [-4.0 - b, 12.0, -2.0], 0.0, false),
            2,
        ),
        (
            boite_skin((0, 16), (4, 12, 4), [-4.0, 0.0, -2.0], 0.0, false),
            4,
        ),
    ];
    if moderne {
        boites.push((
            boite_skin((32, 48), (bras, 12, 4), [4.0, 12.0, -2.0], 0.0, false),
            3,
        ));
        boites.push((
            boite_skin((16, 48), (4, 12, 4), [0.0, 0.0, -2.0], 0.0, false),
            5,
        ));
        boites.push((
            boite_skin((32, 0), (8, 8, 8), [-4.0, 24.0, -4.0], 0.5, false),
            0,
        ));
        boites.push((
            boite_skin((16, 32), (8, 12, 4), [-4.0, 12.0, -2.0], 0.25, false),
            1,
        ));
        boites.push((
            boite_skin((40, 32), (bras, 12, 4), [-4.0 - b, 12.0, -2.0], 0.25, false),
            2,
        ));
        boites.push((
            boite_skin((48, 48), (bras, 12, 4), [4.0, 12.0, -2.0], 0.25, false),
            3,
        ));
        boites.push((
            boite_skin((0, 32), (4, 12, 4), [-4.0, 0.0, -2.0], 0.25, false),
            4,
        ));
        boites.push((
            boite_skin((0, 48), (4, 12, 4), [0.0, 0.0, -2.0], 0.25, false),
            5,
        ));
    } else {
        boites.push((
            boite_skin((40, 16), (bras, 12, 4), [4.0, 12.0, -2.0], 0.0, true),
            3,
        ));
        boites.push((
            boite_skin((0, 16), (4, 12, 4), [0.0, 0.0, -2.0], 0.0, true),
            5,
        ));
        boites.push((
            boite_skin((32, 0), (8, 8, 8), [-4.0, 24.0, -4.0], 0.5, false),
            0,
        ));
    }
    let nb_skin = boites.len();
    let membres = [p.tete, p.corps, p.bras_d, p.bras_g, p.jambe_d, p.jambe_g];
    for (boite, partie) in boites.iter_mut() {
        let m = membres[*partie];
        boite.incline = m.incline;
        boite.roulis = m.roulis;
        boite.pivot = m.pivot;
        boite.decale = m.decale;
    }
    // cape ou élytres, accrochées au dos (elles suivent l'inclinaison du corps)
    let facteur = cape_img
        .as_ref()
        .map(|c| (c.width() / 64).max(1))
        .unwrap_or(1);
    if elytres {
        for (cote, miroir) in [(1.0f32, false), (-1.0, true)] {
            let mut aile = boite_skin((22, 0), (10, 20, 2), [-5.0, 4.0, -4.0], 0.0, miroir);
            aile.incline = p.ailes.0 + p.corps.incline;
            aile.roulis = p.ailes.1 * cote;
            aile.pivot = [5.0 * cote, 24.0, -2.0];
            aile.decale = p.corps.decale;
            aile.faces[0] = aile.faces[1];
            boites.push((aile, 1));
        }
    } else if cape_img.is_some() {
        let mut c = boite_cape(facteur);
        c.incline = p.cape + p.corps.incline;
        c.decale = p.corps.decale;
        boites.push((c, 1));
    }
    let nb_avant_modeles = boites.len();
    // cosmétiques 3D : chaque cube suit la tête ou le corps
    let mut textures_modeles: Vec<&RgbaImage> = Vec::new();
    for modele in modeles {
        let partie = match modele.attache {
            Attache::Tete => 0,
            Attache::Corps => 1,
        };
        let m = membres[partie];
        for b in &modele.boites {
            let mut b = b.clone();
            b.incline = m.incline;
            b.roulis = m.roulis;
            b.pivot = m.pivot;
            b.decale = m.decale;
            boites.push((b, partie));
            textures_modeles.push(&modele.texture);
        }
    }

    // Caméra orthographique sur une sphère autour du personnage
    let (sy_, cy_) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    let vers_camera = [sy_ * cp, sp, cy_ * cp];
    let regard = [-vers_camera[0], -vers_camera[1], -vers_camera[2]];
    let droite = [cy_, 0.0, -sy_];
    let haut = [
        vers_camera[1] * droite[2] - vers_camera[2] * droite[1],
        vers_camera[2] * droite[0] - vers_camera[0] * droite[2],
        vers_camera[0] * droite[1] - vers_camera[1] * droite[0],
    ];
    // un chapeau 3D dépasse de la tête : le cadre monte un peu
    let chapeau = if modeles.iter().any(|m| m.attache == Attache::Tete) {
        2.5
    } else {
        0.0
    };
    let centre = [
        0.0,
        16.0 + chapeau - 4.0 * (p.global / 1.25).clamp(0.0, 1.0),
        0.0,
    ];
    let origine = [
        centre[0] + vers_camera[0] * 100.0,
        centre[1] + vers_camera[1] * 100.0,
        centre[2] + vers_camera[2] * 100.0,
    ];

    // tout le modèle basculé (vol) autour de la taille, puis chaque boîte dans son propre repère
    let taille = [0.0f32, 12.0, 0.0];
    let vers_modele = |v: [f32; 3], point: bool| -> [f32; 3] {
        if p.global == 0.0 {
            return v;
        }
        if point {
            let r = tourner_inverse(
                [v[0] - taille[0], v[1] - taille[1], v[2] - taille[2]],
                p.global,
                0.0,
            );
            [r[0] + taille[0], r[1] + taille[1], r[2] + taille[2]]
        } else {
            tourner_inverse(v, p.global, 0.0)
        }
    };
    let (o_m, d_m, h_m, r_m) = (
        vers_modele(origine, true),
        vers_modele(droite, false),
        vers_modele(haut, false),
        vers_modele(regard, false),
    );
    // partie du corps de chaque boîte : celles du skin, puis la cape ou les élytres
    let parties: Vec<u8> = boites
        .iter()
        .enumerate()
        .map(|(i, (_, partie))| {
            if i >= nb_avant_modeles {
                *partie as u8
            } else if i >= nb_skin {
                if elytres {
                    partie::ELYTRES
                } else {
                    partie::CAPE
                }
            } else {
                *partie as u8
            }
        })
        .collect();
    let posees: Vec<BoitePosee> = boites
        .iter()
        .enumerate()
        .map(|(i, (boite, _))| {
            let texture = if i >= nb_avant_modeles {
                textures_modeles[i - nb_avant_modeles]
            } else if i >= nb_skin {
                cape_img.as_ref().unwrap()
            } else {
                skin
            };
            let [px, py, pz] = boite.pivot;
            let [dx, dy, dz] = boite.decale;
            let o = tourner_inverse(
                [o_m[0] - dx - px, o_m[1] - dy - py, o_m[2] - dz - pz],
                boite.incline,
                boite.roulis,
            );
            let mut origine = [o[0] + px, o[1] + py, o[2] + pz];
            let mut droite = tourner_inverse(d_m, boite.incline, boite.roulis);
            let mut haut = tourner_inverse(h_m, boite.incline, boite.roulis);
            let mut regard = tourner_inverse(r_m, boite.incline, boite.roulis);
            // cube d'un cosmétique 3D : la caméra ramenée dans le repère du cube (inverse de
            // monde = rotation × local + position ; la rotation est orthogonale)
            if let Some((rot, pos)) = &boite.transformation {
                let inverse = |v: [f32; 3]| -> [f32; 3] {
                    [
                        rot[0][0] * v[0] + rot[1][0] * v[1] + rot[2][0] * v[2],
                        rot[0][1] * v[0] + rot[1][1] * v[1] + rot[2][1] * v[2],
                        rot[0][2] * v[0] + rot[1][2] * v[1] + rot[2][2] * v[2],
                    ]
                };
                origine = inverse([
                    origine[0] - pos[0],
                    origine[1] - pos[1],
                    origine[2] - pos[2],
                ]);
                droite = inverse(droite);
                haut = inverse(haut);
                regard = inverse(regard);
            }
            BoitePosee {
                boite,
                texture,
                origine,
                droite,
                haut,
                regard,
            }
        })
        .collect();

    // Rectangle de l'écran que couvre chaque boîte : ses 8 coins vus par la caméra (orthographique,
    // donc une simple projection). Un pixel ne teste que les boîtes dont le rectangle le contient,
    // et les lignes sans aucune boîte sont sautées.
    let demi = LIVE as f32 / 2.0;
    let rectangles: Vec<[u32; 4]> = posees
        .iter()
        .map(|bp| {
            let (mut x0, mut x1, mut y0, mut y1) = (
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::INFINITY,
                f32::NEG_INFINITY,
            );
            for coin in 0..8 {
                let q = [
                    if coin & 1 == 0 {
                        bp.boite.min[0]
                    } else {
                        bp.boite.max[0]
                    },
                    if coin & 2 == 0 {
                        bp.boite.min[1]
                    } else {
                        bp.boite.max[1]
                    },
                    if coin & 4 == 0 {
                        bp.boite.min[2]
                    } else {
                        bp.boite.max[2]
                    },
                ];
                let v = [
                    q[0] - bp.origine[0],
                    q[1] - bp.origine[1],
                    q[2] - bp.origine[2],
                ];
                let sx = v[0] * bp.droite[0] + v[1] * bp.droite[1] + v[2] * bp.droite[2];
                let sy = v[0] * bp.haut[0] + v[1] * bp.haut[1] + v[2] * bp.haut[2];
                // pixel dont le centre est en (sx, sy) : voir le calcul de sx et sy plus bas
                let (px, py) = (sx * ECHELLE + demi - 0.5, demi - sy * ECHELLE - 0.5);
                x0 = x0.min(px);
                x1 = x1.max(px);
                y0 = y0.min(py);
                y1 = y1.max(py);
            }
            // un pixel de marge contre les arrondis
            let borne = |v: f32| v.clamp(0.0, LIVE as f32) as u32;
            [
                borne(x0.floor() - 1.0),
                borne(x1.ceil() + 2.0),
                borne(y0.floor() - 1.0),
                borne(y1.ceil() + 2.0),
            ]
        })
        .collect();
    let lignes_utiles = rectangles
        .iter()
        .fold((LIVE, 0), |(a, b), r| (a.min(r[2]), b.max(r[3])));

    // Rendu en bandes de lignes, directement dans l'image, sur plusieurs cœurs. Les fils sont
    // gardés d'une image à l'autre (en créer jusqu'à 8 par image, 60 fois par seconde, coûtait
    // plus que le dessin lui-même).
    let mut image = RgbaImage::from_pixel(LIVE, LIVE, Rgba([0, 0, 0, 0]));
    let mut carte = vec![partie::AUCUNE; (LIVE * LIVE) as usize];
    let fils = fils_du_rendu().current_num_threads().max(1) as u32;
    let (premiere, derniere) = lignes_utiles;
    if premiere < derniere {
        let bande = (derniere - premiere).div_ceil(fils);
        let largeur = LIVE as usize * 4;
        let utiles = &mut image.as_mut()[premiere as usize * largeur..derniere as usize * largeur];
        let carte_utile = &mut carte[(premiere * LIVE) as usize..(derniere * LIVE) as usize];
        fils_du_rendu().scope(|sc| {
            let (posees, rectangles, parties) = (&posees, &rectangles, &parties);
            for (f, (morceau, morceau_carte)) in utiles
                .chunks_mut(bande as usize * largeur)
                .zip(carte_utile.chunks_mut((bande * LIVE) as usize))
                .enumerate()
            {
                sc.spawn(move |_| {
                    let mut candidates: Vec<usize> = Vec::with_capacity(posees.len());
                    for (l, (ligne, ligne_carte)) in morceau
                        .chunks_mut(largeur)
                        .zip(morceau_carte.chunks_mut(LIVE as usize))
                        .enumerate()
                    {
                        let py = premiere + f as u32 * bande + l as u32;
                        candidates.clear();
                        candidates.extend(
                            (0..posees.len())
                                .filter(|&i| (rectangles[i][2]..rectangles[i][3]).contains(&py)),
                        );
                        if candidates.is_empty() {
                            continue;
                        }
                        let x0 = candidates
                            .iter()
                            .map(|&i| rectangles[i][0])
                            .min()
                            .unwrap_or(0);
                        let x1 = candidates
                            .iter()
                            .map(|&i| rectangles[i][1])
                            .max()
                            .unwrap_or(0);
                        let sy = (demi - (py as f32 + 0.5)) / ECHELLE;
                        for px in x0..x1 {
                            let sx = (px as f32 + 0.5 - demi) / ECHELLE;
                            let mut meilleur: Option<(f32, Rgba<u8>, u8)> = None;
                            for &i in candidates.iter() {
                                if !(rectangles[i][0]..rectangles[i][1]).contains(&px) {
                                    continue;
                                }
                                let bp = &posees[i];
                                let o = [
                                    bp.origine[0] + bp.droite[0] * sx + bp.haut[0] * sy,
                                    bp.origine[1] + bp.droite[1] * sx + bp.haut[1] * sy,
                                    bp.origine[2] + bp.droite[2] * sx + bp.haut[2] * sy,
                                ];
                                let Some((t, axe, positif)) = entrer(o, bp.regard, bp.boite) else {
                                    continue;
                                };
                                if meilleur.map(|(m, _, _)| t >= m).unwrap_or(false) {
                                    continue;
                                }
                                let point = [
                                    o[0] + bp.regard[0] * t,
                                    o[1] + bp.regard[1] * t,
                                    o[2] + bp.regard[2] * t,
                                ];
                                let couleur =
                                    echantillon(bp.boite, bp.texture, point, axe, positif);
                                if couleur[3] >= 128 {
                                    meilleur = Some((
                                        t,
                                        Rgba([couleur[0], couleur[1], couleur[2], 255]),
                                        parties[i],
                                    ));
                                }
                            }
                            if let Some((_, c, partie)) = meilleur {
                                let k = px as usize * 4;
                                let mut c = c;
                                // partie survolée : éclaircie et teintée du bleu de Misty
                                if surbrillance.contains(&partie) {
                                    let bleu = [111.0f32, 156.0, 255.0];
                                    for (canal, b) in c.0.iter_mut().take(3).zip(bleu) {
                                        *canal = (*canal as f32 * 0.55 + b * 0.45).round() as u8;
                                    }
                                }
                                ligne[k..k + 4].copy_from_slice(&c.0);
                                ligne_carte[px as usize] = partie;
                            }
                        }
                    }
                });
            }
        });
    }
    Some((image, carte))
}

#[cfg(test)]
mod tests_live {
    use super::*;

    #[test]
    fn carte_des_parties() {
        let steve = image::load_from_memory(STEVE_PNG).unwrap().to_rgba8();
        let cape = image::load_from_memory(texture_misty()).unwrap().to_rgba8();
        // de face : la tête est en haut, au milieu
        let (_, face) = rendre_live_parties(
            &steve,
            Some(&cape),
            false,
            false,
            Anim::Repos,
            0.0,
            0.0,
            0.0,
            &[],
        )
        .unwrap();
        let tete = (0..LIVE as usize)
            .find(|&y| face[y * LIVE as usize + LIVE as usize / 2] != partie::AUCUNE)
            .unwrap();
        assert_eq!(face[tete * LIVE as usize + LIVE as usize / 2], partie::TETE);
        // de dos : la cape couvre le milieu du corps
        let (_, dos) = rendre_live_parties(
            &steve,
            Some(&cape),
            false,
            false,
            Anim::Repos,
            0.0,
            std::f32::consts::PI,
            0.0,
            &[],
        )
        .unwrap();
        assert_eq!(
            dos[(LIVE as usize / 2) * LIVE as usize + LIVE as usize / 2],
            partie::CAPE
        );
        // avec des élytres
        let (_, ailes) = rendre_live_parties(
            &steve,
            Some(&cape),
            false,
            true,
            Anim::Repos,
            0.0,
            std::f32::consts::PI,
            0.0,
            &[],
        )
        .unwrap();
        assert!(ailes.contains(&partie::ELYTRES));
    }

    #[test]
    fn toutes_les_animations_se_dessinent() {
        let steve = image::load_from_memory(STEVE_PNG).unwrap().to_rgba8();
        for a in 0..5 {
            let img = rendre_live_parties(
                &steve,
                None,
                false,
                a == 3,
                Anim::depuis(a),
                0.4,
                0.5,
                0.3,
                &[],
            )
            .unwrap()
            .0;
            assert_eq!(img.dimensions(), (LIVE, LIVE));
            assert!(img.pixels().any(|p| p[3] == 255), "animation {a} vide");
        }
    }

    /// Temps d'une image du studio : `cargo test --release temps_du_studio -- --ignored --nocapture`.
    /// `MISTY_STUDIO=<dossier>` y écrit aussi les images, pour comparer deux versions du rendu.
    #[test]
    #[ignore]
    fn temps_du_studio() {
        let steve = image::load_from_memory(STEVE_PNG).unwrap().to_rgba8();
        let cape = image::load_from_memory(texture_misty()).unwrap().to_rgba8();
        let dossier = std::env::var("MISTY_STUDIO").ok();
        let debut = std::time::Instant::now();
        let mut n = 0;
        for a in 0..5 {
            for i in 0..24 {
                let yaw = i as f32 * 0.27;
                let img = rendre_live_parties(
                    &steve,
                    Some(&cape),
                    i % 2 == 0,
                    a == 3,
                    Anim::depuis(a),
                    i as f32 * 0.13,
                    yaw,
                    0.2 + (i % 5) as f32 * 0.1,
                    &[],
                )
                .unwrap()
                .0;
                n += 1;
                if let Some(d) = &dossier {
                    img.save(format!("{d}/{a}-{i}.png")).unwrap();
                }
            }
        }
        println!(
            "studio : {:.2} ms par image",
            debut.elapsed().as_secs_f64() * 1000.0 / n as f64
        );
    }
}

/// Planche des poses, pour régler les animations : `MISTY_POSES=<dossier> cargo test planche_des_poses -- --ignored`.
#[cfg(test)]
mod tests_planche {
    use super::*;

    #[test]
    #[ignore]
    fn planche_des_poses() {
        let Ok(dossier) = std::env::var("MISTY_POSES") else {
            return;
        };
        let steve = image::load_from_memory(STEVE_PNG).unwrap().to_rgba8();
        let cape = image::load_from_memory(texture_misty()).unwrap().to_rgba8();
        let mut planche = RgbaImage::from_pixel(LIVE * 5, LIVE * 3, Rgba([30, 40, 70, 255]));
        for a in 0..5 {
            for (ligne, yaw) in [(0u32, 0.35f32), (1, std::f32::consts::FRAC_PI_2), (2, 2.6)] {
                let elytres = a == 3 || a == 4;
                let img = rendre_live_parties(
                    &steve,
                    Some(&cape),
                    false,
                    elytres,
                    Anim::depuis(a),
                    0.3,
                    yaw,
                    0.15,
                    &[],
                )
                .unwrap()
                .0;
                imageops::overlay(
                    &mut planche,
                    &img,
                    (a as u32 * LIVE) as i64,
                    (ligne * LIVE) as i64,
                );
            }
        }
        planche
            .save(std::path::Path::new(&dossier).join("poses.png"))
            .unwrap();
    }
}
