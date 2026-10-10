//! Import d'une présentation PowerPoint en représentation intermédiaire.
//!
//! [`read_pptx`] lit les octets d'un `.pptx` et rend une [`source::SourceDeck`] :
//! géométrie en pt, styles texte résolus, couleurs avec leur provenance,
//! tableaux, graphiques, SmartArt, replis, polices. Bornes : [`ReadLimits`] ;
//! erreurs : [`ImportError`].
//!
//! Un projecteur rend un [`ImportPlan`] : les opérations qui construisent le
//! deck Oku, les assets qu'elles référencent et un [`ImportReport`] (pertes,
//! replis, couverture du contenu). Le retrofit ([`retrofit::project`]) projette
//! la source sur un pack et un thème existants ([`RetrofitTarget`]) ;
//! [`coverage::coverage`] mesure ce que le deck obtenu garde de la source.

pub mod coverage;
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
pub use limits::ReadLimits;
pub use plan::ImportPlan;
pub use pptx::read_pptx;
pub use report::ImportReport;
pub use retrofit::RetrofitTarget;
