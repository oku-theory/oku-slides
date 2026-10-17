//! Consignes de l'agent, en anglais seul : l'agent les lit, pas l'utilisateur.
//!
//! `INSTRUCTIONS` et `WIDGETS` partent au premier tour d'une session ; `IMPORT`
//! au seul tour de fine-tuning d'un import (`AgentCommand::Tune`).
//!
//! La langue de ce que l'utilisateur lit vient du contexte de chaque tour
//! (`interface language` de `<oku-slides-context>`) ; le contenu du deck prend
//! la langue du deck (`deck language`). Les fichiers n'ont pas de saut de ligne
//! final : leur texte part tel quel.

/// Rôle de l'agent, outils à employer, règle des langues.
pub(crate) const INSTRUCTIONS: &str = include_str!("instructions/agent.md");

/// Écrire un widget Typst et le mettre dans le deck.
pub(crate) const WIDGETS: &str = include_str!("instructions/widgets.md");

/// Fine-tuner un deck importé : la boucle bornée, envoyée au seul tour de fine-tuning.
pub(crate) const IMPORT: &str = include_str!("instructions/import.md");

#[cfg(test)]
#[path = "instructions_tests.rs"]
mod instructions_tests;
