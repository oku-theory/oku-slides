//! The per-id opening lock that serializes the opens of one pack or theme.
//!
//! An open of id I runs under I's lock (`with_opening`): a second open of I
//! waits, then finds the first one's tab; opens of other ids never wait on
//! it. Pack and theme ids share the map: their ids never collide. The lock is
//! the first of the lock order, taken before the workspace.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};

use uuid::Uuid;

/// One opening lock per pack or theme id; the map is a leaf.
static OPENING: Mutex<BTreeMap<Uuid, Arc<Mutex<()>>>> = Mutex::new(BTreeMap::new());

/// An id's opening lock, fetched from `OPENING`; its entry goes when unused.
struct OpeningSlot {
    /// The pack or theme.
    id: Uuid,
    /// Its lock, cloned under the map lock.
    arc: Arc<Mutex<()>>,
}

impl Drop for OpeningSlot {
    fn drop(&mut self) {
        // Runs during an unwind too, where a panic would abort: the poison is taken over.
        let mut map = OPENING.lock().unwrap_or_else(PoisonError::into_inner);
        // The map's copy plus this one: every clone is taken under the map lock, so nobody waits.
        if Arc::strong_count(&self.arc) == 2 && map.get(&self.id).is_some_and(|a| Arc::ptr_eq(a, &self.arc)) {
            map.remove(&self.id);
        }
    }
}

/// Runs `f` holding `id`'s opening lock, then forgets the lock if unused.
///
/// # Blocking
///
/// Waits for another open of `id`, its session build included.
pub(super) fn with_opening<R>(id: Uuid, f: impl FnOnce() -> R) -> R {
    let slot = {
        let mut map = OPENING.lock().unwrap_or_else(PoisonError::into_inner);
        OpeningSlot { id, arc: Arc::clone(map.entry(id).or_default()) }
    };
    // Declared after `slot`: released first.
    let _held = slot.arc.lock().unwrap_or_else(PoisonError::into_inner);
    f()
}

/// How many opens of `id` wait on its opening lock.
///
/// The clones beyond the map's and its holder's.
#[cfg(test)]
pub(super) fn opening_waiters(id: Uuid) -> usize {
    let map = OPENING.lock().unwrap_or_else(PoisonError::into_inner);
    map.get(&id).map_or(0, |arc| Arc::strong_count(arc).saturating_sub(2))
}

/// Whether `id` has an opening entry.
#[cfg(test)]
pub(super) fn opening_entry(id: Uuid) -> bool {
    OPENING.lock().unwrap_or_else(PoisonError::into_inner).contains_key(&id)
}
