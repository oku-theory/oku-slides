//! Langues d'Oku Slides : les messages de l'app, en Fluent.
//!
//! - [`Localizer`] : les messages d'une langue, avec l'anglais en secours
//!   pour un message qui manquerait.
//! - [`LiveLocalizer`] : la langue courante de l'interface, changée à chaud.
//! - [`Localize`] : ce qui sait se dire dans une langue (erreurs, rapports).
//! - [`resolve`] : le code de langue choisi, sans charger les messages.
//! - [`english_name`] : le nom anglais d'une langue livrée, pour l'agent.
//! - [`english`] : l'anglais de référence, pour les journaux et `Display`.
//! - [`Message`] : un message à dire plus tard (dans une erreur).
//! - [`platform_keys`], [`pc_keys`] : les raccourcis dans la notation de la plateforme.
//!
//! Les fichiers `.ftl` sont dans `src/locales/<langue>/`, un par domaine ;
//! `en` est la référence, chaque autre langue en reprend toutes les clés.

mod keys;
mod live;
mod message;

use std::sync::LazyLock;

pub use keys::{pc_keys, platform_keys};
pub use live::LiveLocalizer;
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

/// `LOCALES` et `ENGLISH_NAMES` d'une seule liste : une langue livrée a toujours son nom.
macro_rules! locales {
    ($($lang:literal => $name:literal),+ $(,)?) => {
        /// Langues livrées, la référence d'abord.
        const LOCALES: &[(&str, &[&str])] = &[$(locale!($lang)),+];
        /// Nom anglais de chaque langue livrée : l'agent le lit mieux qu'un code (`zh-Hant`).
        const ENGLISH_NAMES: &[(&str, &str)] = &[$(($lang, $name)),+];
    };
}

locales! {
    "en" => "English",
    "fr" => "French",
    "zh-Hans" => "Simplified Chinese",
    "zh-Hant" => "Traditional Chinese",
    "es" => "Spanish",
    "ar" => "Arabic",
    "hi" => "Hindi",
    "bn" => "Bengali",
    "pt" => "Portuguese",
    "ru" => "Russian",
    "ja" => "Japanese",
    "de" => "German",
    "ko" => "Korean",
    "it" => "Italian",
    "tr" => "Turkish",
    "vi" => "Vietnamese",
    "pl" => "Polish",
    "uk" => "Ukrainian",
    "nl" => "Dutch",
    "th" => "Thai",
}

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
        match resolve(requested) {
            locale if locale != REFERENCE => Self::build(locale, Some(Box::new(Self::build(REFERENCE, None)))),
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

    /// Vrai si cette langue a le message `id` elle-même (sans le secours anglais).
    pub fn has(&self, id: &str) -> bool {
        self.bundle.has_message(id)
    }

    /// Un message sans paramètre.
    pub fn tr(&self, id: &str) -> String {
        self.format(id, None)
    }

    /// Un message sans paramètre, ou `None` s'il n'existe ni dans cette langue
    /// ni en anglais (là où `tr` rendrait l'identifiant).
    pub fn try_tr(&self, id: &str) -> Option<String> {
        if self.bundle.get_message(id).and_then(|m| m.value()).is_some() {
            return Some(self.tr(id));
        }
        self.fallback.as_ref().and_then(|f| f.try_tr(id))
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
/// Les langues livrées (`en`, `fr`…), la référence d'abord.
pub fn available() -> Vec<&'static str> {
    LOCALES.iter().map(|(l, _)| *l).collect()
}

/// Nom anglais de la langue livrée `locale` (`fr` → `French`) ; `None` si elle ne l'est pas.
pub fn english_name(locale: &str) -> Option<&'static str> {
    ENGLISH_NAMES.iter().find(|(l, _)| *l == locale).map(|(_, n)| *n)
}

/// Le code de langue que `Localizer::new` choisirait, sans charger les messages.
///
/// C'est la langue demandée si elle est livrée, sinon celle du système,
/// sinon l'anglais.
pub fn resolve(requested: Option<&str>) -> &'static str {
    requested.and_then(shipped).or_else(|| sys_locale::get_locale().as_deref().and_then(shipped)).unwrap_or(REFERENCE)
}

/// La langue livrée pour `tag` (`fr-CA` → `fr`) ; `None` si elle ne l'est pas.
pub fn shipped(tag: &str) -> Option<&'static str> {
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
