//! Sortie des commandes, sur stdout, dans la langue de l'utilisateur.
//!
//! Les résultats d'une commande sont sa raison d'être : ils vont sur stdout
//! (les journaux, eux, vont sur stderr). Une écriture qui échoue, un tube
//! fermé par exemple (`oku-slides schema | head`), arrête la commande au lieu de
//! la faire paniquer comme `println!`.

use std::io::{StdoutLock, Write};

use oku_slides_i18n::{FluentValue, Localizer};

pub struct Out<'a> {
    l: &'a Localizer,
    stdout: StdoutLock<'static>,
}

impl std::fmt::Debug for Out<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Out").field("locale", &self.l.locale()).finish_non_exhaustive()
    }
}

impl<'a> Out<'a> {
    pub fn new(l: &'a Localizer) -> Self {
        Self { l, stdout: std::io::stdout().lock() }
    }

    pub fn localizer(&self) -> &'a Localizer {
        self.l
    }

    /// Une ligne : le message `id` et ses paramètres.
    pub fn line(&mut self, id: &str, args: &[(&str, FluentValue<'_>)]) -> std::io::Result<()> {
        let text = self.l.tr_with(id, args);
        writeln!(self.stdout, "{text}")
    }

    /// Des données brutes (JSON), telles quelles.
    pub fn data(&mut self, text: &str) -> std::io::Result<()> {
        writeln!(self.stdout, "{text}")
    }
}
