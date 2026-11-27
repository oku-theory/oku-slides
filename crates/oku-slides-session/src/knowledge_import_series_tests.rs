//! t-1760: what a re-import replaces, by the mark of the earlier import's files (D1), never an
//! edited one (D2); the room it counts free (F32); another import refused while one runs (D3).

use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::Duration;

use oku_slides_core::knowledge::limits::MAX_FILE_BYTES;
use oku_slides_core::knowledge::{DocumentType, ExtractLimits, ExtractRefusal, Extracted};
use oku_slides_i18n::{Localize, Localizer};
use oku_slides_import::ReadLimits;
use oku_slides_render::RenderError;
use speculoos::prelude::*;

use super::knowledge_import_tests::{
    Recording, attach, deck, extracted, fixed, import, paths, seed_notes, text_at, two_part_import,
};
use super::*;
use crate::SessionErrorKind;
use crate::knowledge_fixture::seed_knowledge;
use crate::knowledge_import_files::import_mark;

/// `text` with `from` replaced by `to` once, past its frontmatter.
fn edited(text: &str, from: &str, to: &str) -> String {
    let at = text.rfind(from).unwrap();
    format!("{}{to}{}", &text[..at], &text[at + from.len()..])
}

/// `s` with the file at `path` now `text`, the others kept.
fn rewrite(s: &mut Session, path: &str, text: &str) {
    let mut files: Vec<(String, String)> =
        s.deck().knowledge.values().filter(|f| f.path != path).map(|f| (f.path.clone(), f.text.clone())).collect();
    files.push((path.to_owned(), text.to_owned()));
    let files: Vec<(&str, &str)> = files.iter().map(|(p, t)| (p.as_str(), t.as_str())).collect();
    seed_knowledge(s, &files, Vec::new());
}

/// Re-imports a one-part document at `/references/gros.md`.
fn reimport_small(s: &mut Session) -> Result<ImportOutcome> {
    s.set_extract_runner(fixed(Ok(extracted(None, "# Nouveau\n"))));
    let id = attach(s, b"%PDF-1.7\n", "new.pdf");
    import(s, &id, Some("/references/gros.md"))
}

#[test]
fn knowledge_import_marks_each_part_with_its_series() {
    let (_data, mut s) = deck();
    two_part_import(&mut s);
    let first = import_mark(&text_at(&s, "/references/gros.md").unwrap()).unwrap();
    let second = import_mark(&text_at(&s, "/references/gros-part-2.md").unwrap()).unwrap();
    assert_that!((first.part, first.of, second.part, second.of)).is_equal_to((1, 2, 2, 2));
    assert_that!(second.series).is_equal_to(first.series);
}

#[test]
fn reimport_does_not_delete_a_later_part_without_a_mark() {
    let (_data, mut s) = deck();
    two_part_import(&mut s);
    // The user's own file in place of the second part, which says it continues the first.
    let mine = "---\ntype: Note\n---\nContinued in [gros-part-2.md](gros-part-2.md)\n";
    rewrite(&mut s, "/references/gros-part-2.md", mine);
    reimport_small(&mut s).unwrap();
    assert_that!(text_at(&s, "/references/gros-part-2.md")).is_equal_to(Some(mine.to_owned()));
}

#[test]
fn reimport_does_not_delete_a_part_of_another_series() {
    let (_data, mut s) = deck();
    two_part_import(&mut s);
    let part = text_at(&s, "/references/gros-part-2.md").unwrap();
    let series = import_mark(&part).unwrap().series;
    let forged = part.replace(&series, "ffffffffffffffff");
    rewrite(&mut s, "/references/gros-part-2.md", &forged);
    reimport_small(&mut s).unwrap();
    assert_that!(text_at(&s, "/references/gros-part-2.md")).is_equal_to(Some(forged));
}

#[test]
fn reimport_onto_an_edited_part_is_refused_in_french() {
    let (_data, mut s) = deck();
    two_part_import(&mut s);
    let part = edited(&text_at(&s, "/references/gros-part-2.md").unwrap(), "bbbb", "BBBB");
    rewrite(&mut s, "/references/gros-part-2.md", &part);
    let before = paths(&s);
    let e = reimport_small(&mut s).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::ImportTargetEdited);
    let fr = Localizer::new(Some("fr"));
    assert_that!(e.localize(&fr))
        .is_equal_to(fr.tr_with("session-import-target-edited", &[("path", "/references/gros-part-2.md".into())]));
    assert_that!(paths(&s)).is_equal_to(before);
    assert_that!(text_at(&s, "/references/gros-part-2.md")).is_equal_to(Some(part));
}

#[test]
fn reimport_onto_an_edited_first_part_is_refused() {
    let (_data, mut s) = deck();
    two_part_import(&mut s);
    let first = edited(&text_at(&s, "/references/gros.md").unwrap(), "aaaa", "AAAA");
    rewrite(&mut s, "/references/gros.md", &first);
    let e = reimport_small(&mut s).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::ImportTargetEdited);
    assert_that!(text_at(&s, "/references/gros.md")).is_equal_to(Some(first));
}

#[test]
fn reimport_after_an_earlier_reimport_still_replaces() {
    let (_data, mut s) = deck();
    two_part_import(&mut s);
    reimport_small(&mut s).unwrap();
    two_part_import_at(&mut s, "/references/gros.md");
    assert_that!(paths(&s))
        .is_equal_to(vec!["/references/gros-part-2.md".to_owned(), "/references/gros.md".to_owned()]);
}

/// Imports the two-part body at `path`.
fn two_part_import_at(s: &mut Session, path: &str) {
    let body = format!("## Un\n{}\n## Deux\n{}\n", "c".repeat(150 * 1024), "d".repeat(150 * 1024));
    s.set_extract_runner(fixed(Ok(extracted(None, &body))));
    let id = attach(s, b"%PDF-1.7\n", "gros.pdf");
    import(s, &id, Some(path)).unwrap();
}

#[test]
fn knowledge_import_job_with_a_path_counts_the_replaced_import_as_room() {
    let (_data, mut s) = deck();
    seed_notes(&mut s, 10, MAX_FILE_BYTES - 64);
    two_part_import(&mut s);
    let replaced =
        text_at(&s, "/references/gros.md").unwrap().len() + text_at(&s, "/references/gros-part-2.md").unwrap().len();
    let recording = Arc::new(Recording::default());
    s.set_extract_runner(recording.clone());
    let id = attach(&mut s, b"%PDF-1.7\n", "new.pdf");
    let room = |s: &Session, path: Option<&str>| {
        s.knowledge_import_job(&id, path).unwrap().run().unwrap();
        recording.0.lock().unwrap_or_else(PoisonError::into_inner).take().unwrap().0.max_text_bytes
    };
    let fresh = room(&s, None);
    let replacing = room(&s, Some("/references/gros.md"));
    assert_that!(replacing).is_greater_than(fresh);
    assert_that!(replacing - fresh).is_less_than_or_equal_to(replaced);
    assert_that!(replacing - fresh).is_greater_than(replaced / 2);
}

#[test]
fn knowledge_import_job_with_an_edited_target_is_refused_before_extraction() {
    let (_data, mut s) = deck();
    two_part_import(&mut s);
    let first = edited(&text_at(&s, "/references/gros.md").unwrap(), "aaaa", "AAAA");
    rewrite(&mut s, "/references/gros.md", &first);
    let id = attach(&mut s, b"%PDF-1.7\n", "new.pdf");
    let e = s.knowledge_import_job(&id, Some("/references/gros.md")).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::ImportTargetEdited);
}

#[test]
fn knowledge_import_busy_is_refused_in_french() {
    /// A runner that signals it started, then waits for the test's go.
    #[derive(Debug)]
    struct Waiting {
        started: Mutex<mpsc::Sender<()>>,
        go: Mutex<mpsc::Receiver<()>>,
    }
    impl ExtractRunner for Waiting {
        fn extract(
            &self,
            _: &[u8],
            _: DocumentType,
            _: &ExtractLimits,
            _: &ReadLimits,
            _: &str,
        ) -> std::result::Result<std::result::Result<Extracted, ExtractRefusal>, RenderError> {
            self.started.lock().unwrap_or_else(PoisonError::into_inner).send(()).unwrap();
            self.go.lock().unwrap_or_else(PoisonError::into_inner).recv_timeout(Duration::from_secs(10)).unwrap();
            Ok(Ok(extracted(None, "corps\n")))
        }
    }
    let (_data, mut s) = deck();
    let (started_tx, started_rx) = mpsc::channel();
    let (go_tx, go_rx) = mpsc::channel();
    s.set_extract_runner(Arc::new(Waiting { started: Mutex::new(started_tx), go: Mutex::new(go_rx) }));
    let id = attach(&mut s, b"%PDF-1.7\n", "a.pdf");
    let first = s.knowledge_import_job(&id, None).unwrap();
    let second = s.knowledge_import_job(&id, None).unwrap();
    let run = std::thread::spawn(move || first.run().map(|_| ()));
    started_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    let busy = second.run();
    go_tx.send(()).unwrap();
    let e = busy.unwrap_err();
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::KnowledgeImportBusy);
    let fr = Localizer::new(Some("fr"));
    assert_that!(e.localize(&fr)).is_equal_to(fr.tr("session-knowledge-import-busy"));
    assert_that!(run.join().unwrap()).is_ok();
    // The first run over, the next one is let through.
    s.set_extract_runner(fixed(Ok(extracted(None, "corps\n"))));
    assert_that!(s.knowledge_import_job(&id, None).unwrap().run()).is_ok();
}

/// t-1760 (m1): an edit of the frontmatter alone (here a tag) is an edit too.
#[test]
fn reimport_onto_a_frontmatter_edit_is_refused() {
    let (_data, mut s) = deck();
    two_part_import(&mut s);
    let first = text_at(&s, "/references/gros.md").unwrap().replacen(
        "type: \"Reference\"\n",
        "type: \"Reference\"\ntags: [a]\n",
        1,
    );
    rewrite(&mut s, "/references/gros.md", &first);
    let e = reimport_small(&mut s).unwrap_err();
    assert_that!(e.kind()).is_equal_to(SessionErrorKind::ImportTargetEdited);
    assert_that!(text_at(&s, "/references/gros.md")).is_equal_to(Some(first));
}

/// t-1760 (m2): a test session's import slot is its own, so tests running at once in one process
/// never see a spurious busy.
#[test]
fn knowledge_import_with_a_fake_runner_is_not_busy_while_another_session_imports() {
    #[derive(Debug)]
    struct Waiting {
        started: Mutex<mpsc::Sender<()>>,
        go: Mutex<mpsc::Receiver<()>>,
    }
    impl ExtractRunner for Waiting {
        fn extract(
            &self,
            _: &[u8],
            _: DocumentType,
            _: &ExtractLimits,
            _: &ReadLimits,
            _: &str,
        ) -> std::result::Result<std::result::Result<Extracted, ExtractRefusal>, RenderError> {
            self.started.lock().unwrap_or_else(PoisonError::into_inner).send(()).unwrap();
            self.go.lock().unwrap_or_else(PoisonError::into_inner).recv_timeout(Duration::from_secs(10)).unwrap();
            Ok(Ok(extracted(None, "corps\n")))
        }
    }
    let (_a_data, mut a) = deck();
    let (started_tx, started_rx) = mpsc::channel();
    let (go_tx, go_rx) = mpsc::channel();
    a.set_extract_runner(Arc::new(Waiting { started: Mutex::new(started_tx), go: Mutex::new(go_rx) }));
    let id = attach(&mut a, b"%PDF-1.7\n", "a.pdf");
    let job = a.knowledge_import_job(&id, None).unwrap();
    let run = std::thread::spawn(move || job.run().map(|_| ()));
    started_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    let (_b_data, mut b) = deck();
    b.set_extract_runner(fixed(Ok(extracted(None, "autre\n"))));
    let other = attach(&mut b, b"%PDF-1.7\n", "b.pdf");
    let outcome = import(&mut b, &other, None);
    go_tx.send(()).unwrap();
    assert_that!(outcome.map(|o| o.paths)).is_ok_containing(vec!["/references/b.md".to_owned()]);
    assert_that!(run.join().unwrap()).is_ok();
}
