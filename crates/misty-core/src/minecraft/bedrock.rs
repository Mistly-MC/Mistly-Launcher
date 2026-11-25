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
//! Minecraft Bedrock Edition (la version du Microsoft Store, Windows seulement).
//!
//! Mistly ne l'installe pas et ne la modifie pas : il vérifie qu'elle est installée (le protocole
//! `minecraft:` est enregistré par le jeu) et la lance par ce protocole, comme le menu Démarrer.

/// Bedrock peut exister sur ce système (Windows).
pub const PRISE_EN_CHARGE: bool = cfg!(windows);

/// Bedrock est installé : le protocole `minecraft:` est enregistré.
#[cfg(windows)]
pub fn installe() -> bool {
    use winreg::enums::HKEY_CLASSES_ROOT;
    winreg::RegKey::predef(HKEY_CLASSES_ROOT)
        .open_subkey(r"minecraft\shell\open\command")
        .is_ok()
}

/// Hors Windows, Bedrock n'existe pas en version PC.
#[cfg(not(windows))]
pub fn installe() -> bool {
    false
}

/// Lance Bedrock (s'il est installé). Renvoie false s'il ne l'est pas.
pub fn lancer() -> bool {
    if !installe() {
        return false;
    }
    crate::systeme::system_info::ouvrir("minecraft:");
    true
}

#[cfg(test)]
mod tests {
    #[test]
    fn hors_windows_rien() {
        if !super::PRISE_EN_CHARGE {
            assert!(!super::installe());
            assert!(!super::lancer());
        }
    }
}
