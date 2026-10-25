//! Actions des surfaces persistantes (réglages, variantes, suites) et élagage.

use oku_slides_session::HOST_VOICE;
use oku_slides_session::agent::{ShowParams, ShowVariants, forms};
use serde::Deserialize;
use serde_json::{Map, Value};
use snafu::{OptionExt, ResultExt};

use super::error::{
    ExpiredSnafu, HubError, IncompleteSnafu, NoFieldSnafu, NoVariantSnafu, SessionSnafu, UnexpectedActionSnafu,
};
use super::{Answer, McpEvent, Shown, SurfaceHub};

/// Action d'une surface, telle que l'interface l'envoie à l'hôte.
#[derive(Debug, Clone, Deserialize)]
pub struct SurfaceAction {
    /// Id of the surface the action comes from.
    pub surface: String,
    /// Name of the action (`dismiss`: the surface is abandoned).
    pub name: String,
    /// The surface's form values at the time of the action.
    #[serde(default)]
    pub context: Map<String, Value>,
}

/// Ce qu'il advient d'une surface persistante après une action.
///
/// `changed` : l'action a changé le deck, les boîtes du hub (thème, packs) se relisent.
enum After {
    Keep {
        changed: bool,
    },
    Close {
        changed: bool,
    },
    Freeze,
    /// La surface reste, avec cet état (sélecteur de thème ou panneau des packs refait).
    Replace(Shown),
}

impl SurfaceHub {
    /// Applies an interface action.
    ///
    /// That is an answer to a question, or a change to a persistent surface
    /// (applied here, noted for the agent).
    ///
    /// The error is told in the user's language ([`Localize`](oku_slides_i18n::Localize));
    /// its `Display` (English, with the surface id) is for the logs.
    ///
    /// # Errors
    ///
    /// Fails if the surface is unknown or expired, if the action or its target does not exist,
    /// or if the session or dictation refuses the change.
    pub fn act(&self, action: SurfaceAction) -> Result<(), HubError> {
        let ask = self.state().take_ask_id(&action.surface);
        if let Some(ask) = ask {
            let answer = if action.name == "dismiss" {
                Answer::Cancelled
            } else {
                Answer::Action { name: action.name, context: action.context }
            };
            let _ = ask.tx.send(answer);
            return Ok(());
        }
        // La boîte des réglages reste montrée pendant le clic : un rafraîchissement n'est jamais perdu.
        if action.surface == HOST_VOICE && action.name != "dismiss" {
            return self.apply_settings(&action.context, &action.name);
        }
        let Some(shown) = self.state().shows.remove(&action.surface) else {
            return Err(HubError::unknown_surface(action.surface));
        };
        // Un échec (valeur refusée, slide disparue) ne doit pas faire perdre
        // la surface : elle est ré-insérée dans tous les cas d'erreur, pour
        // que l'utilisateur puisse corriger ou la fermer explicitement.
        let result = match (&shown, action.name.as_str()) {
            (Shown::Params(_), "dismiss") => Ok(After::Freeze),
            (Shown::Themes(picker), name) if name == oku_slides_session::THEME_EVENT => {
                self.apply_theme(picker, &action.context).map(|p| After::Replace(Shown::Themes(p)))
            }
            (Shown::Packs(_), name) if name != "dismiss" => {
                self.apply_packs(&action.context, name).map(|p| After::Replace(Shown::Packs(p)))
            }
            (Shown::Params(spec), "param") => {
                self.apply_param(&action.surface, spec, &action.context).map(|()| After::Keep { changed: true })
            }
            (Shown::Variants(spec), "variant") => {
                self.apply_variant(spec, &action.context).map(|()| After::Close { changed: true })
            }
            (_, "dismiss") => Ok(After::Close { changed: false }),
            (Shown::Static { expired: false, .. }, "suggest") => Ok(After::Keep { changed: false }),
            (Shown::Static { expired: true, .. }, "suggest") => ExpiredSnafu.fail(),
            (_, other) => UnexpectedActionSnafu { action: other }.fail(),
        };
        match result {
            Ok(After::Keep { changed }) => {
                self.state().shows.insert(action.surface, shown);
                if changed {
                    self.prune();
                }
                Ok(())
            }
            Ok(After::Replace(next)) => {
                self.state().shows.insert(action.surface, next);
                // Le deck a changé : l'autre boîte du hub (thème, packs) se relit ; celle-ci, déjà
                // refaite, est égale à elle-même et ne part pas une seconde fois.
                self.prune();
                Ok(())
            }
            Ok(After::Freeze) => {
                self.freeze(&action.surface);
                Ok(())
            }
            Ok(After::Close { changed }) => {
                self.emit(McpEvent::CloseSurface { id: action.surface });
                if changed {
                    self.prune();
                }
                Ok(())
            }
            Err(e) => {
                self.state().shows.insert(action.surface, shown);
                Err(e.into())
            }
        }
    }

    pub(super) fn index(context: &Map<String, Value>, key: &str) -> Result<usize, super::error::HubInner> {
        context.get(key).and_then(Value::as_u64).and_then(|i| usize::try_from(i).ok()).context(IncompleteSnafu { key })
    }

    fn apply_param(
        &self,
        id: &str,
        spec: &ShowParams,
        context: &Map<String, Value>,
    ) -> Result<(), super::error::HubInner> {
        let i = Self::index(context, "field")?;
        let field = spec.fields.get(i).context(NoFieldSnafu { index: i })?;
        let value = context.get("value").cloned().unwrap_or(Value::Null);
        self.session
            .with(|s| -> oku_slides_session::Result<()> {
                let op = s.param_op(field, &value)?;
                let label =
                    self.localizer.current().tr_with("agent-undo-param", &[("label", field.label.as_str().into())]);
                s.apply(&label, vec![op], None)?;
                s.note(format!("parameter: {:?} = {value} ({})", field.label, spec.slide));
                Ok(())
            })
            .and_then(|r| r)
            .context(SessionSnafu)?;
        // L'interface a déjà la valeur ; la copie du hub la garde pour une resynchronisation.
        if let Some(open) = self.state().open.iter_mut().find(|s| s.id == id) {
            open.set_value(&forms::param_key(i), &value);
        }
        self.emit(McpEvent::Changed);
        Ok(())
    }

    fn apply_variant(&self, spec: &ShowVariants, context: &Map<String, Value>) -> Result<(), super::error::HubInner> {
        let i = Self::index(context, "index")?;
        let variant = spec.variants.get(i).context(NoVariantSnafu { index: i })?;
        self.session
            .with(|s| -> oku_slides_session::Result<()> {
                let label =
                    self.localizer.current().tr_with("agent-undo-variant", &[("label", variant.label.as_str().into())]);
                s.apply(&label, variant.ops.clone(), None)?;
                s.note(format!("variant picked: {:?} ({})", variant.label, spec.slide));
                Ok(())
            })
            .and_then(|r| r)
            .context(SessionSnafu)?;
        self.emit(McpEvent::Changed);
        Ok(())
    }

    /// Rafraîchit les boîtes du hub (thème, packs) et ferme les surfaces orphelines.
    ///
    /// Prend la session si l'un des deux est ouvert ou si un réglage vivant
    /// (`Params`, `Variants`) existe ; sinon n'y touche pas. Verrouille la session : à lancer
    /// hors du runtime asynchrone (appelée après chaque requête qui change le deck).
    pub fn prune(&self) {
        self.refresh_themes();
        self.refresh_packs();
        if !self.state().shows.values().any(|s| matches!(s, Shown::Params(_) | Shown::Variants(_))) {
            return;
        }
        let Ok(gone) = self.session.with(|s| {
            let deck = s.deck();
            let mut state = self.state();
            let gone: Vec<String> = state
                .shows
                .iter()
                .filter(|(_, shown)| {
                    let slide = match shown {
                        Shown::Params(p) => &p.slide,
                        Shown::Variants(v) => &v.slide,
                        Shown::Static { .. } | Shown::Themes(_) | Shown::Packs(_) | Shown::Settings(_) => {
                            return false;
                        }
                    };
                    deck.slide(slide).is_none()
                })
                .map(|(id, _)| id.clone())
                .collect();
            for id in &gone {
                state.shows.remove(id);
            }
            gone
        }) else {
            return;
        };
        for id in gone {
            self.emit(McpEvent::CloseSurface { id });
        }
    }
}
