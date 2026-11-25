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
//! Clients HTTP partagés (un seul par usage, comme les HttpClient statiques en C#).

/// Client général (Modrinth, Mojang, loaders…), avec le User-Agent du launcher : celui du
/// contexte (voir `systeme::contexte`). Un clone est léger (le client est partagé).
pub fn client() -> reqwest::Client {
    crate::systeme::contexte::actuel().http.clone()
}

/// Lit une réponse JSON en vérifiant le code HTTP.
pub async fn get_json<T: serde::de::DeserializeOwned>(url: &str) -> anyhow::Result<T> {
    let reponse = client().get(url).send().await?.error_for_status()?;
    Ok(reponse.json().await?)
}

pub async fn get_text(url: &str) -> anyhow::Result<String> {
    let reponse = client().get(url).send().await?.error_for_status()?;
    Ok(reponse.text().await?)
}
