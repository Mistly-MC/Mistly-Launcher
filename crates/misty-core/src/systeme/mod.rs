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
//! Système : chemins, réglages, stockage, mises à jour, lanceur fixe, raccourcis, télémétrie, journal, traductions, réseau, téléchargements, fichiers.

pub mod app_info;
pub mod archive;
pub mod contexte;
pub mod download;
pub mod erreur;
pub mod fichiers;
pub mod http;
pub mod journal;
pub mod lanceur;
pub mod loc;
pub mod paths;
pub mod reglages;
pub mod shortcut;
pub mod storage;
pub mod system_info;
pub mod telemetry;
pub mod update;
pub mod version_compat;

pub use erreur::MajErreur;
