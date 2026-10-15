//! Les versions de thèmes dans le zip des sources (t-554).

use std::io::Cursor;

use super::*;
use crate::PackFiles;

/// Id des thèmes de test.
const THEME_ID: uuid::Uuid = uuid::Uuid::from_u128(0x554);

/// Une version de thème : `theme.json` seul, sans widget.
fn theme_files(name: &str) -> PackFiles {
    let manifest = serde_json::json!({
        "id": THEME_ID.to_string(), "name": name, "description": "", "parent": null,
        "created": "2026-10-05T00:00:00Z", "tokens": { "color": { "accent": "#ff0000" } },
    });
    PackFiles::new(manifest.as_object().cloned().unwrap(), BTreeMap::new())
}

/// Pose `theme` sur le deck du bundle, en une étape d'annulation.
fn set_deck_theme(b: &mut SourceBundle, theme: crate::ThemeRef) {
    let mut next = b.deck().clone();
    next.theme = Some(theme);
    b.apply(next, "thème", None).unwrap();
}

/// `roadmap` qui référence `files` comme thème, et son bundle qui l'embarque.
fn bundle_with_theme(files: &PackFiles) -> SourceBundle {
    let mut b = SourceBundle::new(roadmap(), crate::test_registry::shared()).unwrap();
    let v = b.add_theme(files.clone());
    set_deck_theme(&mut b, crate::ThemeRef { id: THEME_ID, version: v });
    b
}

fn issue_ids(issues: &[Issue]) -> Vec<&str> {
    issues.iter().map(|i| i.message.id()).collect()
}

#[test]
fn theme_versions_round_trip_through_zip() {
    let files = theme_files("Mon thème");
    let b = bundle_with_theme(&files);
    let v = files.version();
    let zip = b.to_zip().unwrap();
    let za = zip::ZipArchive::new(Cursor::new(zip.clone())).unwrap();
    let names: Vec<&str> = za.file_names().filter(|n| n.starts_with("themes/")).collect();
    assert_that!(names).is_equal_to(vec![format!("themes/{}/theme.json", v.hex()).as_str()]);
    let (read, issues) = SourceBundle::from_zip(&zip, MAX, crate::test_registry::shared()).unwrap();
    assert_that!(issue_ids(&issues)).does_not_contain("issue-theme-version-missing");
    assert_that!(read.themes.get(&v)).is_equal_to(Some(&files));
    assert_that!(read.deck().theme.as_ref().map(|t| t.version.clone())).is_equal_to(Some(v));
}

#[test]
fn theme_with_wrong_hash_is_dropped_with_issue() {
    let loro = deck_loro();
    let name = format!("themes/{}/theme.json", "e".repeat(64));
    let json = theme_files("Altéré").pack_json().unwrap();
    let entries: &[(&str, &[u8])] = &[("manifest.json", MANIFEST), ("deck.loro", &loro), (name.as_str(), &json)];
    let (b, issues) = SourceBundle::from_zip(&zip(entries), MAX, crate::test_registry::shared()).unwrap();
    assert_that!(b.themes.is_empty()).is_true();
    assert_that!(issue_ids(&issues)).is_equal_to(vec!["issue-theme-version-mismatch"]);
}

#[test]
fn theme_whose_json_is_not_an_object_is_dropped_with_issue() {
    let loro = deck_loro();
    let name = format!("themes/{}/theme.json", "e".repeat(64));
    let entries: &[(&str, &[u8])] = &[("manifest.json", MANIFEST), ("deck.loro", &loro), (name.as_str(), b"[1]")];
    let (b, issues) = SourceBundle::from_zip(&zip(entries), MAX, crate::test_registry::shared()).unwrap();
    assert_that!(b.themes.is_empty()).is_true();
    assert_that!(issue_ids(&issues)).is_equal_to(vec!["issue-theme-version-mismatch"]);
}

#[rstest::rstest]
#[case("themes/abc/theme.json")]
#[case("themes/FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF/theme.json")]
#[case("themes/../theme.json")]
#[case(&format!("themes/{}/x/theme.json", "a".repeat(64)))]
#[case(&format!("themes/{}/pack.json", "a".repeat(64)))]
#[case(&format!("themes/{}/widgets/kpi.typ", "a".repeat(64)))]
fn invalid_theme_path_refuses_zip(#[case] name: &str) {
    let loro = deck_loro();
    let entries: &[(&str, &[u8])] = &[("manifest.json", MANIFEST), ("deck.loro", &loro), (name, b"{}")];
    let err = SourceBundle::from_zip(&zip(entries), MAX, crate::test_registry::shared()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::BadName);
}

/// Les entrées de répertoire `themes/` et `themes/<hex>/` sont ignorées.
#[rstest::rstest]
#[case("themes/")]
#[case(&format!("themes/{}/", "a".repeat(64)))]
fn from_zip_bare_themes_directory_entries_are_ignored(#[case] name: &str) {
    let loro = deck_loro();
    let entries: &[(&str, &[u8])] = &[("manifest.json", MANIFEST), ("deck.loro", &loro), (name, b"")];
    let (b, issues) = SourceBundle::from_zip(&zip(entries), MAX, crate::test_registry::shared()).unwrap();
    assert_that!(b.themes.is_empty()).is_true();
    assert_that!(issues.is_empty()).is_true();
}

#[test]
fn missing_deck_theme_version_gives_issue() {
    let mut b = bundle_with_theme(&theme_files("T"));
    b.themes.clear();
    let (_, issues) = SourceBundle::from_zip(&b.to_zip().unwrap(), MAX, crate::test_registry::shared()).unwrap();
    assert_that!(issue_ids(&issues)).is_equal_to(vec!["issue-theme-version-missing"]);
}

#[test]
fn from_zip_themes_count_in_the_total_limit() {
    let loro = deck_loro();
    let files = theme_files(&"x".repeat(4096));
    let name = format!("themes/{}/theme.json", files.version().hex());
    let json = files.pack_json().unwrap();
    let entries: &[(&str, &[u8])] = &[("manifest.json", MANIFEST), ("deck.loro", &loro), (name.as_str(), &json)];
    let limits = SourceLimits { total_bytes: MANIFEST.len() + loro.len() + 100, ..MAX };
    let err = SourceBundle::from_zip(&zip(entries), limits, crate::test_registry::shared()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::TooLarge);
}

#[test]
fn merge_unions_the_themes_of_both_bundles() {
    let (t, u) = (theme_files("T"), theme_files("U"));
    let mut a = bundle_with_theme(&t);
    let mut b = SourceBundle::from_zip(&a.to_zip().unwrap(), MAX, crate::test_registry::shared()).unwrap().0;
    b.add_theme(u.clone());
    a.merge(b).unwrap();
    let mut expected = vec![t.version(), u.version()];
    expected.sort();
    assert_that!(a.themes.keys().cloned().collect::<Vec<_>>()).is_equal_to(expected);
}

/// Toute version de thème que l'historique a référencée reste dans le
/// fichier : restaurer une version passée retrouve son thème.
#[test]
fn theme_versions_referenced_by_the_history_are_kept_on_save() {
    let (t, u) = (theme_files("T"), theme_files("U"));
    let mut b = bundle_with_theme(&t);
    let v = b.add_theme(u.clone());
    // Un autre libellé que celui de `T` : au rejeu (t-809), deux changes de même
    // message dans la même seconde ne feraient qu'une version (ADR-0006, Limites).
    let mut next = b.deck().clone();
    next.theme = Some(crate::ThemeRef { id: THEME_ID, version: v });
    b.apply(next, "thème U", None).unwrap();
    let (back, issues) = SourceBundle::from_zip(&b.to_zip().unwrap(), MAX, crate::test_registry::shared()).unwrap();
    assert_that!(issue_ids(&issues)).does_not_contain("issue-theme-version-missing");
    assert_that!(back.themes.contains_key(&t.version()) && back.themes.contains_key(&u.version())).is_true();
    let preview = back.history();
    let mut restored = back;
    // Version 1 : le thème `T` posé (0 : création du deck, sans thème).
    restored.restore(&preview, 1, "restaurer").unwrap();
    assert_that!(restored.deck().theme.as_ref().map(|r| r.version.clone())).is_equal_to(Some(t.version()));
}

#[test]
fn bundles_with_different_themes_are_not_equal() {
    let a = SourceBundle::new(roadmap(), crate::test_registry::shared()).unwrap();
    let mut b = SourceBundle::new(roadmap(), crate::test_registry::shared()).unwrap();
    b.add_theme(theme_files("T"));
    assert_that!(a == b).is_false();
}

/// t-355 : la version de thème du deck, altérée, est dite altérée une seule
/// fois, pas en plus manquante.
#[test]
fn forged_deck_theme_version_is_reported_once() {
    let files = theme_files("T");
    let bytes = bundle_with_theme(&files).to_zip().unwrap();
    let mut za = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..za.len() {
        let mut f = za.by_index(i).unwrap();
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut f, &mut data).unwrap();
        if f.name().ends_with("theme.json") {
            data = theme_files("Autre").pack_json().unwrap();
        }
        entries.push((f.name().to_owned(), data));
    }
    let refs: Vec<(&str, &[u8])> = entries.iter().map(|(n, d)| (n.as_str(), d.as_slice())).collect();
    let (b, issues) = SourceBundle::from_zip(&zip(&refs), MAX, crate::test_registry::shared()).unwrap();
    assert_that!(b.themes.is_empty()).is_true();
    assert_that!(issue_ids(&issues)).is_equal_to(vec!["issue-theme-version-mismatch"]);
}
