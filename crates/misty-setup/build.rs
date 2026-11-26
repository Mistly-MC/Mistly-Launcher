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
// build.rs — compile la fenêtre (ui/setup.slint) et intègre le launcher compressé.
//
// MISTY_SETUP_EXE : chemin de Misty.exe (release), donné par installer/build.ps1. Il est compressé
// (zstd, niveau 19) dans l'installateur. Sans lui (développement, CI), l'installateur ne contient
// rien et l'installation est simulée.

use std::path::PathBuf;

fn main() {
    slint_build::compile("ui/setup.slint").expect("compilation de la fenêtre de l'installateur");

    println!("cargo:rerun-if-env-changed=MISTY_SETUP_EXE");
    let sortie = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("misty.exe.zst");
    match std::env::var("MISTY_SETUP_EXE") {
        Ok(chemin) if !chemin.is_empty() => {
            println!("cargo:rerun-if-changed={chemin}");
            let exe = std::fs::read(&chemin).expect("lecture de Misty.exe");
            let compresse = zstd::bulk::compress(&exe, 19).expect("compression de Misty.exe");
            std::fs::write(&sortie, compresse).unwrap();
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
