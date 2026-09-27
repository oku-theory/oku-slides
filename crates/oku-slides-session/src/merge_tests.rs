use oku_slides_core::MergeReport;
use speculoos::prelude::*;

use super::*;

/// Le texte sans les marques d'isolement bidirectionnel de Fluent.
fn plain(text: String) -> String {
    text.replace(['\u{2068}', '\u{2069}'], "")
}

fn report(changes: usize, authors: &[&str], anonymous: usize) -> MergeReport {
    MergeReport { changes, authors: authors.iter().map(|a| (*a).to_owned()).collect(), anonymous }
}

#[test]
fn merge_notice_with_authors_names_the_file_count_and_authors() {
    let l = Localizer::new(Some("en"));
    let text = plain(merge_notice(&l, "julie.oku-slides.pdf", &report(3, &["Julie", "Marc"], 0)));
    assert_that!(text.as_str()).is_equal_to("Merged with julie.oku-slides.pdf — changes imported: 3 (Julie, Marc).");
}

#[test]
fn merge_notice_without_authors_uses_the_anonymous_message() {
    let l = Localizer::new(Some("en"));
    let text = plain(merge_notice(&l, "copie.oku-slides.pdf", &report(2, &[], 2)));
    assert_that!(text.as_str()).is_equal_to("Merged with copie.oku-slides.pdf — changes imported: 2.");
}

#[test]
fn merge_notice_in_french_is_translated() {
    let l = Localizer::new(Some("fr"));
    let text = plain(merge_notice(&l, "julie.oku-slides.pdf", &report(1, &["Julie"], 1)));
    assert_that!(text.as_str())
        .is_equal_to("Fusionné avec julie.oku-slides.pdf — modifications importées : 1 (Julie).");
}

// ---- Avis en attente, fusion à l'enregistrement --------------------------------------

/// Un dossier neuf, supprimé à la fin du test.
struct Dir(PathBuf);

impl Drop for Dir {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

fn dir() -> Dir {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    Dir(std::env::temp_dir().join(format!("oku-slides-merge-{}-{n}", std::process::id())))
}

/// Une session sur la fixture, enregistrée dans `dir/deck.oku-slides.pdf`.
fn saved(dir: &Dir) -> (Session, PathBuf) {
    let mut settings = crate::Settings::defaults().unwrap();
    settings.paths.data = Some(dir.0.clone());
    settings.i18n.locale = Some("fr".into());
    let platform = crate::Platform::with_settings(settings).unwrap();
    let mut s = Session::new(crate::Fonts::default(), &platform).unwrap();
    s.open(&PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"))).unwrap();
    let path = dir.0.join("deck.oku-slides.pdf");
    s.save(Some(&path)).unwrap();
    (s, path)
}

/// Le fichier touché par un autre outil : même deck, autre empreinte (la
/// date `/ModDate` change, à longueur égale).
fn touch_elsewhere(path: &Path) {
    let mut bytes = std::fs::read(path).unwrap();
    let at = bytes.windows(8).position(|w| w == b"/ModDate").unwrap();
    let date = at + bytes[at..].windows(2).position(|w| w == b"D:").unwrap() + 2;
    bytes[date..date + 4].copy_from_slice(b"2099");
    std::fs::write(path, bytes).unwrap();
}

#[test]
fn set_notice_appends_to_a_pending_notice_after_a_blank_line() {
    let d = dir();
    let (mut s, _) = saved(&d);
    s.set_notice("Un".into());
    s.set_notice("Deux".into());
    assert_that!(s.doc.notice.as_deref()).is_equal_to(Some("Un\n\nDeux"));
}

#[test]
fn merge_file_keeps_a_pending_notice() {
    let d = dir();
    let (mut s, path) = saved(&d);
    s.set_notice("Avant".into());
    s.merge_file(&path).unwrap();
    let notice = s.doc.notice.clone().unwrap_or_default();
    assert_that!(notice.starts_with("Avant\n\n")).is_true();
    assert_that!(notice.as_str()).contains("deck.oku-slides.pdf");
}

#[test]
fn save_merging_nothing_new_sets_no_notice() {
    let d = dir();
    let (mut s, path) = saved(&d);
    touch_elsewhere(&path);
    let saved = s.save_checked(None, false).unwrap();
    assert_that!(saved).is_equal_to(Saved::Merged(MergeReport::default()));
    assert_that!(s.doc.notice).is_none();
}
