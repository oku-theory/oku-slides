//! The built-in packs and themes ship both guides, and their shipped history stays built-in.

use std::collections::BTreeMap;

use oku_slides_core::{Op, PackFiles, PackKind, PackPin, ThemePin, kinds};
use oku_slides_render::BuiltinPack;
use rstest::rstest;
use serde_json::Value;
use speculoos::prelude::*;
use uuid::Uuid;

use crate::Session;
use crate::guide_origin::GuideOrigin;
use crate::library_kind::LibraryKind;
use crate::pack::{BASE_PACK, ESSENTIEL_PACK, PackManifest};
use crate::test_support::{builtin, fresh_dir, ns, opened_on};
use crate::theme::{
    NEUTRE_NAMESPACE, PAPIER_NAMESPACE, PAPIER_THEME, PRO_NAMESPACE, ThemeManifest, Themes, shipped_theme_versions,
};

/// The manifest of built-in pack `id`.
fn builtin_manifest(id: Uuid) -> PackManifest {
    builtin(id).manifest.clone()
}

/// The stored current version of the built-in theme of namespace `ns`.
fn builtin_theme(ns: &str) -> &'static crate::StoredVersion<ThemeManifest> {
    let found = Themes::builtins().iter().find(|t| t.manifest.namespace.as_str() == ns);
    found.unwrap_or_else(|| panic!("unknown built-in theme: {ns}"))
}

/// The manifest of the built-in theme of namespace `ns`.
fn builtin_theme_manifest(ns: &str) -> ThemeManifest {
    builtin_theme(ns).manifest.clone()
}

#[rstest]
#[case::base(BASE_PACK)]
#[case::essentiel(ESSENTIEL_PACK)]
fn builtin_pack_has_both_guides(#[case] id: Uuid) {
    let m = builtin_manifest(id);
    assert_that!((m.guides.edit.is_empty(), m.guides.use_.is_empty())).is_equal_to((false, false));
}

#[rstest]
#[case::papier("papier")]
#[case::pro("pro")]
#[case::neutre("neutre")]
fn builtin_theme_has_both_guides(#[case] ns: &str) {
    let m = builtin_theme_manifest(ns);
    assert_that!((m.guides.edit.is_empty(), m.guides.use_.is_empty())).is_equal_to((false, false));
}

/// Every built-in guide fits the default sending limit, so none is cut.
#[rstest]
#[case::base(builtin_manifest(BASE_PACK).guides)]
#[case::essentiel(builtin_manifest(ESSENTIEL_PACK).guides)]
#[case::papier(builtin_theme_manifest(PAPIER_NAMESPACE).guides)]
#[case::pro(builtin_theme_manifest(PRO_NAMESPACE).guides)]
#[case::neutre(builtin_theme_manifest(NEUTRE_NAMESPACE).guides)]
fn builtin_guides_fit_the_default_limit(#[case] guides: oku_slides_core::Guides) {
    let longest = guides.edit.chars().count().max(guides.use_.chars().count());
    assert_that!(longest).is_less_than(4000);
}

/// The base use guide names every base widget; the essentiel one every layout and profile.
#[test]
fn builtin_use_guides_name_every_widget_and_layout_of_their_pack() {
    let base = builtin_manifest(BASE_PACK).guides.use_;
    let essentiel = builtin_manifest(ESSENTIEL_PACK);
    let missing: Vec<String> = kinds::base::ALL
        .iter()
        .filter(|k| !base.contains(**k))
        .chain(kinds::essentiel::ALL.iter().filter(|k| !essentiel.guides.use_.contains(**k)))
        .map(|k| (*k).to_owned())
        .chain(essentiel.profiles.iter().filter(|p| !essentiel.guides.use_.contains(&p.name)).map(|p| p.name.clone()))
        .collect();
    assert_that!(missing).is_empty();
}

/// The shipped digest of built-in pack `pack` just before its current one.
fn previous_hex(pack: BuiltinPack) -> String {
    let shipped: Vec<&str> = pack.shipped_versions().collect();
    shipped[shipped.len() - 2].to_owned()
}

/// The previous shipped base: the current files without guides, under the parent before.
fn previous_base() -> PackFiles {
    let current = &builtin(BASE_PACK).files;
    let shipped: Vec<&str> = BuiltinPack::Base.shipped_versions().collect();
    let mut m = current.manifest().clone();
    m.remove("guides");
    m.insert("parent".into(), Value::String(format!("sha256:{}", shipped[shipped.len() - 3])));
    PackFiles::new(PackKind::Widgets, m, current.widgets().clone())
}

/// The previous shipped Papier: the current theme without guides and without parent.
fn previous_papier() -> PackFiles {
    let mut m = builtin_theme(PAPIER_NAMESPACE).files.manifest().clone();
    m.remove("guides");
    m.insert("parent".into(), Value::Null);
    PackFiles::theme(m)
}

/// A deck tab whose deck pins the previous shipped base.
fn deck_pinning_previous_base() -> (tempfile::TempDir, Session, PackFiles) {
    let data = fresh_dir("oku-slides-guides-builtin-");
    let mut s = opened_on(data.path());
    let old = previous_base();
    s.doc.sources.add_pack(old.clone());
    let pin = PackPin { id: BASE_PACK, version: old.version(), requires: Vec::new() };
    let op = Op::AddPack { kind: PackKind::Widgets, namespace: kinds::base::namespace(), pin };
    s.apply("Base", vec![op], None).unwrap();
    (data, s, old)
}

#[test]
fn previous_base_rebuilt_has_the_previous_shipped_digest() {
    assert_that!(previous_base().version().hex().to_owned()).is_equal_to(previous_hex(BuiltinPack::Base));
}

#[test]
fn guide_origin_deck_pinning_the_previous_base_is_builtin_and_asks_nothing() {
    let (_d, s, old) = deck_pinning_previous_base();
    let origin = s.guide_origin(BASE_PACK, &old.version(), false);
    assert_that!((origin, s.guides_to_approve().len())).is_equal_to((GuideOrigin::Builtin, 0));
}

#[test]
fn pack_updates_deck_pinning_the_previous_base_counts_one() {
    let (_d, s, _old) = deck_pinning_previous_base();
    assert_that!(s.pack_updates()).is_equal_to(1);
}

#[test]
fn guide_origin_shipped_base_digest_under_another_id_is_received() {
    let data = fresh_dir("oku-slides-guides-builtin-");
    let s = opened_on(data.path());
    let version = builtin(BASE_PACK).version.clone();
    assert_that!(s.guide_origin(Uuid::new_v4(), &version, false)).is_equal_to(GuideOrigin::Received);
}

#[test]
fn previous_papier_rebuilt_has_the_previous_shipped_digest() {
    let shipped: Vec<&str> = shipped_theme_versions().filter(|(n, _)| *n == PAPIER_NAMESPACE).map(|(_, h)| h).collect();
    assert_that!(previous_papier().version().hex()).is_equal_to(shipped[shipped.len() - 2]);
}

#[test]
fn guide_origin_deck_pinning_the_previous_papier_is_builtin_outdated_and_asks_nothing() {
    let data = fresh_dir("oku-slides-guides-builtin-");
    let mut s = opened_on(data.path());
    let old = previous_papier();
    s.doc.sources.add_theme(old.clone());
    let pin = ThemePin { id: PAPIER_THEME, version: old.version(), extends: Vec::new() };
    let themes = BTreeMap::from([(ns(PAPIER_NAMESPACE), pin)]);
    s.apply("Papier", vec![Op::SetTheme { theme: Some(ns(PAPIER_NAMESPACE)), themes }], None).unwrap();
    s.refresh_pack_updates();
    let origin = s.guide_origin(PAPIER_THEME, &old.version(), true);
    let outdated = s.deck_packs().unwrap().theme.outdated;
    assert_that!((origin, s.guides_to_approve().len(), outdated)).is_equal_to((GuideOrigin::Builtin, 0, true));
}

#[test]
fn guide_origin_papier_id_with_an_unshipped_digest_is_received() {
    let data = fresh_dir("oku-slides-guides-builtin-");
    let s = opened_on(data.path());
    let mut m = builtin_theme(PAPIER_NAMESPACE).files.manifest().clone();
    m.insert("name".into(), Value::String("Papier, edited".into()));
    let other = PackFiles::theme(m);
    assert_that!(s.guide_origin(PAPIER_THEME, &other.version(), true)).is_equal_to(GuideOrigin::Received);
}
