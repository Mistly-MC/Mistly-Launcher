//! Comptes : session Microsoft et coffre des jetons (Gestionnaire d'identifiants Windows).

pub mod account;
pub mod coffre;
pub mod erreur;

pub use erreur::AuthErreur;
