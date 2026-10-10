//! Import d'une présentation PowerPoint en représentation intermédiaire.
//!
//! [`read_pptx`] lit les octets d'un `.pptx` et rend une [`source::SourceDeck`] :
//! géométrie en pt, styles texte résolus, couleurs avec leur provenance,
//! tableaux, graphiques, SmartArt, replis, polices. Bornes : [`ReadLimits`] ;
//! erreurs : [`ImportError`]. Les projecteurs vers un deck Oku viennent aux lots suivants.

pub mod source;

mod collect;
mod error;
mod limits;
mod pptx;

pub use error::{ImportError, ImportErrorKind};
pub use limits::ReadLimits;
pub use pptx::read_pptx;
