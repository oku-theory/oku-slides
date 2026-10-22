//! Les versions de packs dans le zip des sources.

use std::io::Cursor;

use super::*;
use crate::{Namespace, PackFiles, PackKind, PackPin, PackVersion};

/// Id du pack de test, celui que `pin_pack` épingle.
const PACK_ID: &str = "6f3c1e2a-8b4d-4c1e-9a57-2d0e5b7c9f10";

/// Un pack de test sous l'espace de noms `p` : manifeste minimal et ses fichiers de widgets.
fn pack(name: &str, widgets: &[(&str, &str)]) -> PackFiles {
    pack_in("p", &[], name, widgets)
}

/// Comme `pack`, sous l'espace de noms `ns` et avec les besoins `requires`.
fn pack_in(ns: &str, requires: &[&str], name: &str, widgets: &[(&str, &str)]) -> PackFiles {
    kinded_pack(PackKind::Widgets, ns, requires, name, widgets)
}

/// Un pack de test de sorte `kind`, sous l'espace de noms `ns`.
fn kinded_pack(kind: PackKind, ns: &str, requires: &[&str], name: &str, widgets: &[(&str, &str)]) -> PackFiles {
    let manifest = serde_json::json!({
        "id": PACK_ID, "name": name, "namespace": ns, "requires": requires,
        "created": "2026-10-18T00:00:00Z",
    });
    let widgets = widgets.iter().map(|(n, s)| (crate::WidgetFile::parse(n).unwrap(), (*s).to_owned())).collect();
    PackFiles::new(kind, manifest.as_object().cloned().unwrap(), widgets)
}

/// `roadmap` qui épingle sous `ns`, dans la carte de sa sorte, le pack `files`, et son bundle.
fn kinded_bundle(files: &PackFiles, ns: &str) -> SourceBundle {
    let mut deck = roadmap();
    let pin = PackPin { id: PACK_ID.parse().unwrap(), version: files.version(), requires: Vec::new() };
    let pins = match files.kind() {
        PackKind::Widgets => &mut deck.widgets,
        PackKind::Layouts => &mut deck.layouts,
    };
    pins.insert(Namespace::parse(ns).unwrap(), pin);
    let mut b = SourceBundle::new(deck, crate::test_registry::shared()).unwrap();
    b.add_pack(files.clone());
    b
}

/// Les noms des entrées du zip `bytes`.
fn zip_entry_names(bytes: &[u8]) -> Vec<String> {
    let za = zip::ZipArchive::new(Cursor::new(bytes.to_vec())).unwrap();
    za.file_names().map(|n| n.unwrap().into_owned()).collect()
}

/// Épingle `version` sous l'espace de noms de test `p`, à la place du précédent.
fn pin_pack(deck: &mut crate::Deck, version: PackVersion) {
    let pin = PackPin { id: PACK_ID.parse().unwrap(), version, requires: Vec::new() };
    deck.widgets = std::collections::BTreeMap::from([(Namespace::parse("p").unwrap(), pin)]);
}

/// La version épinglée sous `p`, s'il y en a une.
fn pinned(deck: &crate::Deck) -> Option<PackVersion> {
    deck.widgets.get(&Namespace::parse("p").unwrap()).map(|pin| pin.version.clone())
}

/// `roadmap` qui référence `files`, et son bundle qui l'embarque.
fn bundle_with(files: &PackFiles) -> SourceBundle {
    let mut deck = roadmap();
    pin_pack(&mut deck, files.version());
    let mut b = SourceBundle::new(deck, crate::test_registry::shared()).unwrap();
    b.add_pack(files.clone());
    b
}

#[test]
fn to_zip_then_from_zip_keeps_packs_and_each_widget_file() {
    let files = pack("P", &[("kpi", "#let widget = none\n"), ("_commun", "#let x = 1\n")]);
    let b = bundle_with(&files);
    let zip = b.to_zip().unwrap();
    let za = zip::ZipArchive::new(Cursor::new(zip.clone())).unwrap();
    let hex = files.version().hex().to_owned();
    for name in ["widgets.json", "widgets/kpi.typ", "widgets/_commun.typ"] {
        assert_that!(za.file_names().any(|n| n.unwrap() == format!("widgets/{hex}/{name}"))).named(name).is_true();
    }
    let (back, issues) = SourceBundle::from_zip(&zip, MAX, crate::test_registry::shared()).unwrap();
    assert_that!(issues).is_empty();
    assert_that!(back.packs.get(&files.version())).is_equal_to(Some(&files));
}

#[test]
fn to_zip_of_a_pack_without_widgets_writes_no_widgets_folder() {
    let files = pack("P", &[]);
    let zip = bundle_with(&files).to_zip().unwrap();
    let za = zip::ZipArchive::new(Cursor::new(zip)).unwrap();
    assert_that!(za.file_names().any(|n| n.unwrap().contains("/widgets/"))).is_false();
}

#[test]
fn from_zip_pack_whose_content_differs_from_its_hash_is_discarded_and_reported() {
    let loro = deck_loro();
    let forged = "f".repeat(64);
    let name = format!("widgets/{forged}/widgets.json");
    let entries: &[(&str, &[u8])] =
        &[("manifest.json", MANIFEST), ("deck.loro", &loro), (name.as_str(), br#"{"name":"P"}"#)];
    let (b, issues) = SourceBundle::from_zip(&zip(entries), MAX, crate::test_registry::shared()).unwrap();
    assert_that!(b.packs.is_empty()).is_true();
    let ids: Vec<&str> = issues.iter().map(|i| i.message.id()).collect();
    assert_that!(ids).is_equal_to(vec!["issue-pack-version-mismatch"]);
}

#[rstest::rstest]
#[case("widgets/abc/widgets.json")]
#[case("widgets/FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF/widgets.json")]
#[case("widgets/../widgets.json")]
#[case(&format!("layouts/{}/widgets/kpi.typ", "a".repeat(64)))]
#[case("layouts/abc/layouts.json")]
fn from_zip_pack_with_an_invalid_path_returns_bad_name(#[case] name: &str) {
    let loro = deck_loro();
    let entries: &[(&str, &[u8])] = &[("manifest.json", MANIFEST), ("deck.loro", &loro), (name, b"{}")];
    let err = SourceBundle::from_zip(&zip(entries), MAX, crate::test_registry::shared()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::BadName);
}

#[test]
fn from_zip_deck_referencing_an_absent_pack_reports_it() {
    let files = pack("P", &[]);
    let mut b = bundle_with(&files);
    b.packs.clear();
    let (_, issues) = SourceBundle::from_zip(&b.to_zip().unwrap(), MAX, crate::test_registry::shared()).unwrap();
    let ids: Vec<&str> = issues.iter().map(|i| i.message.id()).collect();
    assert_that!(ids).is_equal_to(vec!["issue-pack-version-missing"]);
}

#[test]
fn from_zip_packs_count_in_the_total_limit() {
    let loro = deck_loro();
    let files = pack(&"x".repeat(4096), &[]);
    let name = format!("widgets/{}/widgets.json", files.version().hex());
    let json = files.pack_json().unwrap();
    let entries: &[(&str, &[u8])] = &[("manifest.json", MANIFEST), ("deck.loro", &loro), (name.as_str(), &json)];
    let limits = SourceLimits { total_bytes: MANIFEST.len() + loro.len() + 100, ..MAX };
    let err = SourceBundle::from_zip(&zip(entries), limits, crate::test_registry::shared()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::TooLarge);
}

#[test]
fn merge_unions_the_packs_of_both_bundles() {
    let (p, q) = (pack("P", &[]), pack("Q", &[("kpi", "#let registry = ()")]));
    let mut a = bundle_with(&p);
    let mut b = SourceBundle::from_zip(&a.to_zip().unwrap(), MAX, crate::test_registry::shared()).unwrap().0;
    b.add_pack(q.clone());
    a.merge(b).unwrap();
    assert_that!(a.packs.keys().cloned().collect::<Vec<_>>()).is_equal_to({
        let mut v = vec![p.version(), q.version()];
        v.sort();
        v
    });
}

/// Le mode historique restaure aussi le pack d'une version passée : toute
/// version que le replica a référencée reste dans le fichier.
#[test]
fn versions_referenced_by_the_history_are_kept_on_save() {
    let (p, q) = (pack("P", &[]), pack("Q", &[]));
    let mut b = bundle_with(&p);
    b.add_pack(q.clone());
    let mut next = b.deck().clone();
    pin_pack(&mut next, q.version());
    b.apply(next, "pack", None).unwrap();
    let (back, _) = SourceBundle::from_zip(&b.to_zip().unwrap(), MAX, crate::test_registry::shared()).unwrap();
    assert_that!(back.packs.contains_key(&p.version()) && back.packs.contains_key(&q.version())).is_true();
    let preview = back.history();
    let mut restored = back;
    restored.restore(&preview, 0, "restaurer").unwrap();
    assert_that!(pinned(restored.deck())).is_equal_to(Some(p.version()));
}

#[test]
fn from_zip_pack_with_widgets_but_no_pack_json_is_discarded_and_reported() {
    let loro = deck_loro();
    let name = format!("widgets/{}/widgets/kpi.typ", "a".repeat(64));
    let entries: &[(&str, &[u8])] =
        &[("manifest.json", MANIFEST), ("deck.loro", &loro), (name.as_str(), b"#let registry = ()")];
    let (b, issues) = SourceBundle::from_zip(&zip(entries), MAX, crate::test_registry::shared()).unwrap();
    assert_that!(b.packs.is_empty()).is_true();
    let ids: Vec<&str> = issues.iter().map(|i| i.message.id()).collect();
    assert_that!(ids).is_equal_to(vec!["issue-pack-version-mismatch"]);
}

/// t-328 : les entrées de répertoire `widgets/` et `widgets/<hex>/` sont ignorées.
#[rstest::rstest]
#[case("widgets/")]
#[case("layouts/")]
#[case(&format!("layouts/{}/layouts/", "a".repeat(64)))]
#[case(&format!("widgets/{}/", "a".repeat(64)))]
#[case(&format!("widgets/{}/widgets/", "a".repeat(64)))]
#[case(&format!("widgets/{}/widgets.typ", "a".repeat(64)))]
fn from_zip_bare_packs_directory_entries_are_ignored(#[case] name: &str) {
    let loro = deck_loro();
    let entries: &[(&str, &[u8])] = &[("manifest.json", MANIFEST), ("deck.loro", &loro), (name, b"")];
    let (b, issues) = SourceBundle::from_zip(&zip(entries), MAX, crate::test_registry::shared()).unwrap();
    assert_that!(b.packs.is_empty()).is_true();
    assert_that!(issues.is_empty()).is_true();
}

/// t-329 : un nom invalide sous `widgets/<hex>/widgets/` refuse le zip.
#[rstest::rstest]
#[case("widgets/Kpi.typ")]
#[case("widgets/_autre.typ")]
#[case("widgets/kpi")]
#[case("widgets/a/b.typ")]
#[case("autre/x.typ")]
fn from_zip_pack_with_an_invalid_widget_path_returns_bad_name(#[case] file: &str) {
    let loro = deck_loro();
    let name = format!("widgets/{}/{file}", "a".repeat(64));
    let entries: &[(&str, &[u8])] = &[("manifest.json", MANIFEST), ("deck.loro", &loro), (name.as_str(), b"x")];
    let err = SourceBundle::from_zip(&zip(entries), MAX, crate::test_registry::shared()).unwrap_err();
    assert_that!(err.kind()).is_equal_to(SourcesErrorKind::BadName);
}

#[test]
fn from_zip_pack_whose_widget_file_was_altered_is_discarded_and_reported() {
    let files = pack("P", &[("kpi", "#let widget = none\n")]);
    let bytes = bundle_with(&files).to_zip().unwrap();
    // Réécrit le zip avec un `kpi.typ` modifié : l'empreinte ne correspond plus.
    let mut za = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    for i in 0..za.len() {
        let mut f = za.by_index(i).unwrap();
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut f, &mut data).unwrap();
        if f.name().unwrap().ends_with("widgets/kpi.typ") {
            data.extend_from_slice(b"// altere\n");
        }
        entries.push((f.name().unwrap().into_owned(), data));
    }
    let refs: Vec<(&str, &[u8])> = entries.iter().map(|(n, d)| (n.as_str(), d.as_slice())).collect();
    let (b, issues) = SourceBundle::from_zip(&zip(&refs), MAX, crate::test_registry::shared()).unwrap();
    assert_that!(b.packs.is_empty()).is_true();
    // Le deck référence cette version : elle est dite altérée, pas en plus manquante (t-355).
    let ids: Vec<&str> = issues.iter().map(|i| i.message.id()).collect();
    assert_that!(ids).is_equal_to(vec!["issue-pack-version-mismatch"]);
}

/// Un zip à deux entrées du même nom `name`, l'une `first`, l'autre `last`
/// (les noms sont rendus égaux après coup : `ZipWriter` refuse un doublon).
fn zip_with_duplicate(name: &str, first: &[u8], last: &[u8]) -> Vec<u8> {
    let loro = deck_loro();
    let twin = name.replace("widgets.json", "widgets.jsoX");
    let entries: &[(&str, &[u8])] =
        &[("manifest.json", MANIFEST), ("deck.loro", &loro), (name, first), (twin.as_str(), last)];
    let bytes = zip(entries);
    let (from, to) = (twin.as_bytes(), name.as_bytes());
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(from) {
            out.extend_from_slice(to);
            i += from.len();
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    out
}

/// t-355 : une entrée `widgets/<hex>/widgets.json` en double garde la dernière
/// (règle de la crate zip) ; altérée, la version est écartée et signalée une fois.
#[rstest::rstest]
#[case::last_altered(false, false)]
#[case::last_intact(true, true)]
fn from_zip_duplicated_pack_entry_is_verified_on_the_last_copy(#[case] first_altered: bool, #[case] kept: bool) {
    let files = pack("P", &[]);
    let name = format!("widgets/{}/widgets.json", files.version().hex());
    let good = files.pack_json().unwrap();
    let altered = pack("Q", &[]).pack_json().unwrap();
    let (first, last) = if first_altered { (&altered, &good) } else { (&good, &altered) };
    let (b, issues) =
        SourceBundle::from_zip(&zip_with_duplicate(&name, first, last), MAX, crate::test_registry::shared()).unwrap();
    assert_that!(b.packs.get(&files.version()) == Some(&files)).is_equal_to(kept);
    let ids: Vec<&str> = issues.iter().map(|i| i.message.id()).collect();
    let expected: Vec<&str> = if kept { vec![] } else { vec!["issue-pack-version-mismatch"] };
    assert_that!(ids).is_equal_to(expected);
}

/// Le bundle `b` passé au pack `files`, en une étape nommée d'après lui
/// (deux commits de même libellé se fondent en une version de l'historique).
fn set_pack(b: &mut SourceBundle, files: &PackFiles) {
    let version = b.add_pack(files.clone());
    let mut next = b.deck().clone();
    pin_pack(&mut next, version);
    b.apply(next, files.version().as_str(), None).unwrap();
}

/// Une copie de `b` par son zip : même replica, même historique.
fn copy(b: &SourceBundle) -> SourceBundle {
    SourceBundle::from_zip(&b.to_zip().unwrap(), MAX, crate::test_registry::shared()).unwrap().0
}

/// t-355 : deux copies qui changent de pack en parallèle convergent, quel que
/// soit le sens de la fusion, vers le même pack et l'union des versions.
#[test]
fn concurrent_template_changes_converge_in_both_directions() {
    let base = bundle_with(&pack("O", &[]));
    let (p, q) = (pack("P", &[]), pack("Q", &[("kpi", "#let registry = ()")]));
    let (mut a, mut b) = (copy(&base), copy(&base));
    set_pack(&mut a, &p);
    set_pack(&mut b, &q);
    let (a_then_b, b_then_a) = (copy(&a), copy(&b));
    a.merge(b_then_a).unwrap();
    b.merge(a_then_b).unwrap();
    assert_that!(a.deck().widgets).is_equal_to(b.deck().widgets.clone());
    assert_that!(a.packs.keys().collect::<Vec<_>>()).is_equal_to(b.packs.keys().collect::<Vec<_>>());
    assert_that!(a.packs.len()).is_equal_to(3);
    let chosen = pinned(a.deck());
    assert_that!(chosen == Some(p.version()) || chosen == Some(q.version())).is_true();
}

/// t-355 : une version intermédiaire (ni la première ni la courante) reste dans
/// le bundle et son zip, et la restaurer la retrouve.
#[test]
fn intermediate_template_version_stays_in_the_bundle() {
    let (p, q, r) = (pack("P", &[]), pack("Q", &[]), pack("R", &[]));
    let mut b = bundle_with(&p);
    set_pack(&mut b, &q);
    set_pack(&mut b, &r);
    let back = copy(&b);
    assert_that!(back.packs.get(&q.version())).is_equal_to(Some(&q));
    let preview = back.history();
    let at_q = (0..preview.len())
        .find(|&i| pinned(&preview.deck_at(i).unwrap().0) == Some(q.version()))
        .unwrap_or_else(|| panic!("aucune étape de l'historique au pack Q"));
    let mut restored = copy(&back);
    let issues = restored.restore(&preview, at_q, "restaurer").unwrap();
    assert_that!(issues.iter().any(|i| i.message.id() == "issue-pack-version-missing")).is_false();
}

/// Le zip de `bundle`, relu : les problèmes que signale la relecture.
fn issue_ids_after_roundtrip(bundle: &SourceBundle) -> Vec<String> {
    let (_, issues) = SourceBundle::from_zip(&bundle.to_zip().unwrap(), MAX, crate::test_registry::shared()).unwrap();
    issues.iter().map(|i| i.message.id().to_owned()).collect()
}

/// Un épinglage de `files` sous `ns` avec les besoins `requires`.
fn pin_of(files: &PackFiles, requires: &[&str]) -> PackPin {
    let requires = requires.iter().map(|r| Namespace::parse(r).unwrap()).collect();
    PackPin { id: PACK_ID.parse().unwrap(), version: files.version(), requires }
}

#[test]
fn sources_pack_written_under_widgets_dir() {
    let files = pack("P", &[]);
    let zip = bundle_with(&files).to_zip().unwrap();
    let za = zip::ZipArchive::new(Cursor::new(zip)).unwrap();
    let wanted = format!("widgets/{}/widgets.json", files.version().hex());
    assert_that!(za.file_names().any(|n| n.unwrap() == wanted)).is_true();
}

#[test]
fn sources_pin_with_other_requires_than_manifest_reports_mismatch() {
    let files = pack("P", &[]);
    let mut b = bundle_with(&files);
    let mut deck = b.deck().clone();
    let base = Namespace::parse("base").unwrap();
    let pin = deck.widgets.get_mut(&Namespace::parse("p").unwrap()).unwrap();
    pin.requires = vec![base.clone()];
    deck.widgets.insert(base, pin_of(&files, &[]));
    b.apply_silent(deck).unwrap();
    assert_that!(issue_ids_after_roundtrip(&b)).contains("issue-pack-pin-mismatch".to_owned());
}

#[test]
fn sources_pin_with_other_namespace_than_manifest_reports_mismatch() {
    let files = pack_in("other", &[], "P", &[]);
    let b = bundle_with(&files);
    assert_that!(issue_ids_after_roundtrip(&b)).is_equal_to(vec!["issue-pack-pin-mismatch".to_owned()]);
}

#[test]
fn sources_pin_requiring_an_unpinned_namespace_reports_it() {
    let files = pack_in("p", &["base"], "P", &[]);
    let mut deck = roadmap();
    deck.widgets.insert(Namespace::parse("p").unwrap(), pin_of(&files, &["base"]));
    let mut b = SourceBundle::new(deck, crate::test_registry::shared()).unwrap();
    b.add_pack(files);
    assert_that!(issue_ids_after_roundtrip(&b)).is_equal_to(vec!["issue-pack-requirement-missing".to_owned()]);
}

#[test]
fn sources_same_missing_version_pinned_twice_is_reported_once() {
    let files = pack("P", &[]);
    let mut b = bundle_with(&files);
    let mut deck = b.deck().clone();
    deck.widgets.insert(Namespace::parse("q").unwrap(), pin_of(&files, &[]));
    b.apply_silent(deck).unwrap();
    b.packs.clear();
    let ids = issue_ids_after_roundtrip(&b);
    assert_that!(ids.iter().filter(|i| *i == "issue-pack-version-missing").count()).is_equal_to(1);
}

#[test]
fn sources_layouts_pack_round_trips_under_layouts_dir() {
    let files = kinded_pack(PackKind::Layouts, "essentiel", &[], "E", &[("content", "#let widget = none")]);
    let zip = kinded_bundle(&files, "essentiel").to_zip().unwrap();
    let hex = files.version().hex().to_owned();
    let names = zip_entry_names(&zip);
    let (back, issues) = SourceBundle::from_zip(&zip, MAX, crate::test_registry::shared()).unwrap();
    let expected = [format!("layouts/{hex}/layouts.json"), format!("layouts/{hex}/layouts/content.typ")];
    assert_that!(expected.iter().all(|e| names.contains(e))).is_true();
    assert_that!(issues).is_empty();
    assert_that!(back.packs.get(&files.version())).is_equal_to(Some(&files));
    assert_that!(back.packs.values().next().map(PackFiles::kind)).is_equal_to(Some(PackKind::Layouts));
}

#[test]
fn sources_pin_of_layouts_on_widget_files_is_an_issue() {
    let files = pack("P", &[]);
    let mut b = kinded_bundle(&files, "p");
    let mut deck = b.deck().clone();
    let pin = deck.widgets.remove(&Namespace::parse("p").unwrap()).unwrap();
    deck.layouts.insert(Namespace::parse("p").unwrap(), pin);
    b.apply_silent(deck).unwrap();
    let keys: Vec<String> = b.embedded_issues().iter().map(|i| i.message.id().to_owned()).collect();
    assert_that!(keys).is_equal_to(vec!["issue-pack-kind-mismatch".to_owned()]);
}

#[test]
fn sources_namespace_pinned_under_both_kinds_is_a_conflict_issue() {
    let widgets = pack("P", &[]);
    let layouts = kinded_pack(PackKind::Layouts, "p", &[], "P", &[]);
    let mut b = kinded_bundle(&widgets, "p");
    let mut deck = b.deck().clone();
    let pin = PackPin { id: PACK_ID.parse().unwrap(), version: layouts.version(), requires: Vec::new() };
    deck.layouts.insert(Namespace::parse("p").unwrap(), pin);
    b.add_pack(layouts);
    b.apply_silent(deck).unwrap();
    let keys: Vec<String> = b.embedded_issues().iter().map(|i| i.message.id().to_owned()).collect();
    assert_that!(keys).is_equal_to(vec!["issue-pack-kind-conflict".to_owned()]);
}

#[test]
fn sources_pin_of_widgets_on_layout_files_is_an_issue() {
    let files = kinded_pack(PackKind::Layouts, "p", &[], "P", &[]);
    let mut b = kinded_bundle(&files, "p");
    let mut deck = b.deck().clone();
    let pin = deck.layouts.remove(&Namespace::parse("p").unwrap()).unwrap();
    deck.widgets.insert(Namespace::parse("p").unwrap(), pin);
    b.apply_silent(deck).unwrap();
    assert_that!(issue_ids_after_roundtrip(&b)).is_equal_to(vec!["issue-pack-kind-mismatch".to_owned()]);
}

#[test]
fn sources_layouts_pin_requiring_an_unpinned_widget_pack_reports_it() {
    let files = kinded_pack(PackKind::Layouts, "p", &["base"], "P", &[]);
    let mut deck = roadmap();
    deck.layouts.insert(Namespace::parse("p").unwrap(), pin_of(&files, &["base"]));
    let mut b = SourceBundle::new(deck, crate::test_registry::shared()).unwrap();
    b.add_pack(files);
    assert_that!(issue_ids_after_roundtrip(&b)).is_equal_to(vec!["issue-pack-requirement-missing".to_owned()]);
}

#[test]
fn from_zip_layouts_entry_named_widgets_json_is_ignored() {
    // Un pack de widgets rangé sous `layouts/` : son manifeste n'est pas `layouts.json`, il est ignoré.
    let files = pack("P", &[]);
    let loro = deck_loro();
    let name = format!("layouts/{}/widgets.json", files.version().hex());
    let json = files.pack_json().unwrap();
    let entries: &[(&str, &[u8])] = &[("manifest.json", MANIFEST), ("deck.loro", &loro), (name.as_str(), &json)];
    let (b, issues) = SourceBundle::from_zip(&zip(entries), MAX, crate::test_registry::shared()).unwrap();
    assert_that!((b.packs.is_empty(), issues.is_empty())).is_equal_to((true, true));
}

#[test]
fn from_zip_widgets_files_relabelled_as_layouts_fail_their_hash() {
    // Les octets d'un pack de widgets sous `layouts/<même hex>/layouts.json` : la sorte change l'empreinte.
    let files = pack("P", &[]);
    let loro = deck_loro();
    let name = format!("layouts/{}/layouts.json", files.version().hex());
    let json = files.pack_json().unwrap();
    let entries: &[(&str, &[u8])] = &[("manifest.json", MANIFEST), ("deck.loro", &loro), (name.as_str(), &json)];
    let (b, issues) = SourceBundle::from_zip(&zip(entries), MAX, crate::test_registry::shared()).unwrap();
    let ids: Vec<&str> = issues.iter().map(|i| i.message.id()).collect();
    assert_that!((b.packs.is_empty(), ids)).is_equal_to((true, vec!["issue-pack-version-mismatch"]));
}
