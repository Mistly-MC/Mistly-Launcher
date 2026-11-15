// build.rs — intègre l'icône Windows et la clé CurseForge. Misty Client n'est plus dans l'exe :
// le launcher le télécharge depuis les releases GitHub de Misty-Client (src/services/client.rs).
// (L'interface Slint se compile dans son propre crate : misty-ui/.)

fn main() {
    // Clé API CurseForge (privée, ignorée par git) : misty-rust/curseforge.key
    for chemin in ["curseforge.key"] {
        println!("cargo:rerun-if-changed={chemin}");
        if let Ok(cle) = std::fs::read_to_string(chemin) {
            let cle = cle.trim();
            if !cle.is_empty() {
                println!("cargo:rustc-env=MISTY_CF_KEY={cle}");
                break;
            }
        }
    }

    // Notes de version affichées dans l'application
    println!("cargo:rerun-if-changed=../CHANGELOG.md");

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
