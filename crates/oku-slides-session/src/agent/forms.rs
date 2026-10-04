//! Formes de surface de l'agent : arguments des outils `ask_*` / `show_*`
//! et leur composition en messages A2UI (catalogue v3).
//!
//! Actions renvoyées par l'interface : `choose {choice}`, `confirm
//! {confirmed}`, `permission {index}`, `rewrite {kept, text}`, `param
//! {field, value}`, `variant {index}`, `dismiss`, `answer
//! {text}` (réponse dictée à une question en attente).

use oku_slides_a2ui::{
    Action, ChoiceOption, Component, ComponentKind, Dynamic, ServerBody, ServerMessage, TextVariant,
};
use oku_slides_core::Op;
use oku_slides_i18n::Localizer;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Où l'interface place une surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Place {
    /// Fiche ancrée sous un élément de la slide.
    Sheet,
    /// Dans le fil de l'agent : dans son tour, ou au dock si vivante.
    Thread,
    /// Tiroir sous la scène (variantes).
    Drawer,
    /// Fenêtre modale (confirmation, permission).
    Modal,
}

/// Une surface prête à afficher : ses messages A2UI et sa place.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Surface {
    pub id: String,
    pub place: Place,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor: Option<Anchor>,
    /// Le tour qui l'a créée (surfaces du fil) ; le hub la remplit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn: Option<u32>,
    /// Vivante : dans le dock, elle attend ou agit encore ; figée : dans son tour.
    pub live: bool,
    /// Une question (`ask_*`, confirmation, permission) : elle attend la réponse de
    /// l'utilisateur. Sa version figée (question répondue) n'en est plus une.
    pub question: bool,
    pub messages: Vec<ServerMessage>,
}

impl Surface {
    /// Change une valeur racine du modèle de données (`f0`, `choice`…).
    ///
    /// Sert à garder la copie de l'hôte à jour quand l'interface a déjà
    /// changé la sienne (un réglage de surface), pour une resynchronisation.
    pub fn set_value(&mut self, key: &str, value: Value) {
        for m in &mut self.messages {
            if let ServerBody::UpdateDataModel(u) = &mut m.body
                && u.path.is_none()
                && let Some(Value::Object(root)) = &mut u.value
            {
                root.insert(key.to_owned(), value.clone());
            }
        }
    }
}

/// L'élément sous lequel afficher une fiche.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Anchor {
    pub slide: String,
    /// Identifiant d'élément, ou champ de la slide : `@title`, `@subtitle`, `@kicker`, `@text`.
    pub element: String,
}

/// `ask_choice` : une question et ses options ; l'outil renvoie l'option choisie.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
pub struct AskChoice {
    /// La question, courte, dans la langue de l'utilisateur.
    pub question: String,
    /// 2 à 6 options.
    pub options: Vec<ChoiceOption>,
    /// Élément concerné : la question s'affiche dessous ; sinon dans le fil.
    #[serde(default)]
    pub anchor: Option<Anchor>,
}

/// Une question du fil, répondue ou annulée : sa question et la réponse, figées.
pub fn frozen_choice(id: &str, question: &str, answer: Option<&str>, l: &Localizer) -> Surface {
    let said = match answer {
        Some(a) => l.tr_with("agent-choice-answered", &[("answer", a.into())]),
        None => l.tr("agent-choice-cancelled"),
    };
    let components = vec![
        Component::card("root", "body"),
        Component::column("body", &["question", "answer"]),
        Component::text("question", question.to_owned(), Some(TextVariant::Caption)),
        Component::text("answer", said, Some(TextVariant::Body)),
    ];
    let mut s = surface(id, Place::Thread, None, components, json!({}));
    s.live = false;
    s
}

/// `ask_confirm` : oui ou non.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
pub struct AskConfirm {
    pub message: String,
    /// Action destructive : le bouton le dit.
    #[serde(default)]
    pub danger: bool,
}

/// `ask_rewrite` : 2 ou 3 reformulations d'un texte ; celle retenue est appliquée.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
pub struct AskRewrite {
    pub slide: String,
    /// Élément texte (ou carte), ou `@title`, `@subtitle`, `@kicker`, `@text`.
    pub element: String,
    /// 2 ou 3 propositions ; une ligne par paragraphe, `- ` pour une puce.
    pub proposals: Vec<String>,
}

/// Nature d'un champ de `show_params`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FieldKind {
    Stepper,
    Slider,
    Color,
    Select,
    Text,
}

/// Un champ de `show_params` et l'op qu'il applique.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
pub struct ParamField {
    pub label: String,
    pub kind: FieldKind,
    /// Valeur actuelle.
    pub value: Value,
    /// Une op du deck où la chaîne `"$value"` sera remplacée par la valeur choisie.
    pub op_template: Value,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub step: Option<f64>,
    /// Options d'un champ `select`.
    #[serde(default)]
    pub options: Vec<ChoiceOption>,
}

/// `show_params` : une surface de réglages qui s'appliquent tout de suite.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
pub struct ShowParams {
    pub title: String,
    /// Slide réglée : la surface disparaît si elle est supprimée.
    pub slide: String,
    /// 1 à 8 champs.
    pub fields: Vec<ParamField>,
}

/// Une variante : un libellé et les ops qui la produisent.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
pub struct Variant {
    pub label: String,
    pub ops: Vec<Op>,
}

/// `show_variants` : 2 à 4 variantes d'une slide, rendues ; la choisie est appliquée.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
pub struct ShowVariants {
    pub slide: String,
    pub variants: Vec<Variant>,
}

/// Bornes du nombre d'options, de propositions, de variantes et de champs.
pub const CHOICES: (usize, usize) = (2, 6);
pub const PROPOSALS: (usize, usize) = (2, 3);
pub const VARIANTS: (usize, usize) = (2, 4);
pub const FIELDS: (usize, usize) = (1, 8);
/// Bornes du nombre d'opérations d'une variante (contre un lot démesuré venant de l'agent).
pub const OPS: (usize, usize) = (1, 50);
/// Bornes par défaut d'un `Slider` sans `min`/`max`. Partagées avec
/// `variants::check_field`, pour valider les mêmes valeurs que celles rendues.
pub(crate) const SLIDER_DEFAULT_MIN: f64 = 0.0;
pub(crate) const SLIDER_DEFAULT_MAX: f64 = 100.0;

/// Surface : création, composants, modèle de données.
pub(crate) fn surface(
    id: &str,
    place: Place,
    anchor: Option<Anchor>,
    components: Vec<Component>,
    data: Value,
) -> Surface {
    Surface {
        id: id.to_owned(),
        place,
        anchor,
        turn: None,
        live: true,
        question: false,
        messages: vec![
            ServerMessage::create_surface(id),
            ServerMessage::update_components(id, components),
            ServerMessage::set_data_model(id, data),
        ],
    }
}

/// `s`, marquée comme une question (elle attend la réponse de l'utilisateur).
fn question(mut s: Surface) -> Surface {
    s.question = true;
    s
}

pub(crate) fn lit(v: Value) -> Dynamic<Value> {
    Dynamic::Literal(v)
}

/// `ask_choice` : pastilles ; fiche si ancrée, sinon dans le fil.
pub fn choice(id: &str, spec: &AskChoice, l: &Localizer) -> Surface {
    let components = vec![
        Component::card("root", "body"),
        Component::column("body", &["question", "options", "cancel"]),
        Component::text("question", spec.question.clone(), Some(TextVariant::H4)),
        Component::new(
            "options",
            ComponentKind::ChoiceChips {
                options: spec.options.clone(),
                value: Dynamic::path("/choice"),
                action: Some(Action::event("choose", [("choice", Dynamic::path("/choice"))])),
            },
        ),
        Component::button("cancel", l.tr("agent-surface-cancel"), false, Action::named("dismiss")),
    ];
    let place = if spec.anchor.is_some() { Place::Sheet } else { Place::Thread };
    question(surface(id, place, spec.anchor.clone(), components, json!({ "choice": "" })))
}

/// `ask_confirm` : fenêtre modale, confirmer ou annuler.
pub fn confirm(id: &str, spec: &AskConfirm, l: &Localizer) -> Surface {
    let yes = if spec.danger { "agent-surface-confirm-danger" } else { "agent-surface-confirm" };
    let answer = |v: bool| Action::event("confirm", [("confirmed", lit(json!(v)))]);
    let components = vec![
        Component::card("root", "body"),
        Component::column("body", &["message", "buttons"]),
        Component::text("message", spec.message.clone(), Some(TextVariant::Body)),
        Component::row("buttons", &["yes", "no"]),
        Component::button("yes", l.tr(yes), true, answer(true)).danger(spec.danger),
        Component::button("no", l.tr("agent-surface-cancel"), false, answer(false)),
    ];
    question(surface(id, Place::Modal, None, components, json!({})))
}

/// La nature d'une option de permission (ACP `PermissionOptionKind`).
///
/// Seule « autoriser une fois » peut être le bouton principal. « Toujours
/// autoriser » et une nature inconnue sont marquées comme engageantes ;
/// refuser, même pour toujours, n'accorde rien.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionKind {
    /// Autoriser cette fois seulement.
    AllowOnce,
    /// Autoriser et s'en souvenir.
    AllowAlways,
    /// Refuser cette fois.
    RejectOnce,
    /// Refuser et s'en souvenir.
    RejectAlways,
    /// Une nature que cette version ne connaît pas : traitée comme engageante
    /// (elle pourrait autoriser durablement). À revoir quand le schéma ACP change.
    Other,
}

impl PermissionKind {
    /// Rang d'affichage : la plus sûre d'abord, les plus engageantes en dernier
    /// (Tab ne tombe pas d'abord sur « toujours autoriser »).
    fn rank(self) -> u8 {
        match self {
            Self::AllowOnce => 0,
            Self::RejectOnce => 1,
            Self::RejectAlways => 2,
            Self::Other => 3,
            Self::AllowAlways => 4,
        }
    }

    fn engages(self) -> bool {
        matches!(self, Self::AllowAlways | Self::Other)
    }
}

/// Une option de permission, telle que l'agent la propose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionChoice {
    /// Libellé, écrit par l'agent.
    pub name: String,
    /// Nature ACP : elle seule décide de l'ordre, du bouton principal et du style.
    pub kind: PermissionKind,
}

/// Permission demandée par l'agent (ACP) : un bouton par option (Échap : `dismiss`).
///
/// Les libellés viennent de l'agent ; le reste non. Les boutons sont rangés du
/// plus sûr au plus engageant ; le principal est la première option
/// « autoriser une fois » (aucun s'il n'y en a pas) ; « toujours autoriser »
/// est marquée engageante et reçoit une mention traduite par l'app (« pour
/// toujours »), pour qu'un libellé trompeur de l'agent se voie. Chaque bouton
/// renvoie l'index de l'option dans l'ordre de l'agent.
pub fn permission(id: &str, title: &str, options: &[PermissionChoice], l: &Localizer) -> Surface {
    let mut order: Vec<usize> = (0..options.len()).collect();
    order.sort_by_key(|&i| options[i].kind.rank());
    let primary = options.iter().position(|o| o.kind == PermissionKind::AllowOnce);
    let ids: Vec<String> = order.iter().map(|i| format!("option-{i}")).collect();
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let title = if title.trim().is_empty() { l.tr("agent-surface-permission-title") } else { title.to_owned() };
    let mut components = vec![
        Component::card("root", "body"),
        Component::column("body", &["title", "buttons"]),
        Component::text("title", title, Some(TextVariant::H4)),
        Component::row("buttons", &refs),
    ];
    for (&i, bid) in order.iter().zip(&ids) {
        let option = &options[i];
        let label = if option.kind == PermissionKind::AllowAlways {
            format!("{} · {}", option.name, l.tr("agent-surface-permission-always"))
        } else {
            option.name.clone()
        };
        let action = Action::event("permission", [("index", lit(json!(i)))]);
        components.push(Component::button(bid, label, primary == Some(i), action).danger(option.kind.engages()));
    }
    question(surface(id, Place::Modal, None, components, json!({})))
}

/// `ask_rewrite` : l'original et les propositions, différences surlignées.
pub(crate) fn rewrite(id: &str, spec: &AskRewrite, original: &str, l: &Localizer) -> Surface {
    let mut body: Vec<String> = vec!["title".into()];
    let mut components = vec![Component::text("title", l.tr("agent-surface-rewrite"), Some(TextVariant::H4))];
    for (i, proposal) in spec.proposals.iter().enumerate() {
        let (diff, pick) = (format!("diff-{i}"), format!("pick-{i}"));
        components.push(Component::new(
            &diff,
            ComponentKind::DiffText { segments: super::diff::diff_words(original, proposal) },
        ));
        let label = l.tr_with("agent-surface-pick", &[("n", (i + 1).into())]);
        let action = Action::event("rewrite", [("kept", lit(json!(i))), ("text", lit(json!(proposal)))]);
        components.push(Component::button(&pick, label, i == 0, action));
        body.extend([diff, pick]);
    }
    let keep = Action::event("rewrite", [("kept", lit(json!("original"))), ("text", lit(json!(original)))]);
    components.push(Component::button("keep", l.tr("agent-surface-keep"), false, keep));
    let edited = Action::event("rewrite", [("kept", lit(json!("edited"))), ("text", Dynamic::path("/draft"))]);
    components.push(Component::new(
        "edit",
        ComponentKind::TextField {
            label: Dynamic::Literal(l.tr("agent-surface-edit")),
            value: Dynamic::path("/draft"),
            action: Some(edited),
        },
    ));
    body.extend(["keep".into(), "edit".into()]);
    let refs: Vec<&str> = body.iter().map(String::as_str).collect();
    components.insert(0, Component::column("body", &refs));
    components.insert(0, Component::card("root", "body"));
    let anchor = Anchor { slide: spec.slide.clone(), element: spec.element.clone() };
    let draft = spec.proposals.first().cloned().unwrap_or_default();
    question(surface(id, Place::Sheet, Some(anchor), components, json!({ "draft": draft })))
}

/// Clé du champ `i` d'une surface `show_params` dans son modèle de données.
pub fn param_key(i: usize) -> String {
    format!("f{i}")
}

/// `show_params` : surface ; chaque champ `f<i>` envoie `param {field, value}`.
pub(crate) fn params(id: &str, spec: &ShowParams, palette: &[String], l: &Localizer) -> Surface {
    let mut body: Vec<String> = vec!["title".into()];
    let mut components = vec![Component::text("title", spec.title.clone(), Some(TextVariant::H4))];
    let mut data = serde_json::Map::new();
    for (i, f) in spec.fields.iter().enumerate() {
        let key = param_key(i);
        let path = format!("/{key}");
        let action = Some(Action::event("param", [("field", lit(json!(i))), ("value", Dynamic::path(&path))]));
        let label = Dynamic::Literal(f.label.clone());
        let kind = match f.kind {
            FieldKind::Stepper => ComponentKind::Stepper {
                label,
                value: Dynamic::path(&path),
                min: f.min.map(|v| v.round() as i64),
                max: f.max.map(|v| v.round() as i64),
                action,
            },
            FieldKind::Slider => ComponentKind::Slider {
                label,
                value: Dynamic::path(&path),
                min: f.min.unwrap_or(SLIDER_DEFAULT_MIN),
                max: f.max.unwrap_or(SLIDER_DEFAULT_MAX),
                step: f.step,
                action,
            },
            FieldKind::Color => {
                ComponentKind::ColorField { label, value: Dynamic::path(&path), palette: palette.to_vec(), action }
            }
            FieldKind::Select => {
                ComponentKind::Select { label, value: Dynamic::path(&path), options: f.options.clone(), action }
            }
            FieldKind::Text => ComponentKind::TextField { label, value: Dynamic::path(&path), action },
        };
        components.push(Component::new(&key, kind));
        data.insert(key.clone(), f.value.clone());
        body.push(key);
    }
    components.push(Component::button("close", l.tr("agent-surface-close"), false, Action::named("dismiss")));
    body.push("close".into());
    let refs: Vec<&str> = body.iter().map(String::as_str).collect();
    components.insert(0, Component::column("body", &refs));
    components.insert(0, Component::card("root", "body"));
    surface(id, Place::Thread, None, components, Value::Object(data))
}

/// `show_variants` : tiroir de miniatures ; `variant {index}` au clic.
pub(crate) fn variants(id: &str, spec: &ShowVariants, svgs: &[std::sync::Arc<str>], l: &Localizer) -> Surface {
    let ids: Vec<String> = (0..svgs.len()).map(|i| format!("v{i}")).collect();
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let mut components = vec![
        Component::column("root", &["title", "row", "close"]),
        Component::text("title", l.tr("agent-surface-variants"), Some(TextVariant::H4)),
        Component::row("row", &refs),
        Component::button("close", l.tr("agent-surface-close"), false, Action::named("dismiss")),
    ];
    for (i, (vid, (svg, v))) in ids.iter().zip(svgs.iter().zip(&spec.variants)).enumerate() {
        components.push(Component::new(
            vid,
            ComponentKind::SlideThumb {
                svg: svg.to_string(),
                label: Dynamic::Literal(format!("{} · {}", i + 1, v.label)),
                action: Some(Action::event("variant", [("index", lit(json!(i)))])),
            },
        ));
    }
    surface(id, Place::Drawer, None, components, json!({}))
}

#[cfg(test)]
#[path = "forms_tests.rs"]
mod forms_tests;
