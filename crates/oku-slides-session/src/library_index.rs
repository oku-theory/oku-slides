//! Index en mémoire d'une bibliothèque : ses versions sans relire leurs fichiers.
//!
//! Construit à la demande, pack par pack, et tenu à jour par chaque écriture
//! de la [`crate::Library`]. Le fichier `generation` à la racine de la
//! bibliothèque change à chaque écriture, de ce processus ou d'un autre (CLI,
//! autre onglet) : un `generation` différent de celui vu vide l'index. Une
//! version n'est jamais crue sur l'index quand on la charge : ses octets sont
//! relus et vérifiés contre son empreinte.

use std::collections::HashMap;
use std::path::Path;

use uuid::Uuid;

use oku_slides_core::PackVersion;
use oku_slides_platform::errors::one_line;
use oku_slides_platform::storage::{Storage, StorageErrorKind};

/// Marque de génération, à la racine du dossier de la sorte.
pub(crate) const GENERATION_FILE: &str = "generation";

/// Ce qu'on sait d'une version sans ses fichiers.
#[derive(Debug, Clone)]
pub(crate) struct Indexed<M> {
    pub(crate) version: PackVersion,
    pub(crate) manifest: M,
    /// Arrivée avec un deck reçu.
    pub(crate) received: bool,
}

/// Le fichier `current` d'un pack.
#[derive(Debug, Clone)]
pub(crate) enum Current {
    Absent,
    At(PackVersion),
    /// Illisible ou mal formé.
    Broken,
}

/// Un pack indexé : ses versions (l'intégrée d'abord) et son `current`.
#[derive(Debug)]
pub(crate) struct PackEntry<M> {
    pub(crate) versions: Vec<Indexed<M>>,
    pub(crate) current: Current,
}

/// Le fichier `generation` tel que lu.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Stamp {
    Absent,
    Token(Vec<u8>),
    /// Illisible : jamais égal à lui-même, l'index se reconstruit à chaque accès.
    Unreadable,
}

/// L'index d'une bibliothèque, partagé par ses clones.
#[derive(Debug)]
pub(crate) struct Index<M> {
    /// Génération vue à la construction ; `None` : à vérifier.
    stamp: Option<Stamp>,
    /// Ids des entrées de l'utilisateur, listés à la demande.
    pub(crate) ids: Option<Vec<Uuid>>,
    pub(crate) packs: HashMap<Uuid, PackEntry<M>>,
    /// Packs lus sur le disque depuis la création, pour les tests.
    #[cfg(test)]
    pub(crate) scans: usize,
}

impl<M> Default for Index<M> {
    fn default() -> Self {
        Self {
            stamp: None,
            ids: None,
            packs: HashMap::new(),
            #[cfg(test)]
            scans: 0,
        }
    }
}

impl<M> Index<M> {
    /// Vide l'index si `dir` a changé de génération depuis sa construction.
    pub(crate) fn refresh(&mut self, storage: &Storage, dir: &Path) {
        let seen = read_stamp(storage, dir);
        let fresh = seen != Stamp::Unreadable && self.stamp.as_ref() == Some(&seen);
        if !fresh {
            self.ids = None;
            self.packs.clear();
            self.stamp = Some(seen);
        }
    }

    /// Change la génération de `dir` après une écriture déjà reportée dans l'index.
    pub(crate) fn bump(&mut self, storage: &Storage, dir: &Path) {
        let token = Uuid::new_v4().to_string().into_bytes();
        let path = dir.join(GENERATION_FILE);
        match storage.write_atomic(&path, token.clone()) {
            Ok(()) => self.stamp = Some(Stamp::Token(token)),
            Err(e) => {
                // Les autres processus ne verront pas l'écriture avant la suivante.
                tracing::warn!(path = %path.display(), error = %one_line(&e), "génération non écrite");
                self.stamp = None;
            }
        }
    }
}

/// Taille au-delà de laquelle un `generation` est illisible (un UUID en fait 36).
const MAX_STAMP: u64 = 64;

fn read_stamp(storage: &Storage, dir: &Path) -> Stamp {
    let path = dir.join(GENERATION_FILE);
    match storage.read_limited(&path, MAX_STAMP) {
        Ok(bytes) => Stamp::Token(bytes),
        Err(e) if e.kind() == StorageErrorKind::NotFound => Stamp::Absent,
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %one_line(&e), "génération illisible");
            Stamp::Unreadable
        }
    }
}
