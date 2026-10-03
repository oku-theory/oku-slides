//! Transformations d'une surface déjà construite : mise à jour de sa copie, gel en lecture seule.

use oku_slides_a2ui::{Component, ComponentKind, ServerBody, ServerMessage, UpdateDataModel};
use serde_json::Value;

use super::Surface;

impl Surface {
    /// Applique des messages de mise à jour à la copie de l'hôte (resynchronisation).
    ///
    /// Un modèle racine remplace celui de la surface (et rend caducs les
    /// chemins écrits avant lui) ; un chemin `/clé` (JSON Pointer, `~0`/`~1`
    /// décodés) y écrit sa valeur, ou retire la clé sans valeur ; tout autre
    /// message est ajouté tel quel.
    pub fn apply_patch(&mut self, messages: &[ServerMessage]) {
        for m in messages {
            let ServerBody::UpdateDataModel(patch) = &m.body else {
                self.messages.push(m.clone());
                continue;
            };
            match patch.path.as_deref() {
                None => {
                    // Les écritures par chemin antérieures sont remplacées par la nouvelle racine.
                    self.messages.retain(|x| !matches!(&x.body, ServerBody::UpdateDataModel(u) if u.path.is_some()));
                    match self.root_model() {
                        Some(root) => root.value.clone_from(&patch.value),
                        None => self.messages.push(m.clone()),
                    }
                }
                Some(path) => {
                    if let Some(key) = path.strip_prefix('/')
                        && !key.contains('/')
                        && let Some(root) = self.root_model()
                        && let Some(Value::Object(map)) = &mut root.value
                    {
                        let key = key.replace("~1", "/").replace("~0", "~");
                        match &patch.value {
                            Some(v) => map.insert(key, v.clone()),
                            None => map.remove(&key),
                        };
                    } else {
                        self.messages.push(m.clone());
                    }
                }
            }
        }
    }

    /// La surface envoie-t-elle une action (bouton, champ) ?
    pub fn has_actions(&self) -> bool {
        self.messages.iter().any(|m| match &m.body {
            ServerBody::UpdateComponents(u) => u.components.iter().any(|c| !c.kind.actions().is_empty()),
            _ => false,
        })
    }

    /// La même surface, en lecture seule : champs sans action, boutons retirés (et retirés
    /// de leurs parents ; un parent à enfant unique dont l'enfant disparaît disparaît aussi).
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

#[cfg(test)]
#[path = "surface_ops_tests.rs"]
mod surface_ops_tests;
