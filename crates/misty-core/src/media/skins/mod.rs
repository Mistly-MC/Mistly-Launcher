//! Skin et capes d'un compte Microsoft (API api.minecraftservices.com).

use crate::{auth::account, systeme::http};
use image::{imageops, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

// Sous-modules : fichiers (historique, cache), rendu (vues fixes), studio (aperçu en temps réel)
mod fichiers;
mod rendu;
mod studio;
pub use fichiers::*;
pub use rendu::*;
pub use studio::*;

const API: &str = "https://api.minecraftservices.com/minecraft/profile";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cape {
    pub id: String,
    pub nom: String,
    pub url: String,
    pub active: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Profil {
    pub skin_url: String,
    pub fin: bool,
    pub capes: Vec<Cape>,
}

#[derive(Deserialize)]
struct ProfilJson {
    #[serde(default)]
    skins: Vec<SkinJson>,
    #[serde(default)]
    capes: Vec<CapeJson>,
}

#[derive(Deserialize)]
struct SkinJson {
    #[serde(default)]
    state: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    variant: String,
}

#[derive(Deserialize)]
struct CapeJson {
    id: String,
    #[serde(default)]
    state: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    alias: String,
}

/// Jeton valide (renouvelé si besoin).
async fn jeton() -> anyhow::Result<String> {
    Ok(account::connecter().await?.jeton)
}

/// Mojang refuse les demandes (HTTP 429). `attente` : secondes demandées par Mojang (0 = non précisé).
#[derive(Debug)]
pub struct Limite {
    pub attente: u64,
}

impl std::fmt::Display for Limite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HTTP 429 (trop de requêtes)")
    }
}

impl std::error::Error for Limite {}

/// Skin et capes actuels du compte.
pub async fn profil() -> anyhow::Result<Profil> {
    // Outil de développement : MISTY_FAKE_429=1 simule la limite de requêtes de Mojang (version debug seulement)
    if cfg!(debug_assertions) && std::env::var_os("MISTY_FAKE_429").is_some() {
        return Err(Limite { attente: 4 }.into());
    }
    let r = http::client()
        .get(API)
        .bearer_auth(jeton().await?)
        .send()
        .await?;
    if r.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let attente = r
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        return Err(Limite { attente }.into());
    }
    let p: ProfilJson = r.error_for_status()?.json().await?;
    let skin = p
        .skins
        .iter()
        .find(|s| s.state == "ACTIVE")
        .or(p.skins.first());
    Ok(Profil {
        skin_url: skin.map(|s| s.url.clone()).unwrap_or_default(),
        fin: skin
            .map(|s| s.variant.eq_ignore_ascii_case("slim"))
            .unwrap_or(false),
        capes: p
            .capes
            .into_iter()
            .map(|c| Cape {
                active: c.state == "ACTIVE",
                nom: majuscule(&c.alias),
                id: c.id,
                url: c.url,
            })
            .collect(),
    })
}

fn majuscule(alias: &str) -> String {
    let mut c = alias.chars();
    match c.next() {
        Some(p) => p.to_uppercase().collect::<String>() + c.as_str(),
        None => "Cape".into(),
    }
}

pub async fn changer_skin(png: Vec<u8>, fin: bool) -> anyhow::Result<()> {
    let limite = "----MistyLimite7MA4YWxkTrZu0gW";
    let mut corps: Vec<u8> = Vec::new();
    corps.extend_from_slice(
        format!(
            "--{limite}\r\nContent-Disposition: form-data; name=\"variant\"\r\n\r\n{}\r\n--{limite}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"skin.png\"\r\nContent-Type: image/png\r\n\r\n",
            if fin { "slim" } else { "classic" }
        )
        .as_bytes(),
    );
    corps.extend_from_slice(&png);
    corps.extend_from_slice(format!("\r\n--{limite}--\r\n").as_bytes());
    let r = http::client()
        .post(format!("{API}/skins"))
        .bearer_auth(jeton().await?)
        .header(
            "Content-Type",
            format!("multipart/form-data; boundary={limite}"),
        )
        .body(corps)
        .send()
        .await?;
    verifier(r).await
}

/// Active une cape (`None` = ne plus en porter).
pub async fn activer_cape(id: Option<&str>) -> anyhow::Result<()> {
    if cfg!(debug_assertions) && std::env::var_os("MISTY_FAKE_429").is_some() {
        return Err(Limite { attente: 2 }.into()); // simulation (voir profil)
    }
    let jeton = jeton().await?;
    let url = format!("{API}/capes/active");
    let r = match id {
        Some(id) => {
            http::client()
                .put(url)
                .bearer_auth(jeton)
                .json(&serde_json::json!({ "capeId": id }))
                .send()
                .await?
        }
        None => http::client().delete(url).bearer_auth(jeton).send().await?,
    };
    verifier(r).await
}

async fn verifier(r: reqwest::Response) -> anyhow::Result<()> {
    if r.status().is_success() {
        return Ok(());
    }
    if r.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let attente = r
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        return Err(Limite { attente }.into());
    }
    let statut = r.status();
    let texte = r.text().await.unwrap_or_default();
    let detail = serde_json::from_str::<serde_json::Value>(&texte)
        .ok()
        .and_then(|v| {
            v["errorMessage"]
                .as_str()
                .or(v["error"].as_str())
                .map(str::to_string)
        })
        .unwrap_or(texte);
    anyhow::bail!("{statut} {detail}")
}

/// Un skin est valide s'il fait 64×64 ou 64×32 pixels.
pub fn skin_valide(png: &[u8]) -> bool {
    image::load_from_memory(png)
        .map(|i| i.width() == 64 && (i.height() == 64 || i.height() == 32))
        .unwrap_or(false)
}

/// Modèle d'un skin : vrai pour les bras fins (Alex, « slim »), faux pour les bras larges (Steve).
/// Un bras fin fait 3 pixels de large au lieu de 4 : sur la texture, les 2 dernières colonnes de la
/// zone de chaque bras restent vides (bras droit : x 54–55, y 20–31 ; bras gauche : x 46–47, y 52–63).
/// Les anciens skins 64 × 32 sont toujours larges.
pub fn est_fin(png: &[u8]) -> bool {
    let Ok(img) = image::load_from_memory(png) else {
        return false;
    };
    let img = img.to_rgba8();
    if img.width() != 64 || img.height() != 64 {
        return false;
    }
    let vide =
        |x0: u32, y0: u32| (x0..x0 + 2).all(|x| (y0..y0 + 12).all(|y| img.get_pixel(x, y)[3] == 0));
    vide(54, 20) && vide(46, 52)
}

#[cfg(test)]
mod tests_modele {
    use super::*;

    #[test]
    fn steve_et_alex() {
        let steve = STEVE_PNG;
        assert!(!est_fin(steve));
        // la même texture, bras réduits à 3 pixels : les colonnes en trop sont effacées
        let mut alex = image::load_from_memory(steve).unwrap().to_rgba8();
        for (x0, y0) in [(54u32, 20u32), (46, 52)] {
            for x in x0..x0 + 2 {
                for y in y0..y0 + 12 {
                    alex.put_pixel(x, y, Rgba([0, 0, 0, 0]));
                }
            }
        }
        let mut png = Vec::new();
        alex.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        assert!(est_fin(&png));
    }
}
