//! Les versions de thèmes dans le zip des sources (t-554).

use std::io::Cursor;

use super::*;
use crate::PackFiles;

/// Id des thèmes de test.
const THEME_ID: uuid::Uuid = uuid::Uuid::from_u128(0x554);

/// Espace de noms des thèmes de test.
const THEME_NS: &str = "mien";

fn ns(s: &str) -> crate::Namespace {
    crate::Namespace::parse(s).unwrap()
}

/// Une version du thème `namespace` : `theme.json` seul, sans widget.
fn theme_files_of(namespace: &str, name: &str, extends: &[&str]) -> PackFiles {
    let manifest = serde_json::json!({
        "id": THEME_ID.to_string(), "name": name, "namespace": namespace, "extends": extends,
        "description": "", "parent": null,
        "created": "2026-10-05T00:00:00Z", "tokens": { "color": { "accent": "#ff0000" } },
    });
    PackFiles::theme(manifest.as_object().cloned().unwrap())
}

/// Une version du thème de test [`THEME_NS`].
fn theme_files(name: &str) -> PackFiles {
    theme_files_of(THEME_NS, name, &[])
}

/// L'épinglage de la version `version` du thème de test, sans parent.
fn theme_pin(version: PackVersion) -> crate::ThemePin {
    crate::ThemePin { id: THEME_ID, version, extends: Vec::new() }
}

/// Pose les épinglages `themes` sur le deck du bundle (thème : `theme`), en une étape d'annulation.
fn set_deck_themes(b: &mut SourceBundle, theme: &str, themes: BTreeMap<crate::Namespace, crate::ThemePin>) {
    let mut next = b.deck().clone();
    next.theme = Some(ns(theme));
    next.themes = themes;
    b.apply(next, "thème", None).unwrap();
}

/// Pose `pin` comme seul thème du deck, sous [`THEME_NS`].
fn set_deck_theme(b: &mut SourceBundle, pin: crate::ThemePin) {
    set_deck_themes(b, THEME_NS, [(ns(THEME_NS), pin)].into());
}

/// `roadmap` qui référence `files` comme thème, et son bundle qui l'embarque.
fn bundle_with_theme(files: &PackFiles) -> SourceBundle {
    let mut b = SourceBundle::new(roadmap(), crate::test_registry::shared()).unwrap();
    let v = b.add_theme(files.clone());
    set_deck_theme(&mut b, theme_pin(v));
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
    let names: Vec<String> =
        za.file_names().map(|n| n.unwrap().into_owned()).filter(|n| n.starts_with("themes/")).collect();
    assert_that!(names).is_equal_to(vec![format!("themes/{}/theme.json", v.hex())]);
    let (read, issues) = SourceBundle::from_zip(&zip, MAX, crate::test_registry::shared()).unwrap();
    assert_that!(issue_ids(&issues)).does_not_contain("issue-theme-version-missing");
    assert_that!(read.themes.get(&v)).is_equal_to(Some(&files));
    assert_that!(read.deck().themes.get(&ns(THEME_NS)).map(|t| t.version.clone())).is_equal_to(Some(v));
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
    next.themes = [(ns(THEME_NS), theme_pin(v))].into();
    b.apply(next, "thème U", None).unwrap();
    let (back, issues) = SourceBundle::from_zip(&b.to_zip().unwrap(), MAX, crate::test_registry::shared()).unwrap();
    assert_that!(issue_ids(&issues)).does_not_contain("issue-theme-version-missing");
    assert_that!(back.themes.contains_key(&t.version()) && back.themes.contains_key(&u.version())).is_true();
    let preview = back.history();
    let mut restored = back;
    // Version 1 : le thème `T` posé (0 : création du deck, sans thème).
    restored.restore(&preview, 1, "restaurer").unwrap();
    assert_that!(restored.deck().themes.get(&ns(THEME_NS)).map(|r| r.version.clone())).is_equal_to(Some(t.version()));
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
        if f.name().unwrap().ends_with("theme.json") {
            data = theme_files("Autre").pack_json().unwrap();
        }
        entries.push((f.name().unwrap().into_owned(), data));
    }
    let refs: Vec<(&str, &[u8])> = entries.iter().map(|(n, d)| (n.as_str(), d.as_slice())).collect();
    let (b, issues) = SourceBundle::from_zip(&zip(&refs), MAX, crate::test_registry::shared()).unwrap();
    assert_that!(b.themes.is_empty()).is_true();
    assert_that!(issue_ids(&issues)).is_equal_to(vec!["issue-theme-version-mismatch"]);
}

/// Chaque épinglage du deck, parent compris, doit avoir sa version dans les sources.
#[test]
fn every_missing_theme_pin_version_gives_issue() {
    let (child, parent) = (theme_files_of("enfant", "E", &["parent"]), theme_files_of("parent", "P", &[]));
    let mut b = SourceBundle::new(roadmap(), crate::test_registry::shared()).unwrap();
    let pins = [
        (ns("enfant"), crate::ThemePin { id: THEME_ID, version: child.version(), extends: vec![ns("parent")] }),
        (ns("parent"), theme_pin(parent.version())),
    ];
    set_deck_themes(&mut b, "enfant", pins.into());
    let (_, issues) = SourceBundle::from_zip(&b.to_zip().unwrap(), MAX, crate::test_registry::shared()).unwrap();
    assert_that!(issue_ids(&issues)).is_equal_to(vec!["issue-theme-version-missing", "issue-theme-version-missing"]);
}

#[test]
fn complete_theme_closure_gives_no_issue() {
    let (child, parent) = (theme_files_of("enfant", "E", &["parent"]), theme_files_of("parent", "P", &[]));
    let mut b = SourceBundle::new(roadmap(), crate::test_registry::shared()).unwrap();
    let pins = [
        (ns("enfant"), crate::ThemePin { id: THEME_ID, version: b.add_theme(child), extends: vec![ns("parent")] }),
        (ns("parent"), theme_pin(b.add_theme(parent))),
    ];
    set_deck_themes(&mut b, "enfant", pins.into());
    let (_, issues) = SourceBundle::from_zip(&b.to_zip().unwrap(), MAX, crate::test_registry::shared()).unwrap();
    assert_that!(issues.is_empty()).is_true();
}

/// Un épinglage dont le manifeste embarqué a un autre espace de noms, id ou `extends`.
#[rstest::rstest]
#[case::namespace(theme_files_of("autre", "T", &[]), THEME_ID, &[])]
#[case::id(theme_files("T"), uuid::Uuid::from_u128(1), &[])]
#[case::extends(theme_files("T"), THEME_ID, &["papier"])]
fn theme_pin_not_matching_its_manifest_gives_issue(
    #[case] files: PackFiles,
    #[case] id: uuid::Uuid,
    #[case] extends: &[&str],
) {
    let mut b = SourceBundle::new(roadmap(), crate::test_registry::shared()).unwrap();
    let version = b.add_theme(files);
    let mut pins: BTreeMap<_, _> =
        [(ns(THEME_NS), crate::ThemePin { id, version, extends: extends.iter().map(|e| ns(e)).collect() })].into();
    for parent in extends {
        pins.insert(ns(parent), theme_pin(b.add_theme(theme_files_of(parent, "P", &[]))));
    }
    set_deck_themes(&mut b, THEME_NS, pins);
    let (_, issues) = SourceBundle::from_zip(&b.to_zip().unwrap(), MAX, crate::test_registry::shared()).unwrap();
    assert_that!(issue_ids(&issues)).is_equal_to(vec!["issue-theme-pin-mismatch"]);
}

/// Des épinglages qui ne sont pas la fermeture du thème sont signalés.
#[test]
fn theme_pins_not_the_closure_give_issue() {
    let (t, extra) = (theme_files("T"), theme_files_of("autre", "A", &[]));
    let mut b = SourceBundle::new(roadmap(), crate::test_registry::shared()).unwrap();
    let pins = [(ns(THEME_NS), theme_pin(b.add_theme(t))), (ns("autre"), theme_pin(b.add_theme(extra)))];
    set_deck_themes(&mut b, THEME_NS, pins.into());
    let (_, issues) = SourceBundle::from_zip(&b.to_zip().unwrap(), MAX, crate::test_registry::shared()).unwrap();
    assert_that!(issues).is_equal_to(vec![Issue::theme_closure(&ns("autre"))]);
}

/// Deux `set_theme` concurrents depuis `x` (épinglages `{x, p}`) : A pose `y` (`{y}`), B pose `z`
/// (`{z, p}`). `theme` est un atome LWW, `themes` fusionne par entrée : `x` est retiré des deux
/// côtés, `p` par A seul (B ne l'écrit pas), `y` et `z` ajoutés. La fusion épingle `{y, z}`, ce qui
/// n'est la fermeture ni de `y` (`z` de trop) ni de `z` (`p` manque) : le cas est signalé.
#[test]
fn concurrent_set_theme_merge_reports_the_broken_closure() {
    let files = [
        theme_files_of("x", "X", &["p"]),
        theme_files_of("p", "P", &[]),
        theme_files_of("y", "Y", &[]),
        theme_files_of("z", "Z", &["p"]),
    ];
    let mut base = SourceBundle::new(roadmap(), crate::test_registry::shared()).unwrap();
    let versions: Vec<PackVersion> = files.iter().map(|f| base.add_theme(f.clone())).collect();
    let pin = |i: usize, extends: &[&str]| crate::ThemePin {
        id: THEME_ID,
        version: versions[i].clone(),
        extends: extends.iter().map(|e| ns(e)).collect(),
    };
    set_deck_themes(&mut base, "x", [(ns("x"), pin(0, &["p"])), (ns("p"), pin(1, &[]))].into());
    let zip = base.to_zip().unwrap();
    let open = || SourceBundle::from_zip(&zip, MAX, crate::test_registry::shared()).unwrap().0;
    let (mut a, mut b) = (open(), open());
    set_deck_themes(&mut a, "y", [(ns("y"), pin(2, &[]))].into());
    set_deck_themes(&mut b, "z", [(ns("z"), pin(3, &["p"])), (ns("p"), pin(1, &[]))].into());
    let (_, issues) = a.merge(b).unwrap();
    let deck = a.deck();
    assert_that!(deck.themes.keys().map(|n| n.as_str().to_owned()).collect::<Vec<_>>())
        .is_equal_to(vec!["y".to_owned(), "z".to_owned()]);
    let expected = match deck.theme.as_ref().map(crate::Namespace::as_str) {
        Some("y") => Issue::theme_closure(&ns("z")),
        Some("z") => Issue::theme_closure(&ns("p")),
        other => panic!("thème fusionné inattendu : {other:?}"),
    };
    assert_that!(crate::check_closure(deck.theme.as_ref(), &deck.themes)).is_err();
    assert_that!(issues.iter().filter(|i| i.message.id() == "issue-theme-closure").cloned().collect::<Vec<_>>())
        .is_equal_to(vec![expected]);
}
