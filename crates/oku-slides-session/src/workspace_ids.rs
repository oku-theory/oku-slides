//! Identifiants d'onglets et de fenêtres.

/// Plus grand numéro d'onglet ou de fenêtre : 18 chiffres.
pub(super) const MAX_NUMBER: u64 = 999_999_999_999_999_999;

/// `prefix` suivi de 1 à 18 chiffres (un `u64` sans débordement).
fn numbered(s: &str, prefix: &str) -> bool {
    s.strip_prefix(prefix).is_some_and(|n| (1..=18).contains(&n.len()) && n.bytes().all(|b| b.is_ascii_digit()))
}

/// Identifiant d'onglet sûr dans un chemin : `t-` et des chiffres.
pub(super) fn is_tab_id(id: &str) -> bool {
    numbered(id, "t-")
}

/// Un label de fenêtre de l'espace de travail : `w-<n>`.
///
/// Accepté par Tauri et par la capacité `w-*`.
#[must_use]
pub fn is_window_label(label: &str) -> bool {
    numbered(label, "w-")
}

/// Le numéro d'un identifiant déjà validé.
pub(super) fn number(id: &str) -> u64 {
    id.get(2..).and_then(|n| n.parse().ok()).unwrap_or(0)
}
