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
    Dir(crate::test_support::fresh_dir("oku-slides-merge-").keep())
}

/// Une session sur la fixture, enregistrée dans `dir/deck.oku-slides.pdf`.
fn saved(dir: &Dir) -> (Session, PathBuf) {
    let mut settings = crate::Settings::defaults().unwrap();
    settings.paths.data = Some(dir.0.clone());
    // Jamais dans le vrai dossier Documents : les decks sans fichier restent ici.
    settings.paths.decks = settings.paths.data.as_ref().map(|d| d.join("decks"));
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

// ---- Dossier par défaut d'un deck sans fichier ------------------------------------------

/// Une session neuve (deck vierge), qui range ses decks sans fichier dans `dir`.
fn blank_in(dir: &Dir) -> Session {
    let mut settings = crate::Settings::defaults().unwrap();
    settings.paths.data = Some(dir.0.join("data"));
    // Jamais dans le vrai dossier Documents : les decks sans fichier restent ici.
    settings.paths.decks = settings.paths.data.as_ref().map(|d| d.join("decks"));
    settings.i18n.locale = Some("fr".into());
    let platform = crate::Platform::with_settings(settings).unwrap();
    let mut s = Session::new(crate::Fonts::default(), &platform).unwrap();
    s.set_decks_dir(dir.0.join("decks"));
    s
}

/// Renomme le deck, comme le titre modifié dans l'interface.
fn titled(s: &mut Session, title: &str) {
    let op = oku_slides_core::Op::SetParam { key: "title".into(), value: title.into() };
    s.apply("titre", vec![op]).unwrap();
}

#[test]
fn save_without_path_writes_into_decks_dir_under_the_deck_title() {
    let d = dir();
    let mut s = blank_in(&d);
    titled(&mut s, "Budget 2027");
    s.save(None).unwrap();
    let path = d.0.join("decks/Budget 2027.oku-slides.pdf");
    assert_that!(path.exists()).is_true();
    assert_that!(s.doc.path.as_deref()).is_equal_to(Some(path.as_path()));
}

#[test]
fn save_without_path_numbers_a_name_already_taken() {
    let d = dir();
    let mut first = blank_in(&d);
    titled(&mut first, "Budget");
    first.save(None).unwrap();
    let mut second = blank_in(&d);
    titled(&mut second, "Budget");
    second.save(None).unwrap();
    assert_that!(second.doc.path.clone()).is_equal_to(Some(d.0.join("decks/Budget 2.oku-slides.pdf")));
}

#[test]
fn save_without_path_keeps_separators_out_of_the_file_name() {
    let d = dir();
    let mut s = blank_in(&d);
    titled(&mut s, "Q1/Q2 : plan");
    s.save(None).unwrap();
    assert_that!(s.doc.path.clone()).is_equal_to(Some(d.0.join("decks/Q1-Q2 - plan.oku-slides.pdf")));
}

// ---- Déplacer, enregistrer une copie ----------------------------------------------------

#[test]
fn move_to_writes_the_new_file_and_removes_the_old_one() {
    let d = dir();
    let (mut s, old) = saved(&d);
    let new = d.0.join("ailleurs/deck.oku-slides.pdf");
    s.move_to(&new).unwrap();
    assert_that!(new.exists()).is_true();
    assert_that!(old.exists()).is_false();
    assert_that!(s.doc.path.clone()).is_equal_to(Some(new));
}

#[test]
fn move_to_its_own_path_keeps_the_file() {
    let d = dir();
    let (mut s, path) = saved(&d);
    s.move_to(&path).unwrap();
    assert_that!(path.exists()).is_true();
}

#[test]
fn move_to_without_a_file_just_writes_there() {
    let d = dir();
    let mut s = blank_in(&d);
    let new = d.0.join("ici.oku-slides.pdf");
    s.move_to(&new).unwrap();
    assert_that!(s.doc.path.clone()).is_equal_to(Some(new));
}

#[test]
fn save_copy_writes_a_copy_and_keeps_the_document_file() {
    let d = dir();
    let (s, path) = saved(&d);
    let copy = d.0.join("copie.oku-slides.pdf");
    s.save_copy(&copy).unwrap();
    assert_that!(copy.exists()).is_true();
    assert_that!(s.doc.path.clone()).is_equal_to(Some(path));
}

#[test]
fn save_copy_leaves_unsaved_changes_unsaved() {
    let d = dir();
    let (mut s, _) = saved(&d);
    titled(&mut s, "Autre");
    s.save_copy(&d.0.join("copie.oku-slides.pdf")).unwrap();
    assert_that!(s.doc.dirty).is_true();
}
