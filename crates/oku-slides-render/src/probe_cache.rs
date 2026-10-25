//! Cache des sondes de fichiers de widgets, borné.
//!
//! Règles :
//! - Par nom de fichier, au plus [`FileProbeCache::PER_FILE`] sondes, chacune
//!   sous sa [`crate::probe_key`] : alterner quelques versions (deux packs
//!   aux mêmes noms, annuler puis rétablir) ne recompile rien.
//! - Au-delà, la sonde la moins récemment servie de ce nom est évincée :
//!   éditer un widget ne fait pas grandir le cache.

use std::collections::{HashMap, VecDeque};

use oku_slides_core::WidgetFile;

use crate::widget_probe::FileProbe;

/// Les sondes gardées, par nom puis par clé, de la moins à la plus récente.
#[derive(Debug, Default)]
pub struct FileProbeCache {
    by_name: HashMap<WidgetFile, VecDeque<([u8; 32], FileProbe)>>,
}

impl FileProbeCache {
    /// Sondes gardées par nom de fichier, au plus.
    pub const PER_FILE: usize = 4;

    /// La sonde de `name` sous `key`, devenue la plus récente.
    pub fn get(&mut self, name: &WidgetFile, key: &[u8; 32]) -> Option<FileProbe> {
        let entries = self.by_name.get_mut(name)?;
        let i = entries.iter().position(|(k, _)| k == key)?;
        let entry = entries.remove(i)?;
        let probe = entry.1.clone();
        entries.push_back(entry);
        Some(probe)
    }

    /// Garde `probe` sous `key` ; évince la plus ancienne de `name` au-delà de [`Self::PER_FILE`].
    pub fn insert(&mut self, name: &WidgetFile, key: [u8; 32], probe: FileProbe) {
        let entries = self.by_name.entry(name.clone()).or_default();
        entries.retain(|(k, _)| *k != key);
        while entries.len() >= Self::PER_FILE {
            entries.pop_front();
        }
        entries.push_back((key, probe));
    }

    /// Nombre de sondes gardées, tous noms confondus.
    pub fn len(&self) -> usize {
        self.by_name.values().map(VecDeque::len).sum()
    }

    /// True if no probe is kept.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
#[path = "probe_cache_tests.rs"]
mod probe_cache_tests;
