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
// build.rs — intègre la clé API CurseForge (contenu/curseforge.rs la lit avec option_env!).
// Le fichier est privé et ignoré par git : <racine du repo>/curseforge.key. Sans lui, CurseForge
// est désactivé (la CI des PR compile sans).

fn main() {
    let chemin = "../../curseforge.key";
    println!("cargo:rerun-if-changed={chemin}");
    if let Ok(cle) = std::fs::read_to_string(chemin) {
        let cle = cle.trim();
        if !cle.is_empty() {
            println!("cargo:rustc-env=MISTY_CF_KEY={cle}");
        }
    }
}
