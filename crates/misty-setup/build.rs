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
// build.rs — compile la fenêtre (ui/setup.slint) et intègre l'installateur Inno Setup.
//
// MISTY_SETUP_INTERIEUR : chemin de l'installateur Inno Setup compilé par installer/build.ps1.
// Sans lui (développement, CI), l'exe ne contient rien et l'installation est simulée.

use std::path::PathBuf;

fn main() {
    slint_build::compile("ui/setup.slint").expect("compilation de la fenêtre de l'installateur");

    println!("cargo:rerun-if-env-changed=MISTY_SETUP_INTERIEUR");
    let sortie = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("interieur.exe");
    match std::env::var("MISTY_SETUP_INTERIEUR") {
        Ok(chemin) if !chemin.is_empty() => {
            println!("cargo:rerun-if-changed={chemin}");
            std::fs::copy(&chemin, &sortie).expect("copie de l'installateur Inno Setup");
        }
        _ => std::fs::write(&sortie, []).unwrap(),
    }

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../misty-app/assets/icone.ico");
        res.set("FileDescription", "Mistly Launcher Installer");
        res.set("ProductName", "Mistly Launcher");
        let _ = res.compile();
    }
}
