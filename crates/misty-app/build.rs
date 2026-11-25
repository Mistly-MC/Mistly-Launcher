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
// build.rs — intègre l'icône Windows et les infos de l'exe. La clé CurseForge est intégrée par
// misty-core (crates/misty-core/build.rs). Misty Client n'est pas dans l'exe : le launcher le
// télécharge depuis les releases GitHub de Misty-Client (misty-core, minecraft/client.rs).
// (L'interface Slint se compile dans son propre crate : crates/misty-ui.)

fn main() {
    // Notes de version affichées dans l'application
    println!("cargo:rerun-if-changed=../../CHANGELOG.md");

    // Icône de l'exe (Windows)
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icone.ico");
        // nom affiché par Windows (Gestionnaire des tâches, propriétés de l'exe) ; l'exe garde le nom Misty.exe
        res.set("FileDescription", "Mistly Launcher");
        res.set("ProductName", "Mistly Launcher");
        let _ = res.compile();
    }
}
