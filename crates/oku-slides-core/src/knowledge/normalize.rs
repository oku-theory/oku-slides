//! Repair of a knowledge bundle read from untrusted bytes or a merge.
//!
//! Ops never produce what is repaired here; a hostile peer, a forged file or two peers
//! creating the same path can. [`repair`] is the structural part (ids, paths, collisions) that
//! `canonicalize` runs on every apply; [`normalize`] adds what a read reports (frontmatter,
//! ceilings). Both are pure and deterministic, so every peer reads the same bundle, and
//! idempotent, so the first apply after a read persists the repair and the next is write-free.
//!
//! Reading is lenient: no readable file is ever dropped, an over-limit bundle included (two
//! peers can each stay under a ceiling and pass it together); only ops refuse to grow it.

use std::collections::{BTreeMap, BTreeSet};

use super::limits::{MAX_FILE_BYTES, MAX_FILES, MAX_TOTAL_BYTES};
use super::path::FileId;
use super::{KnowledgeFile, KnowledgePath, check_text};
use crate::error::KnowledgeLimit;
use crate::validate::Issue;

/// The bundle repaired, and one issue per repair or problem found.
///
/// [`repair`], then: a text whose frontmatter is invalid is kept unchanged and reported; each
/// bundle ceiling passed (file count, bytes per file, total bytes, counted in id order) is
/// reported once, with the first file past it and how many are, and every file is kept.
#[must_use]
pub fn normalize(files: BTreeMap<String, KnowledgeFile>) -> (BTreeMap<String, KnowledgeFile>, Vec<Issue>) {
    let mut issues = Vec::new();
    let placed = placed(files, &mut issues);
    for (path, file) in placed.values() {
        if let Err(reason) = check_text(path, &file.text) {
            issues.push(Issue::knowledge_invalid(path.as_str(), &reason));
        }
    }
    let files: BTreeMap<_, _> = placed.into_iter().map(|(id, (_, file))| (id, file)).collect();
    issues.extend(over_limits(&files));
    (files, issues)
}

/// The bundle structurally repaired, and one issue per repair; texts are not read.
///
/// In order: entries with a malformed id are dropped (they do not read as a file); an
/// invalid path becomes `/recovered/<id>.md`; each path is kept by its lowest id and the other
/// claimants take the first free `-n` suffix, or the recovered path then a suffix when no
/// suffix fits.
#[must_use]
pub(crate) fn repair(files: BTreeMap<String, KnowledgeFile>) -> (BTreeMap<String, KnowledgeFile>, Vec<Issue>) {
    let mut issues = Vec::new();
    let files = placed(files, &mut issues).into_iter().map(|(id, (_, file))| (id, file)).collect();
    (files, issues)
}

/// A file's path: as written, or recovered because it was invalid.
enum Claim {
    Valid(KnowledgePath),
    Recovered(KnowledgePath),
}

/// Every readable file, by id, with its repaired path (also written into the file).
fn placed(
    files: BTreeMap<String, KnowledgeFile>,
    issues: &mut Vec<Issue>,
) -> BTreeMap<String, (KnowledgePath, KnowledgeFile)> {
    let mut claims = BTreeMap::new();
    for (id, file) in files {
        let id = match FileId::parse(id) {
            Ok(id) => id,
            Err(id) => {
                issues.push(Issue::knowledge_unreadable(&id));
                continue;
            }
        };
        let claim = match KnowledgePath::parse_writable(&file.path) {
            Ok(path) => Claim::Valid(path),
            Err(reason) => {
                issues.push(Issue::knowledge_path_invalid(id.as_str(), reason));
                Claim::Recovered(KnowledgePath::recovered(&id))
            }
        };
        claims.insert(id, (claim, file));
    }
    without_collisions(claims, issues)
}

/// The first free path among `base` and its suffixes `-2` to `-(bound + 1)`.
///
/// `None` when a suffix no longer fits in a path (`with_suffix` fails only with `TooLong`)
/// before a free one is found, or when all are taken; with fewer than `bound` paths taken,
/// the second cannot happen.
fn first_free(base: &KnowledgePath, taken: &BTreeSet<KnowledgePath>, bound: usize) -> Option<KnowledgePath> {
    if !taken.contains(base) {
        return Some(base.clone());
    }
    // `take` bounds the count without converting `bound` to the suffix type.
    for n in (2..=u32::MAX).take(bound) {
        let Ok(candidate) = base.with_suffix(n) else { return None };
        if !taken.contains(&candidate) {
            return Some(candidate);
        }
    }
    None
}

/// Each valid path kept by its lowest id; the others suffixed, or recovered then suffixed when
/// no suffix fits. A file that still finds no place is reported unreadable, never dropped
/// silently (it cannot happen: a recovered path has room for any suffix, see [`first_free`]).
fn without_collisions(
    claims: BTreeMap<FileId, (Claim, KnowledgeFile)>,
    issues: &mut Vec<Issue>,
) -> BTreeMap<String, (KnowledgePath, KnowledgeFile)> {
    let bound = claims.len();
    let mut taken = BTreeSet::new();
    let mut out = BTreeMap::new();
    let mut later = Vec::new();
    // Pass one: claims go in id order, so the lowest id keeps its path.
    for (id, (claim, file)) in claims {
        match claim {
            Claim::Valid(path) if taken.insert(path.clone()) => {
                out.insert(id.into_string(), at(path, file));
            }
            Claim::Valid(path) => later.push((id, path, true, file)),
            Claim::Recovered(path) => later.push((id, path, false, file)),
        }
    }
    // Pass two: the losers, and the recovered paths, take the first free place.
    for (id, path, lost, file) in later {
        let place =
            first_free(&path, &taken, bound).or_else(|| first_free(&KnowledgePath::recovered(&id), &taken, bound));
        let Some(place) = place else {
            // Unreachable: a recovered path (`/recovered/<32 hex>.md`) takes any suffix, and
            // fewer than `bound` paths are taken. Were it reached, the file would leave the
            // bundle as an entry that does not read as a file does, hence the same issue: the
            // user is told which id is missing rather than given a message for a dead case.
            issues.push(Issue::knowledge_unreadable(id.as_str()));
            continue;
        };
        if lost || place != path {
            issues.push(Issue::knowledge_path_collision(path.as_str()));
        }
        taken.insert(place.clone());
        out.insert(id.into_string(), at(place, file));
    }
    out
}

/// `file` moved to `path`.
fn at(path: KnowledgePath, mut file: KnowledgeFile) -> (KnowledgePath, KnowledgeFile) {
    if file.path != path.as_str() {
        path.as_str().clone_into(&mut file.path);
    }
    (path, file)
}

/// One issue per ceiling the bundle passes, counted in id order: the first file past it, and
/// how many are.
fn over_limits(files: &BTreeMap<String, KnowledgeFile>) -> Vec<Issue> {
    let mut past: [(KnowledgeLimit, Option<&str>, usize); 3] = [
        (KnowledgeLimit::TooManyFiles, None, 0),
        (KnowledgeLimit::FileTooLarge, None, 0),
        (KnowledgeLimit::TotalTooLarge, None, 0),
    ];
    let mut total = 0usize;
    for (n, (id, file)) in files.iter().enumerate() {
        let len = file.text.len();
        total = total.saturating_add(len);
        let hits = [n >= MAX_FILES, len > MAX_FILE_BYTES, total > MAX_TOTAL_BYTES];
        for ((_, first, count), hit) in past.iter_mut().zip(hits) {
            if hit {
                first.get_or_insert(id.as_str());
                *count += 1;
            }
        }
    }
    past.iter()
        .filter_map(|(reason, first, count)| first.map(|id| Issue::knowledge_over_limit(*reason, id, *count)))
        .collect()
}

#[cfg(test)]
#[path = "normalize_tests.rs"]
mod normalize_tests;
