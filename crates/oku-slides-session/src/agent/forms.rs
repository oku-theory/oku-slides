//! Formes de surface de l'agent : arguments des outils `ask_*` / `show_*`
//! et leur composition en messages A2UI (catalogue v3).
//!
//! Actions renvoyées par l'interface : `choose {choice}`, `confirm
//! {confirmed}`, `permission {index}`, `rewrite {kept, text}`, `param
//! {field, value}` (`param {field, change}` pour une liste), `variant
//! {index}`, `dismiss`, `answer {text}` (réponse dictée à une question en
//! attente).

use oku_slides_a2ui::{
    Action, ChoiceOption, ColumnKind, Component, ComponentKind, Dynamic, ListColumn, ServerBody, ServerMessage,
    TextVariant,
};
use oku_slides_core::{Op, Paragraph};
use oku_slides_i18n::Localizer;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::text::paragraphs_to_text;

/// Où l'interface place une surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Surface {
    /// A2UI id of the surface, unique within the session.
    pub id: String,
    /// Where the interface displays it.
    pub place: Place,
    /// Element to anchor a card under; without it, no anchor.
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
    /// A2UI messages that build the surface, in sending order.
    pub messages: Vec<ServerMessage>,
}

impl Surface {
    /// Change une valeur racine du modèle de données (`f0`, `choice`…).
    ///
    /// Sert à garder la copie de l'hôte à jour quand l'interface a déjà
    /// changé la sienne (un réglage de surface), pour une resynchronisation.
    pub fn set_value(&mut self, key: &str, value: &Value) {
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

/// The field or element under which to show a card.
///
/// It comes from the agent: the session resolves it against the active deck
/// (`Session::check_anchor`) before any surface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Anchor {
    /// Id of the slide that holds the targeted object.
    pub slide: String,
    /// The targeted object: an element of this slide (its id), or the slide itself.
    ///
    /// Give the slide's own id for one of its fields.
    pub owner: String,
    /// A text parameter of `owner` (see `widgets`), by its name.
    ///
    /// Required when `owner` is the slide. Without it: the whole element.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

/// `ask_choice`: a question and its options; the tool returns the chosen option.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
pub struct AskChoice {
    /// The question, short, in the user's language.
    pub question: String,
    /// 2 to 6 options.
    pub options: Vec<ChoiceOption>,
    /// Element concerned: the question shows under it; otherwise in the thread.
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

/// `ask_confirm`: yes or no.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
pub struct AskConfirm {
    /// The question asked, in the user's language.
    pub message: String,
    /// Destructive action: the button says so.
    #[serde(default)]
    pub danger: bool,
}

/// `ask_rewrite`: 2 or 3 rewrites of a text; the one picked is applied.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
pub struct AskRewrite {
    /// Id of the slide that holds the text.
    pub slide: String,
    /// The object that holds the text: an element of this slide (its id), or the slide itself.
    ///
    /// Give the slide's own id for one of its fields.
    pub owner: String,
    /// The text parameter of `owner` to rewrite (see `widgets`).
    ///
    /// Required when `owner` is the slide. Without it: the element's first `rich` parameter.
    #[serde(default)]
    pub field: Option<String>,
    /// 2 or 3 proposals; one line per paragraph, `- ` for a bullet.
    pub proposals: Vec<String>,
}

/// Kind of a `show_params` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FieldKind {
    /// Integer set with buttons, between rounded `min` and `max` (1 and 24 by default).
    Stepper,
    /// Number set with a slider between `min` and `max`, in steps of `step`.
    Slider,
    /// Hexadecimal color, with the theme palette as shortcuts.
    Color,
    /// One value among `options`.
    Select,
    /// Plain text, without formatting.
    Text,
    /// Rich text: paragraphs, shown as editing markdown.
    Rich,
    /// List of items, edited as a table: `value` = items.
    ///
    /// Host settings only: absent from the tools' schema, refused by `show_params`.
    #[schemars(skip)]
    List,
}

/// A `show_params` field and the op it applies.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
pub struct ParamField {
    /// Field label, in the user's language.
    pub label: String,
    /// Kind of field, which sets its control.
    pub kind: FieldKind,
    /// Current value.
    pub value: Value,
    /// A deck op where the string `"$value"` is replaced by the chosen value.
    pub op_template: Value,
    /// Lower bound of a `stepper` or a `slider`.
    ///
    /// When absent, it is 1 for a `stepper` and 0 for a `slider`.
    #[serde(default)]
    pub min: Option<f64>,
    /// Upper bound of a `stepper` or a `slider`.
    ///
    /// When absent, it is 24 for a `stepper` and 100 for a `slider`.
    #[serde(default)]
    pub max: Option<f64>,
    /// Step of a `slider`; ignored elsewhere.
    #[serde(default)]
    pub step: Option<f64>,
    /// Options of a `select` field.
    #[serde(default)]
    pub options: Vec<ChoiceOption>,
    /// Columns of a `list` field (host settings).
    #[serde(default)]
    #[schemars(skip)]
    pub columns: Vec<ListColumn>,
    /// Name of one item of a `list` field ("step"); without it, the generic name.
    #[serde(default)]
    #[schemars(skip)]
    pub item_label: Option<String>,
}

/// `show_params`: a surface of settings that apply at once.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
pub struct ShowParams {
    /// Surface title, in the user's language.
    pub title: String,
    /// Slide being set: the surface goes away if the slide is deleted.
    pub slide: String,
    /// 1 to 8 fields.
    pub fields: Vec<ParamField>,
}

/// A variant: a label and the ops that produce it.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
pub struct Variant {
    /// Short label of the variant, in the user's language.
    pub label: String,
    /// Deck ops applied to the slide to produce the variant.
    pub ops: Vec<Op>,
}

/// `show_variants`: 2 to 4 rendered variants of a slide; the chosen one is applied.
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
pub struct ShowVariants {
    /// Id of the slide the variants derive from.
    pub slide: String,
    /// 2 to 4 variants of this slide.
    pub variants: Vec<Variant>,
}

/// Bounds `(min, max)` on the number of `ask_choice` options.
pub const CHOICES: (usize, usize) = (2, 6);
/// Bounds `(min, max)` on the number of `ask_rewrite` proposals.
pub const PROPOSALS: (usize, usize) = (2, 3);
/// Bounds `(min, max)` on the number of `show_variants` variants.
pub const VARIANTS: (usize, usize) = (2, 4);
/// Bounds `(min, max)` on the number of `show_params` fields.
pub const FIELDS: (usize, usize) = (1, 8);
/// Plafond du nombre d'options d'un champ `select` (large : une liste vide reste permise).
pub const SELECT_OPTIONS_MAX: usize = 100;
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

/// Surface `id` des données d'un graphique ou d'un tableau : un titre, puis `data`
/// (`ChartData` ou `TableData`), dont la modification s'écrit en `/edit`.
pub(crate) fn data_surface(id: &str, title: String, data: ComponentKind) -> Surface {
    let components = vec![
        Component::column("root", &["title", "data"]),
        Component::text("title", title, Some(TextVariant::H4)),
        Component::new("data", data),
    ];
    surface(id, Place::Thread, None, components, json!({ "edit": null }))
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
    let anchor = Anchor { slide: spec.slide.clone(), owner: spec.owner.clone(), field: spec.field.clone() };
    let draft = spec.proposals.first().cloned().unwrap_or_default();
    question(surface(id, Place::Sheet, Some(anchor), components, json!({ "draft": draft })))
}

/// Clé du champ `i` d'une surface `show_params` dans son modèle de données.
#[must_use]
pub fn param_key(i: usize) -> String {
    format!("f{i}")
}

/// Extras des réglages de l'hôte : clés des champs, lignes en lecture seule, « × ».
pub(crate) struct FormExtras<'a> {
    /// Clé de chaque champ, même ordre que les champs ; vide : pas de clé.
    pub keys: &'a [String],
    /// Lignes en légende, après les champs.
    pub notes: &'a [String],
    /// Bouton « × » (`dismiss`).
    pub closable: bool,
}

/// `show_params` : surface ; chaque champ `f<i>` envoie `param {field, value}`.
pub(crate) fn params(id: &str, spec: &ShowParams, palette: &[String], l: &Localizer) -> Surface {
    params_with(id, spec, &FormExtras { keys: &[], notes: &[], closable: true }, palette, l)
}

/// `show_params` avec les extras de l'hôte : l'action d'un champ porte aussi
/// `key` quand `extras.keys` en donne une.
///
/// Ne lit ni `spec.slide` ni les `op_template` : les réglages de l'hôte en ont des vides.
pub(crate) fn params_with(
    id: &str,
    spec: &ShowParams,
    extras: &FormExtras<'_>,
    palette: &[String],
    l: &Localizer,
) -> Surface {
    let mut body: Vec<String> = vec!["title".into()];
    let mut components = vec![Component::text("title", spec.title.clone(), Some(TextVariant::H4))];
    let mut data = serde_json::Map::new();
    for (i, f) in spec.fields.iter().enumerate() {
        let key = param_key(i);
        let mut context = vec![("field", lit(json!(i)))];
        if let Some(k) = extras.keys.get(i) {
            context.push(("key", lit(json!(k))));
        }
        let (component, entries) = field_components(&key, f, &FieldEvent { name: "param", context }, palette);
        components.push(component);
        data.extend(entries);
        body.push(key);
    }
    for (i, note) in extras.notes.iter().enumerate() {
        let nid = format!("note-{i}");
        components.push(Component::text(&nid, note.clone(), Some(TextVariant::Caption)));
        body.push(nid);
    }
    if extras.closable {
        components.push(Component::button("close", l.tr("agent-surface-close"), false, Action::named("dismiss")));
        body.push("close".into());
    }
    let refs: Vec<&str> = body.iter().map(String::as_str).collect();
    components.insert(0, Component::column("body", &refs));
    components.insert(0, Component::card("root", "body"));
    surface(id, Place::Thread, None, components, Value::Object(data))
}

/// L'évènement d'un champ et son contexte fixe ; la valeur s'y ajoute sous
/// `value` (sous `change` pour une liste).
pub(crate) struct FieldEvent<'a> {
    pub name: &'a str,
    pub context: Vec<(&'a str, Dynamic<Value>)>,
}

impl FieldEvent<'_> {
    /// L'action qui envoie le chemin `path` du modèle sous `entry`.
    fn action(&self, entry: &'static str, path: &str) -> Action {
        let mut context = self.context.clone();
        context.push((entry, Dynamic::path(path)));
        Action::event(self.name, context)
    }
}

/// Le composant `key` d'un champ et ses entrées du modèle de données : sa
/// valeur sous `key`, plus les entrées annexes d'un texte riche ou d'une liste.
pub(crate) fn field_components(
    key: &str,
    f: &ParamField,
    event: &FieldEvent<'_>,
    palette: &[String],
) -> (Component, serde_json::Map<String, Value>) {
    let mut data = serde_json::Map::new();
    let path = format!("/{key}");
    let mut markdown: Option<String> = None;
    let action = Some(event.action("value", &path));
    let label = Dynamic::Literal(f.label.clone());
    let kind = match f.kind {
        FieldKind::Stepper => ComponentKind::Stepper {
            label,
            value: Dynamic::path(&path),
            min: f.min.map(oku_slides_core::num::round_i64),
            max: f.max.map(oku_slides_core::num::round_i64),
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
        FieldKind::Rich => {
            let source = format!("{path}-source");
            let paragraphs: Vec<Paragraph> = serde_json::from_value(f.value.clone()).unwrap_or_default();
            data.insert(format!("{key}-source"), f.value.clone());
            markdown = Some(paragraphs_to_text(&paragraphs));
            ComponentKind::RichTextField {
                label,
                value: Dynamic::path(&path),
                source: Dynamic::path(&source),
                palette: palette.to_vec(),
                action: Some(event.action("value", &source)),
            }
        }
        FieldKind::List => {
            let change = format!("{path}-change");
            data.insert(format!("{key}-change"), Value::Null);
            data.insert(format!("{key}-markdown"), rich_markdown(&f.value, &f.columns));
            ComponentKind::ListTable {
                label,
                rows: Dynamic::path(&path),
                change: Dynamic::path(&change),
                markdown: Dynamic::path(format!("{path}-markdown")),
                columns: f.columns.clone(),
                min: f.min.map_or(0, |m| oku_slides_core::num::trunc_u32(m.max(0.0))),
                max: f.max.map(|m| oku_slides_core::num::trunc_u32(m.max(0.0))),
                item_label: f.item_label.clone(),
                palette: palette.to_vec(),
                action: Some(event.action("change", &change)),
            }
        }
    };
    data.insert(key.to_owned(), markdown.map_or_else(|| f.value.clone(), Value::from));
    (Component::new(key, kind), data)
}

/// Markdown d'édition des cellules `rich` des items `rows` : `{id: {colonne: markdown}}` ;
/// ceux d'une liste imbriquée sous la clé `id/colonne/sous-id`.
fn rich_markdown(rows: &Value, columns: &[ListColumn]) -> Value {
    let mut out = serde_json::Map::new();
    let rich = columns.iter().filter(|c| c.kind == ColumnKind::Rich).map(|c| c.key.as_str());
    cells_markdown(rows, &rich.collect::<Vec<_>>(), "", &mut out);
    for row in rows.as_array().into_iter().flatten() {
        let Some(id) = row.get("id").and_then(Value::as_str) else { continue };
        // Liste imbriquée (un niveau) : ses items, sous `id/colonne/`.
        for c in columns.iter().filter(|c| c.kind == ColumnKind::List) {
            if let (Some(sub), Some(nested)) = (&c.list, row.get(&c.key)) {
                let rich = sub.columns.iter().filter(|s| s.kind == ColumnKind::Rich).map(|s| s.key.as_str());
                cells_markdown(nested, &rich.collect::<Vec<_>>(), &format!("{id}/{}/", c.key), &mut out);
            }
        }
    }
    Value::Object(out)
}

/// Markdown des colonnes riches `rich` des items `rows`, aux clés préfixées de `prefix`.
fn cells_markdown(rows: &Value, rich: &[&str], prefix: &str, out: &mut serde_json::Map<String, Value>) {
    for row in rows.as_array().into_iter().flatten() {
        let Some(id) = row.get("id").and_then(Value::as_str) else { continue };
        let cells: serde_json::Map<String, Value> = rich
            .iter()
            .filter_map(|&k| {
                let paragraphs: Vec<Paragraph> = serde_json::from_value(row.get(k)?.clone()).ok()?;
                Some((k.to_owned(), Value::from(paragraphs_to_text(&paragraphs))))
            })
            .collect();
        if !cells.is_empty() {
            out.insert(format!("{prefix}{id}"), Value::Object(cells));
        }
    }
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
