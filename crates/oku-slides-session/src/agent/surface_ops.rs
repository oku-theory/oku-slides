//! Transformations d'une surface déjà construite : mise à jour de sa copie, gel en lecture seule.

use oku_slides_a2ui::{Component, ComponentKind, ServerBody, ServerMessage, UpdateDataModel};
use serde_json::Value;

use super::Surface;

impl Surface {
    /// Applique des messages de mise à jour à la copie de l'hôte (resynchronisation).
    ///
    /// Un modèle racine remplace celui de la surface (et rend caducs les
    /// chemins écrits avant lui) ; un chemin (JSON Pointer, `~0`/`~1` décodés,
    /// `/steps/4` compris) y écrit sa valeur, ou retire la clé sans valeur ; un
    /// chemin qui n'y mène pas est ignoré ; sans modèle racine, ou pour tout autre message,
    /// le message est ajouté tel quel.
    pub fn apply_patch(&mut self, messages: &[ServerMessage]) {
        for m in messages {
            let ServerBody::UpdateDataModel(update) = &m.body else {
                self.messages.push(m.clone());
                continue;
            };
            match update.path.as_deref() {
                None => {
                    // Les écritures par chemin antérieures sont remplacées par la nouvelle racine.
                    self.messages.retain(|x| !matches!(&x.body, ServerBody::UpdateDataModel(u) if u.path.is_some()));
                    match self.root_model() {
                        Some(root) => root.value.clone_from(&update.value),
                        None => self.messages.push(m.clone()),
                    }
                }
                Some(path) => match self.root_model().and_then(|root| root.value.as_mut()) {
                    // Un chemin qui n'y mène pas est caduc : rejoué après la racine, il
                    // écraserait un état plus récent (resynchronisation).
                    Some(value) => {
                        if !write_pointer(value, path, update.value.as_ref()) {
                            tracing::debug!(path, "écriture sans destination dans le modèle : ignorée");
                        }
                    }
                    None => self.messages.push(m.clone()),
                },
            }
        }
    }

    /// La surface envoie-t-elle une action (bouton, champ) ?
    #[must_use]
    pub fn has_actions(&self) -> bool {
        self.messages.iter().any(|m| match &m.body {
            ServerBody::UpdateComponents(u) => u.components.iter().any(|c| !c.kind.actions().is_empty()),
            _ => false,
        })
    }

    /// La même surface, en lecture seule : champs sans action, boutons retirés (et retirés
    /// de leurs parents ; un parent à enfant unique dont l'enfant disparaît disparaît aussi).
    #[must_use]
    pub fn without_actions(&self) -> Surface {
        let mut s = self.clone();
        for m in &mut s.messages {
            let ServerBody::UpdateComponents(u) = &mut m.body else { continue };
            let mut removed: Vec<String> = Vec::new();
            let mut kept = Vec::with_capacity(u.components.len());
            for c in std::mem::take(&mut u.components) {
                match c.kind.without_action() {
                    Some(kind) => kept.push(Component { id: c.id, kind }),
                    None => removed.push(c.id),
                }
            }
            // La racine ne disparaît jamais : un bouton racine devient une colonne vide.
            if removed.iter().any(|id| id == "root") {
                removed.retain(|id| id != "root");
                kept.push(Component::column("root", &[]));
            }
            // Jusqu'à stabilité : retirer un bouton peut vider une carte, qui disparaît à son tour.
            loop {
                let before = removed.len();
                kept.retain_mut(|c| match &mut c.kind {
                    ComponentKind::Column { children } | ComponentKind::Row { children } => {
                        children.retain(|id| !removed.contains(id));
                        true
                    }
                    ComponentKind::Card { child, .. } | ComponentKind::Disclosure { child, .. }
                        if removed.contains(child) =>
                    {
                        if c.id == "root" {
                            // La racine reste : elle devient une colonne vide.
                            c.kind = ComponentKind::Column { children: Vec::new() };
                            return true;
                        }
                        removed.push(c.id.clone());
                        false
                    }
                    _ => true,
                });
                if removed.len() == before {
                    break;
                }
            }
            u.components = kept;
        }
        s
    }

    /// Le message qui porte le modèle de données racine.
    fn root_model(&mut self) -> Option<&mut UpdateDataModel> {
        self.messages.iter_mut().find_map(|x| match &mut x.body {
            ServerBody::UpdateDataModel(u) if u.path.is_none() => Some(u),
            _ => None,
        })
    }
}

/// Écrit `value` (ou retire, sans valeur) au chemin `pointer` de `root`, comme `DataModel.set`
/// de l'interface : un index de liste égal à sa longueur ajoute. Rend `false` si le chemin
/// ne mène pas à un objet ou une liste existants (rien n'est écrit).
fn write_pointer(root: &mut Value, pointer: &str, value: Option<&Value>) -> bool {
    let Some((parent, leaf)) = pointer.rsplit_once('/') else { return false };
    let key = leaf.replace("~1", "/").replace("~0", "~");
    let Some(target) = (if parent.is_empty() { Some(root) } else { root.pointer_mut(parent) }) else {
        return false;
    };
    match (target, value) {
        (Value::Object(map), Some(v)) => {
            map.insert(key, v.clone());
        }
        (Value::Object(map), None) => {
            map.remove(&key);
        }
        (Value::Array(items), Some(v)) => match key.parse::<usize>() {
            Ok(i) if i == items.len() => items.push(v.clone()),
            Ok(i) => match items.get_mut(i) {
                Some(slot) => *slot = v.clone(),
                None => return false,
            },
            Err(_) => return false,
        },
        _ => return false,
    }
    true
}

#[cfg(test)]
#[path = "surface_ops_tests.rs"]
mod surface_ops_tests;
