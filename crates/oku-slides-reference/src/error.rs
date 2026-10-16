//! Erreurs du rendu de référence : opaques, avec leur nature
//! ([`ReferenceErrorKind`]) pour le message que l'appelant localise.

use snafu::{Backtrace, Snafu};

/// Nature d'une erreur du rendu de référence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceErrorKind {
    /// Le témoin n'a pas pu rendre la présentation.
    Render,
    /// Le témoin, un programme externe, a dépassé son délai ; il a été tué.
    Timeout,
    /// Le PDF rendu est illisible, ou n'a aucune page.
    Pdf,
    /// La page demandée n'existe pas.
    NoPage,
    /// L'image demandée dépasse [`crate::MAX_RASTER_SIDE`] ou est vide.
    Size,
    /// Un PNG illisible ou impossible à écrire.
    Png,
}

/// Une erreur du rendu de référence.
#[derive(Debug, Snafu)]
#[snafu(display("{inner}"))]
pub struct ReferenceError {
    inner: Inner,
}

impl ReferenceError {
    /// Sa nature.
    pub fn kind(&self) -> ReferenceErrorKind {
        match self.inner {
            Inner::Render { .. } => ReferenceErrorKind::Render,
            Inner::Timeout { .. } => ReferenceErrorKind::Timeout,
            Inner::Pdf { .. } => ReferenceErrorKind::Pdf,
            Inner::NoPage { .. } => ReferenceErrorKind::NoPage,
            Inner::Size { .. } => ReferenceErrorKind::Size,
            Inner::Png { .. } => ReferenceErrorKind::Png,
        }
    }
}

impl From<Inner> for ReferenceError {
    fn from(inner: Inner) -> Self {
        Self { inner }
    }
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
pub(crate) enum Inner {
    #[snafu(display("the witness could not render the presentation: {message}"))]
    Render { message: String, backtrace: Backtrace },
    #[snafu(display("the witness took more than {ms} ms and was killed"))]
    Timeout { ms: u64, backtrace: Backtrace },
    #[snafu(display("unreadable reference pdf: {message}"))]
    Pdf { message: String, backtrace: Backtrace },
    #[snafu(display("no page {index} in the reference pdf"))]
    NoPage { index: usize, backtrace: Backtrace },
    #[snafu(display("raster size out of range: {width} x {height}"))]
    Size { width: u32, height: u32, backtrace: Backtrace },
    #[snafu(display("png: {message}"))]
    Png { message: String, backtrace: Backtrace },
}
