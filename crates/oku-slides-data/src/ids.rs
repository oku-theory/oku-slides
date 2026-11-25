//! Identifiers of data, rows, columns and generations.

use std::collections::{HashMap, HashSet};

use crate::limits::{MAX_ID_CHARS, NEW_ID_CHARS};

/// Draws a new id absent from the `taken` predicate it is given.
///
/// Every id is drawn through one, always passed as `&mut IdGen<'_>`: `&mut new_id` coerces to it,
/// and tests pass a counter.
pub type IdGen<'a> = dyn FnMut(&dyn Fn(&str) -> bool) -> String + 'a;

/// A valid id: 1 to 64 ASCII letters, digits, `-` or `_` (never `/`).
#[must_use]
pub fn valid_id(id: &str) -> bool {
    (1..=MAX_ID_CHARS).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// A new id of 8 hexadecimal characters, absent from `taken`.
#[must_use]
pub fn new_id(taken: &dyn Fn(&str) -> bool) -> String {
    loop {
        let mut id = uuid::Uuid::new_v4().simple().to_string();
        id.truncate(NEW_ID_CHARS);
        if !taken(&id) {
            return id;
        }
    }
}

/// Repairs duplicate or invalid ids, the same way on every peer.
///
/// Ids are given in order: the first occurrence of a valid id keeps it; a later one, or an
/// invalid one (repaired from `x`), becomes `<base>_<n>` with the smallest free `n ≥ 2`, the
/// base cut to a stem so the whole fits [`MAX_ID_CHARS`]. A per-stem counter keeps the work
/// linear, even for distinct long ids that share a stem.
#[derive(Debug, Default)]
pub struct IdRepair {
    /// Ids given out so far.
    taken: HashSet<String>,
    /// Next suffix to try, per stem and suffix length: every lower suffix of that length is
    /// taken with that stem.
    next: HashMap<(String, usize), usize>,
    /// Taken-set lookups, for the linearity test.
    #[cfg(test)]
    probes: usize,
}

impl IdRepair {
    /// A repair with no id taken.
    #[must_use]
    pub fn new() -> IdRepair {
        IdRepair::default()
    }

    /// `id` if valid and free, else its repaired form; either is now taken.
    pub fn repair_id(&mut self, id: &str) -> String {
        let base = if valid_id(id) { id } else { "x" };
        if self.free(base) {
            self.taken.insert(base.to_owned());
            return base.to_owned();
        }
        let mut n = 2;
        loop {
            let suffix = format!("_{n}");
            // Ids are ASCII, so a byte cut is a character cut.
            let stem = &base[..base.len().min(MAX_ID_CHARS - suffix.len())];
            // Every `<stem>_<m>` with as many digits, below the counter, is taken: skip them.
            // More digits cut a shorter stem, so the stem is recomputed after the jump.
            let key = (stem.to_owned(), suffix.len());
            if let Some(&next) = self.next.get(&key)
                && next > n
            {
                n = next;
                continue;
            }
            let candidate = format!("{stem}{suffix}");
            n += 1;
            let free = self.free(&candidate);
            self.next.insert(key, n);
            if free {
                self.taken.insert(candidate.clone());
                return candidate;
            }
        }
    }

    /// Whether `id` is not taken yet.
    fn free(&mut self, id: &str) -> bool {
        #[cfg(test)]
        {
            self.probes += 1;
        }
        !self.taken.contains(id)
    }

    /// Taken-set lookups made so far.
    #[cfg(test)]
    pub(crate) fn probes(&self) -> usize {
        self.probes
    }
}

#[cfg(test)]
#[path = "ids_tests.rs"]
mod ids_tests;
