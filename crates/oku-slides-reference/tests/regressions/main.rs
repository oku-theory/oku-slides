//! Tests de non-régression : un fichier par bogue corrigé.
//! Numérotation `issue_NNNN` : identifiant de tâche Oku quand il existe, locale sinon.
#![expect(clippy::panic, reason = "test helpers fail through panic!, out of reach of allow-panic-in-tests")]

mod issue_0898_embedded_font_path_traversal;
