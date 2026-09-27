//! Langues d'Oku Slides : les messages de l'app, en Fluent.
//!
//! - [`Localizer`] : les messages d'une langue, avec l'anglais en secours
//!   pour un message qui manquerait.
//! - [`Localize`] : ce qui sait se dire dans une langue (erreurs, rapports).
//! - [`english`] : l'anglais de référence, pour les journaux et `Display`.
//! - [`Message`] : un message à dire plus tard (dans une erreur).
//!
//! Les fichiers `.ftl` sont dans `src/locales/<langue>/`, un par domaine ;
//! `en` est la référence, chaque autre langue en reprend toutes les clés.

mod message;

use std::sync::LazyLock;

pub use message::{Arg, Message};

use fluent_bundle::FluentResource;
use fluent_bundle::concurrent::FluentBundle;
pub use fluent_bundle::{FluentArgs, FluentValue};
use unic_langid::LanguageIdentifier;

macro_rules! locale {
    ($lang:literal) => {
        (
            $lang,
            &[
                include_str!(concat!("locales/", $lang, "/core.ftl")),
                include_str!(concat!("locales/", $lang, "/document.ftl")),
                include_str!(concat!("locales/", $lang, "/session.ftl")),
                include_str!(concat!("locales/", $lang, "/stt.ftl")),
                include_str!(concat!("locales/", $lang, "/cli.ftl")),
                include_str!(concat!("locales/", $lang, "/chart.ftl")),
                include_str!(concat!("locales/", $lang, "/agent.ftl")),
                // Toujours en dernier : voir `ui_messages`.
                include_str!(concat!("locales/", $lang, "/ui.ftl")),
            ] as &[&str],
        )
    };
}

/// Langues livrées, la référence d'abord.
const LOCALES: &[(&str, &[&str])] = &[
    locale!("en"),
    locale!("fr"),
    locale!("zh-Hans"),
    locale!("zh-Hant"),
    locale!("es"),
    locale!("ar"),
    locale!("hi"),
    locale!("bn"),
    locale!("pt"),
    locale!("ru"),
    locale!("ja"),
    locale!("de"),
    locale!("ko"),
    locale!("it"),
    locale!("tr"),
    locale!("vi"),
    locale!("pl"),
    locale!("uk"),
    locale!("nl"),
    locale!("th"),
];

/// Langue de référence : chaque message y existe.
pub const REFERENCE: &str = "en";

type Bundle = FluentBundle<FluentResource>;

/// Les messages d'une langue.
pub struct Localizer {
    locale: &'static str,
    bundle: Bundle,
    /// `ui.ftl` de cette langue, pour l'interface.
    ui: &'static str,
    /// Anglais, pour un message absent de la langue choisie.
    fallback: Option<Box<Localizer>>,
}

impl std::fmt::Debug for Localizer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Localizer").field("locale", &self.locale).finish_non_exhaustive()
    }
}

/// Ce qui sait se dire dans la langue de l'utilisateur.
///
/// Implémentez ce trait pour une erreur ou un rapport montré à
/// l'utilisateur ; `Display` reste l'anglais des journaux.
pub trait Localize {
    fn localize(&self, l: &Localizer) -> String;
}

static ENGLISH: LazyLock<Localizer> = LazyLock::new(|| Localizer::build(REFERENCE, None));

/// L'anglais de référence.
pub fn english() -> &'static Localizer {
    &ENGLISH
}

impl Localizer {
    /// La langue demandée (réglages) si elle est livrée, sinon celle du
    /// système, sinon l'anglais. `fr-CA` donne `fr`.
    pub fn new(requested: Option<&str>) -> Self {
        let system = sys_locale::get_locale();
        let chosen = requested.and_then(supported).or_else(|| system.as_deref().and_then(supported));
        match chosen {
            Some(locale) if locale != REFERENCE => Self::build(locale, Some(Box::new(Self::build(REFERENCE, None)))),
            _ => Self::build(REFERENCE, None),
        }
    }

    fn build(locale: &'static str, fallback: Option<Box<Localizer>>) -> Self {
        let id: LanguageIdentifier = locale.parse().unwrap_or_default();
        let mut bundle = Bundle::new_concurrent(vec![id]);
        // Pas de marques d'isolement bidirectionnel autour des valeurs :
        // les chemins et nombres s'impriment tels quels dans un terminal.
        bundle.set_use_isolating(false);
        let sources = LOCALES.iter().find(|(l, _)| *l == locale).map_or(&[] as &[&str], |(_, s)| s);
        // `locale` vient de LOCALES, dont chaque entrée finit par ui.ftl.
        let ui = sources.last().copied().unwrap_or_default();
        for source in sources {
            // Une ressource mal formée garde ses messages valides (les tests
            // vérifient qu'il n'y en a pas).
            let resource = FluentResource::try_new((*source).to_owned()).unwrap_or_else(|(partial, _)| partial);
            let _ = bundle.add_resource(resource);
        }
        Self { locale, bundle, ui, fallback }
    }

    /// Messages de l'interface (`ui.ftl`), pour son propre moteur Fluent :
    /// l'anglais de référence d'abord, puis la langue choisie.
    pub fn ui_messages(&self) -> Vec<&'static str> {
        match &self.fallback {
            Some(english) => vec![english.ui, self.ui],
            None => vec![self.ui],
        }
    }

    /// Code de la langue (`en`, `fr`).
    pub fn locale(&self) -> &'static str {
        self.locale
    }

    /// Un message sans paramètre.
    pub fn tr(&self, id: &str) -> String {
        self.format(id, None)
    }

    /// Un message et ses paramètres : `l.tr_with("cli-deck-valid", &[("slides", 8.into())])`.
    pub fn tr_with(&self, id: &str, args: &[(&str, FluentValue<'_>)]) -> String {
        let mut fluent = FluentArgs::with_capacity(args.len());
        for (name, value) in args {
            fluent.set(*name, value.clone());
        }
        self.format(id, Some(&fluent))
    }

    fn format(&self, id: &str, args: Option<&FluentArgs<'_>>) -> String {
        if let Some(pattern) = self.bundle.get_message(id).and_then(|m| m.value()) {
            let mut errors = Vec::new();
            return self.bundle.format_pattern(pattern, args, &mut errors).into_owned();
        }
        match &self.fallback {
            Some(fallback) => fallback.format(id, args),
            // Un identifiant inconnu se voit tel quel plutôt que de disparaître.
            None => id.to_owned(),
        }
    }
}

/// La langue livrée qui correspond à `tag` (`fr-FR` → `fr`, `zh-TW` → `zh-Hant`).
fn supported(tag: &str) -> Option<&'static str> {
    let wanted = candidate(tag);
    LOCALES.iter().map(|(l, _)| *l).find(|l| *l == wanted)
}

/// Code de langue cherché pour `tag` : la langue seule, sauf le chinois,
/// livré en écriture simplifiée et traditionnelle (Taïwan, Hong Kong, Macao).
fn candidate(tag: &str) -> String {
    let lower = tag.replace('_', "-").to_ascii_lowercase();
    let mut parts = lower.split('-');
    let language = parts.next().unwrap_or_default();
    if language != "zh" {
        return language.to_owned();
    }
    let traditional = parts.any(|p| matches!(p, "hant" | "tw" | "hk" | "mo"));
    if traditional { "zh-Hant" } else { "zh-Hans" }.to_owned()
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod lib_tests;
