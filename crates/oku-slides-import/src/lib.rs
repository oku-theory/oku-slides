//! Import d'une présentation PowerPoint en représentation intermédiaire.
//!
//! [`read_pptx`] lit les octets d'un `.pptx` et rend une [`source::SourceDeck`] :
//! géométrie en pt, styles texte résolus, couleurs avec leur provenance,
//! tableaux, graphiques, SmartArt, replis, polices. Bornes : [`ReadLimits`] ;
//! erreurs : [`ImportError`].
//!
//! Un projecteur rend un [`ImportPlan`] : les opérations qui construisent le
//! deck Oku, les assets qu'elles référencent et un [`ImportReport`] (pertes,
//! replis, couverture du contenu) ; l'appelant borne sa taille par
//! [`ImportPlan::check_budget`]. Le retrofit ([`retrofit::project`]) projette
//! la source sur un pack et un thème existants ([`RetrofitTarget`]) ; le hi-fi
//! ([`hifi::project`]) la reproduit fidèlement sur un thème et un pack générés
//! pour elle ([`HifiTarget`]), que le plan porte avec les polices à embarquer.
//! [`coverage::coverage`] mesure ce que le deck obtenu garde de la source ;
//! [`metrics::score`], l'écart visuel d'une slide hi-fi à un rendu de référence.
//! [`fonts::resolve`] choisit la police de chaque famille source et les
//! fichiers à embarquer dans le deck.

pub mod coverage;
pub mod fonts;
pub mod hifi;
pub mod metrics;
pub mod plan;
pub mod report;
pub mod retrofit;
pub mod source;

mod collect;
mod error;
mod limits;
mod pattern;
mod pptx;
#[cfg(test)]
mod test_support;

pub use error::{ImportError, ImportErrorKind};
pub use hifi::{HifiTarget, PackLimits};
pub use limits::ReadLimits;
pub use plan::ImportPlan;
pub use pptx::read_pptx;
pub use report::ImportReport;
pub use retrofit::RetrofitTarget;
