//! Raccourcis clavier dans la notation de la plateforme.
//!
//! Les messages et l'aide écrivent les raccourcis à la manière de macOS
//! (`⌘⇧S`, `⌥↑`) ; ailleurs, ils se lisent `Ctrl+Shift+S`, `Alt+↑`. L'interface
//! fait de même pour les raccourcis de ses menus (`ui/src/keys.ts`).

use std::borrow::Cow;

const MODIFIERS: [char; 4] = ['⌃', '⌥', '⇧', '⌘'];

/// `text` dans la notation de la plateforme où tourne l'app.
pub fn platform_keys(text: &str) -> Cow<'_, str> {
    if cfg!(target_os = "macos") { Cow::Borrowed(text) } else { pc_keys(text) }
}

/// `text` avec ses raccourcis macOS récrits pour Windows et Linux.
///
/// Une suite de modificateurs devient leurs noms dans l'ordre d'un PC
/// (Ctrl, Alt, Shift), reliés par `+` à la touche qui suit, même séparée
/// par une espace (`⌘ Entrée`) ; seule, elle reste son nom (`⇧`).
pub fn pc_keys(text: &str) -> Cow<'_, str> {
    if !text.contains(MODIFIERS) {
        return Cow::Borrowed(text);
    }
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len() + 16);
    let mut i = 0;
    while i < chars.len() {
        if !MODIFIERS.contains(&chars[i]) {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && MODIFIERS.contains(&chars[i]) {
            i += 1;
        }
        let run = &chars[start..i];
        let names = [
            (run.contains(&'⌘') || run.contains(&'⌃')).then_some("Ctrl"),
            run.contains(&'⌥').then_some("Alt"),
            run.contains(&'⇧').then_some("Shift"),
        ];
        out.push_str(&names.into_iter().flatten().collect::<Vec<_>>().join("+"));
        // La touche suit : collée, ou après une espace si c'est un mot (`⌘ Entrée`).
        match (chars.get(i), chars.get(i + 1)) {
            (Some(c), _) if !c.is_whitespace() => out.push('+'),
            (Some(' '), Some(c)) if c.is_alphanumeric() => {
                out.push('+');
                i += 1;
            }
            _ => {}
        }
    }
    Cow::Owned(out)
}

#[cfg(test)]
#[path = "keys_tests.rs"]
mod keys_tests;
