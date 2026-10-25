//! Fabrique de session pour les tests, ouverte sur `fixtures/roadmap.json`.

use std::sync::Arc;

use oku_slides_core::WidgetRegistry;

use crate::{Fonts, Platform, Session, Settings, builtin_versions};

/// Le registre de base figé (`fixtures/widgets/builtin-registry.json`).
pub(crate) fn registry() -> Arc<WidgetRegistry> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/builtin-registry.json");
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path} : {e}"));
    let probe = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path} : {e}"));
    Arc::new(WidgetRegistry::from_probe(probe).unwrap_or_else(|e| panic!("{path} : {e}")))
}

/// Un dossier neuf, créé atomiquement sous un nom aléatoire (t-264).
///
/// Jamais celui d'un lancement précédent, même quand macOS recycle le PID
/// (nextest lance un processus par test). Supprimé au `drop`, sauf `keep()`.
pub(crate) fn fresh_dir(prefix: &str) -> tempfile::TempDir {
    tempfile::Builder::new().prefix(prefix).tempdir().unwrap_or_else(|e| panic!("dossier de test impossible : {e}"))
}

/// Une session isolée, en français, ouverte sur `fixtures/roadmap.json`.
pub(crate) fn session() -> Session {
    // Sans reprise activée, la session n'écrit rien dans son dossier de
    // données : il peut disparaître une fois la session construite.
    let data = fresh_dir("oku-slides-session-test-support-");
    opened_on(data.path())
}

/// Une session isolée sur `data`, en français, ouverte sur `fixtures/roadmap.json`.
///
/// Pour les tests qui écrivent dans la bibliothèque : `data` reste à l'appelant.
pub(crate) fn opened_on(data: &std::path::Path) -> Session {
    let mut s = session_on(data);
    let fixture = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"));
    s.open(&fixture).unwrap_or_else(|e| panic!("ouverture du fixture impossible : {}", e.one_line()));
    s
}

/// Une session isolée dans la langue `locale`, ouverte sur `fixtures/roadmap.json`, comme [`session`].
pub(crate) fn session_in_locale(locale: &str) -> Session {
    let data = fresh_dir("oku-slides-session-test-support-");
    let mut settings = settings_on(data.path());
    settings.i18n.locale = Some(locale.into());
    let mut s = session_with(settings);
    let fixture = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/roadmap.json"));
    s.open(&fixture).unwrap_or_else(|e| panic!("ouverture du fixture impossible : {}", e.one_line()));
    s
}

/// La version intégrée du pack `id` (cherchée par id, jamais par rang).
pub(crate) fn builtin(id: uuid::Uuid) -> &'static crate::library_kind::StoredVersion<crate::pack::PackManifest> {
    builtin_versions().iter().find(|b| b.manifest.id == id).unwrap_or_else(|| panic!("pack intégré inconnu : {id}"))
}

/// Un espace de noms des tests.
pub(crate) fn ns(s: &str) -> oku_slides_core::Namespace {
    oku_slides_core::Namespace::parse(s).unwrap_or_else(|| panic!("espace de noms invalide : {s}"))
}

/// L'espace de noms d'un pack des tests, propre à son id : celui des packs
/// intégrés pour leurs ids, sinon `t<id en hex>` (un par pack dans une bibliothèque).
pub(crate) fn namespace_of(id: uuid::Uuid) -> oku_slides_core::Namespace {
    match builtin_versions().iter().find(|b| b.manifest.id == id) {
        Some(b) => b.manifest.namespace.clone(),
        None => ns(&format!("t{}", id.simple())),
    }
}

/// Un manifeste sans profils ni exemples : pack `id`, nom `name`, parente `parent`, créé à `created`.
///
/// Son espace de noms est celui de son id ([`namespace_of`]), sans besoins.
pub(crate) fn manifest(
    id: uuid::Uuid,
    name: &str,
    parent: Option<&oku_slides_core::PackVersion>,
    created: &str,
) -> crate::pack::PackManifest {
    crate::pack::PackManifest {
        id,
        name: name.into(),
        namespace: namespace_of(id),
        requires: vec![],
        description: String::new(),
        parent: parent.cloned(),
        created: created.into(),
        scope: oku_slides_core::PackScope::Library,
        profiles: Vec::new(),
        examples: std::collections::BTreeMap::default(),
        styles: std::collections::BTreeMap::default(),
        tokens: Vec::new(),
    }
}

/// Des fichiers de widgets, par nom.
pub(crate) fn widget_files(
    entries: &[(&str, &str)],
) -> std::collections::BTreeMap<oku_slides_core::WidgetFile, String> {
    entries
        .iter()
        .map(|(n, s)| {
            let name = oku_slides_core::WidgetFile::parse(n).unwrap_or_else(|| panic!("nom invalide : {n}"));
            (name, (*s).to_owned())
        })
        .collect()
}

/// L'épinglage du deck de `s` sous l'espace de noms `namespace`, de l'une ou l'autre sorte.
pub(crate) fn pin(s: &Session, namespace: &str) -> Option<oku_slides_core::PackPin> {
    s.deck().all_pins().find(|(_, n, _)| n.as_str() == namespace).map(|(_, _, pin)| pin.clone())
}

/// Un manifeste sans profils ni exemples du pack `id` sous l'espace de noms `namespace`, avec ces besoins.
pub(crate) fn manifest_ns(id: uuid::Uuid, namespace: &str, requires: &[&str]) -> crate::pack::PackManifest {
    crate::pack::PackManifest {
        namespace: ns(namespace),
        requires: requires.iter().map(|r| ns(r)).collect(),
        ..manifest(id, namespace, None, "2026-10-01T10:00:00Z")
    }
}

/// Les fichiers de `manifest`, pack de widgets sans widgets.
pub(crate) fn files_of(manifest: &crate::pack::PackManifest) -> oku_slides_core::PackFiles {
    manifest
        .to_files(oku_slides_core::PackKind::Widgets, std::collections::BTreeMap::default())
        .unwrap_or_else(|e| panic!("manifeste : {e}"))
}

/// Ajoute `manifest` (et ses fichiers de widgets), pack de widgets, à `library` ; son épinglage.
pub(crate) fn add_version(
    library: &crate::PackLibrary,
    manifest: &crate::pack::PackManifest,
    widgets: &[(&str, &str)],
) -> oku_slides_core::PackPin {
    add_version_of(library, oku_slides_core::PackKind::Widgets, manifest, widgets)
}

/// Ajoute `manifest` (et ses fichiers), pack de sorte `kind`, à `library` ; son épinglage.
pub(crate) fn add_version_of(
    library: &crate::PackLibrary,
    kind: oku_slides_core::PackKind,
    manifest: &crate::pack::PackManifest,
    widgets: &[(&str, &str)],
) -> oku_slides_core::PackPin {
    let files = manifest.to_files(kind, widget_files(widgets)).unwrap_or_else(|e| panic!("manifeste : {e}"));
    library.add_version(manifest.id, &files).unwrap_or_else(|e| panic!("version : {}", e.one_line()));
    oku_slides_core::PackPin { id: manifest.id, version: files.version(), requires: manifest.requires.clone() }
}

/// L'op qui épingle `pin`, version de `manifest`, pack de widgets, sous son espace de noms.
pub(crate) fn add_pack(manifest: &crate::pack::PackManifest, pin: &oku_slides_core::PackPin) -> oku_slides_core::Op {
    let kind = oku_slides_core::PackKind::Widgets;
    oku_slides_core::Op::AddPack { kind, namespace: manifest.namespace.clone(), pin: pin.clone() }
}

/// L'id du pack de widgets `atelier` des tests (voir [`pin_atelier`]).
pub(crate) const ATELIER: uuid::Uuid = uuid::Uuid::from_u128(0xa7e11e40_0000_4000_8000_000000000001);

/// Ajoute à la bibliothèque de `s` le pack de widgets `atelier`, vide, qui requiert `base`, et le deck l'épingle.
pub(crate) fn pin_atelier(s: &mut Session) {
    let m = manifest_ns(ATELIER, "atelier", &["base"]);
    let pin = add_version(&s.library, &m, &[]);
    s.apply("Atelier", vec![add_pack(&m, &pin)], None).unwrap_or_else(|e| panic!("atelier : {}", e.one_line()));
}

/// Une bibliothèque des packs sous `data` : `widgets/` et `layouts/`, comme celle de l'app.
pub(crate) fn pack_library(data: &std::path::Path) -> crate::PackLibrary {
    let storage = crate::Storage::local().unwrap_or_else(|e| panic!("stockage : {e}"));
    let themes = crate::ThemeLibrary::new(Some(data.join("themes")), storage.clone());
    crate::PackLibrary::new(Some(data.join("widgets")), Some(data.join("layouts")), themes, storage)
}

/// Une bibliothèque sur un dossier neuf, gardé tant qu'elle vit.
pub(crate) struct TestLibrary {
    _dir: tempfile::TempDir,
    library: crate::PackLibrary,
}

impl std::ops::Deref for TestLibrary {
    type Target = crate::PackLibrary;

    fn deref(&self) -> &Self::Target {
        &self.library
    }
}

/// Une bibliothèque qui a, pour chaque `(espace de noms, besoins)`, un pack sans widgets.
pub(crate) fn library_with(packs: &[(&str, &[&str])]) -> TestLibrary {
    let dir = fresh_dir("oku-slides-library-with-");
    let storage = crate::Storage::local().unwrap_or_else(|e| panic!("stockage : {e}"));
    let themes = crate::ThemeLibrary::new(Some(dir.path().join("themes")), storage.clone());
    let library =
        crate::PackLibrary::new(Some(dir.path().join("widgets")), Some(dir.path().join("layouts")), themes, storage);
    for (namespace, requires) in packs {
        add_version(&library, &manifest_ns(uuid::Uuid::new_v4(), namespace, requires), &[]);
    }
    TestLibrary { _dir: dir, library }
}

/// L'épinglage de la version courante du pack `namespace` de `library`.
pub(crate) fn current_pin(library: &crate::PackLibrary, namespace: &str) -> oku_slides_core::PackPin {
    crate::deck_closure::current_pin(library, oku_slides_core::PackKind::Widgets, &ns(namespace))
        .unwrap_or_else(|e| panic!("{}", e.one_line()))
}

/// L'op qui épingle la version courante du pack de widgets `namespace` de `library`.
pub(crate) fn add_current(library: &crate::PackLibrary, namespace: &str) -> oku_slides_core::Op {
    let kind = oku_slides_core::PackKind::Widgets;
    oku_slides_core::Op::AddPack { kind, namespace: ns(namespace), pin: current_pin(library, namespace) }
}

/// Un deck vide qui épingle la version courante des packs `namespaces` de `library`.
pub(crate) fn deck_pinning(library: &crate::PackLibrary, namespaces: &[&str]) -> oku_slides_core::Deck {
    let mut deck = oku_slides_core::Deck::empty();
    for n in namespaces {
        deck.widgets.insert(ns(n), current_pin(library, n));
    }
    deck
}

/// Les espaces de noms des `AddPack` de `ops`, dans l'ordre.
pub(crate) fn namespaces_added(ops: &[oku_slides_core::Op]) -> Vec<String> {
    ops.iter()
        .filter_map(|op| match op {
            oku_slides_core::Op::AddPack { namespace, .. } => Some(namespace.to_string()),
            _ => None,
        })
        .collect()
}

/// Source du widget de pack `frise` des tests (`fixtures/widgets/frise-test.typ`) :
/// liste `etapes` (date, texte), quatre étapes par défaut, `max: 8`.
pub(crate) const FRISE_TEST: &str = include_str!("../../../fixtures/widgets/frise-test.typ");

/// Écrit le widget `frise` des tests dans le pack en cours de `s` (le pack `deck`
/// du deck, `deck/frise`, ou celui des modèles en cours d'édition).
pub(crate) fn set_frise_widget(s: &mut Session) {
    s.set_widget(None, "frise", FRISE_TEST.into()).unwrap_or_else(|e| panic!("frise : {}", e.one_line()));
}

/// Source du widget de pack `plan` des tests (`fixtures/widgets/plan-test.typ`) : étapes
/// (`titre`) à liste imbriquée `taches` (`texte`, `fait`) ; deux étapes par défaut.
pub(crate) const PLAN_TEST: &str = include_str!("../../../fixtures/widgets/plan-test.typ");

/// Écrit le widget `plan` des tests dans le pack en cours de `s` (en mode deck, `deck/plan`).
pub(crate) fn set_plan_widget(s: &mut Session) {
    s.set_widget(None, "plan", PLAN_TEST.into()).unwrap_or_else(|e| panic!("plan : {}", e.one_line()));
}

/// Les réglages de test sur `data` : en français, decks dans `data/decks`.
pub(crate) fn settings_on(data: &std::path::Path) -> Settings {
    let mut settings = Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
    settings.paths.data = Some(data.to_owned());
    // Jamais dans le vrai dossier Documents : les decks sans fichier restent ici.
    settings.paths.decks = Some(data.join("decks"));
    settings.i18n.locale = Some("fr".into());
    settings
}

/// Une session sur `settings`, sans deck ouvert.
pub(crate) fn session_with(settings: Settings) -> Session {
    let platform = Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme : {e}"));
    Session::new(Fonts::default(), &platform).unwrap_or_else(|e| panic!("session : {}", e.one_line()))
}

/// Une session isolée sur `data`, en français, sans deck ouvert.
pub(crate) fn session_on(data: &std::path::Path) -> Session {
    session_with(settings_on(data))
}

/// La bibliothèque de la session `s`.
pub(crate) fn library_of(s: &Session) -> &crate::PackLibrary {
    &s.library
}

/// Le fichier touché par un autre outil : même deck, autre empreinte.
///
/// La date `/ModDate` change, à longueur égale.
pub(crate) fn touch_elsewhere(path: &std::path::Path) {
    let mut bytes = std::fs::read(path).unwrap_or_else(|e| panic!("{} : {e}", path.display()));
    let at = bytes.windows(8).position(|w| w == b"/ModDate").unwrap_or_else(|| panic!("sans /ModDate"));
    let date = at + bytes[at..].windows(2).position(|w| w == b"D:").unwrap_or_else(|| panic!("date illisible")) + 2;
    bytes[date..date + 4].copy_from_slice(b"2099");
    std::fs::write(path, bytes).unwrap_or_else(|e| panic!("{} : {e}", path.display()));
}

/// Le deck de `s` épingle sous `deck` une copie du deck seul de son pack `base`
/// (nouvel id, mêmes widgets), embarquée ; son épinglage.
///
/// Comme le pack d'un import : la bibliothèque ne l'a pas.
pub(crate) fn adopt_deck_only_pack(s: &mut Session) -> oku_slides_core::PackPin {
    let t = s.widget_target(Some(&ns("base"))).unwrap_or_else(|e| panic!("pack du deck : {}", e.one_line()));
    let mut m = crate::pack::PackManifest::from_files(&t.files).unwrap_or_else(|e| panic!("manifeste : {e}"));
    m.id = uuid::Uuid::new_v4();
    m.name = "Importé".into();
    m.namespace = oku_slides_core::Namespace::deck();
    m.requires = vec![oku_slides_core::kinds::base::namespace()];
    m.parent = Some(t.version);
    m.scope = oku_slides_core::PackScope::Deck;
    let files = m.to_files(t.kind, t.files.widgets().clone()).unwrap_or_else(|e| panic!("manifeste : {e}"));
    let pin = oku_slides_core::PackPin { id: m.id, version: files.version(), requires: m.requires.clone() };
    s.doc.sources.add_pack(files);
    s.apply("Pack importé", vec![add_pack(&m, &pin)], None)
        .unwrap_or_else(|e| panic!("pack du deck seul : {}", e.one_line()));
    pin
}

/// Un fichier de bibliothèque : son chemin et ses octets.
pub(crate) type LibraryFile = (std::path::PathBuf, Vec<u8>);

/// Les fichiers sous `dir` avec leurs octets, triés : vide sans dossier.
fn files_under(dir: &std::path::Path) -> Vec<LibraryFile> {
    fn walk(dir: &std::path::Path, out: &mut Vec<LibraryFile>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for e in entries {
            let p = e.unwrap_or_else(|e| panic!("{e}")).path();
            if p.is_dir() {
                walk(&p, out);
            } else {
                let bytes = std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
                out.push((p, bytes));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, &mut out);
    out.sort();
    out
}

/// Les fichiers de la bibliothèque des packs sous `data` (`widgets/`), avec leurs octets, triés.
pub(crate) fn library_files(data: &std::path::Path) -> Vec<LibraryFile> {
    files_under(&data.join("widgets"))
}

/// Le nom court (`papier`, `pro`, `neutre`) d'un thème intégré, depuis son espace de noms.
pub(crate) fn theme_name(ns: &oku_slides_core::Namespace) -> Option<&'static str> {
    use crate::theme::{NEUTRE_NAMESPACE, PAPIER_NAMESPACE, PRO_NAMESPACE};
    [PAPIER_NAMESPACE, PRO_NAMESPACE, NEUTRE_NAMESPACE].into_iter().find(|n| *n == ns.as_str())
}

/// `deck` épingle le thème intégré `name` (un espace de noms) dans sa version livrée, sans parent.
pub(crate) fn pin_builtin_theme(deck: &mut oku_slides_core::Deck, name: &str) {
    use crate::library_kind::LibraryKind;
    let builtin = crate::theme::Themes::builtins().iter().find(|b| b.manifest.namespace.as_str() == name);
    let builtin = builtin.unwrap_or_else(|| panic!("thème intégré inconnu : {name}"));
    let pin =
        oku_slides_core::ThemePin { id: builtin.manifest.id, version: builtin.version.clone(), extends: Vec::new() };
    deck.theme = Some(ns(name));
    deck.themes = std::collections::BTreeMap::from([(ns(name), pin)]);
}

/// L'op qui passe le deck au thème `name` (un espace de noms), sa fermeture prise dans la bibliothèque.
pub(crate) fn set_theme_op(name: &str) -> oku_slides_core::Op {
    oku_slides_core::Op::SetTheme { theme: Some(ns(name)), themes: std::collections::BTreeMap::default() }
}

impl crate::ThemeLibrary {
    /// Une bibliothèque de thèmes sans dossier : les seuls intégrés.
    pub(crate) fn builtin_only() -> Self {
        Self::new(None, crate::Storage::local().unwrap_or_else(|e| panic!("stockage : {e}")))
    }
}

/// Ajoute à `themes` une version du thème `id` sous `namespace`, qui étend `extends`, rendue courante ; sa version.
pub(crate) fn add_user_theme(
    themes: &crate::ThemeLibrary,
    id: uuid::Uuid,
    namespace: &str,
    extends: &[&str],
    tokens: serde_json::Map<String, serde_json::Value>,
    parent: Option<&oku_slides_core::PackVersion>,
) -> oku_slides_core::PackVersion {
    let manifest = crate::ThemeManifest {
        id,
        name: namespace.to_owned(),
        namespace: ns(namespace),
        extends: extends.iter().map(|e| ns(e)).collect(),
        description: String::new(),
        parent: parent.cloned(),
        // Plus récente à chaque version : la courante par défaut reste la dernière ajoutée.
        created: format!("2026-10-{:02}T10:00:00Z", 1 + themes.versions(id).len()),
        tokens,
    };
    let files = manifest.to_files().unwrap_or_else(|e| panic!("theme.json : {e}"));
    themes.add_version(id, &files).unwrap_or_else(|e| panic!("thème {namespace} : {}", e.one_line()));
    themes.set_current(id, &files.version()).unwrap_or_else(|e| panic!("thème {namespace} : {}", e.one_line()));
    files.version()
}

/// Une bibliothèque de thèmes sur un dossier neuf, avec un thème par `(espace de noms, extends)`, dans l'ordre.
pub(crate) fn library_with_themes(themes: &[(&str, &[&str])]) -> (crate::ThemeLibrary, tempfile::TempDir) {
    let dir = fresh_dir("oku-slides-themes-with-");
    let storage = crate::Storage::local().unwrap_or_else(|e| panic!("stockage : {e}"));
    let library = crate::ThemeLibrary::new(Some(dir.path().to_owned()), storage);
    for (namespace, extends) in themes {
        add_user_theme(&library, uuid::Uuid::new_v4(), namespace, extends, serde_json::Map::default(), None);
    }
    (library, dir)
}

/// Une bibliothèque de thèmes avec le seul thème `namespace`, qui étend `extends`.
pub(crate) fn library_with_child(namespace: &str, extends: &[&str]) -> (crate::ThemeLibrary, tempfile::TempDir) {
    library_with_themes(&[(namespace, extends)])
}

/// Écrit sous le dossier de thèmes `themes` une version au `theme.json` brut `json`, sous son id.
pub(crate) fn write_raw_theme(themes: &std::path::Path, json: &str) {
    let files = oku_slides_core::PackFiles::theme_from_bytes(json.as_bytes()).unwrap_or_else(|| panic!("pas un objet"));
    let id = files.manifest()["id"].as_str().unwrap_or_else(|| panic!("sans id")).to_owned();
    let folder = themes.join(id).join("versions").join(files.version().hex());
    std::fs::create_dir_all(&folder).unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(folder.join("theme.json"), json).unwrap_or_else(|e| panic!("{e}"));
}

/// Un `theme.json` d'avant les espaces de noms, que le manifeste actuel ne lit plus.
pub(crate) const OLD_THEME_JSON: &str =
    r#"{"id":"0b0b0b0b-0000-4000-8000-000000000001","name":"Old","created":"2026-10-01T00:00:00Z","tokens":{}}"#;

/// Une bibliothèque de thèmes dont le dossier tient une version au `theme.json` brut `json`, sous son id.
pub(crate) fn library_with_raw_theme(json: &str) -> (crate::ThemeLibrary, tempfile::TempDir) {
    let dir = fresh_dir("oku-slides-themes-raw-");
    write_raw_theme(dir.path(), json);
    let storage = crate::Storage::local().unwrap_or_else(|e| panic!("stockage : {e}"));
    (crate::ThemeLibrary::new(Some(dir.path().to_owned()), storage), dir)
}

/// Les réglages d'un espace de travail de test : données et decks sous `data`, interface en français.
pub(crate) fn workspace_platform(data: &std::path::Path) -> crate::Platform {
    let mut settings = crate::Settings::defaults().unwrap_or_else(|e| panic!("réglages : {e}"));
    settings.paths.data = Some(data.to_owned());
    // Jamais dans le vrai dossier Documents : les decks sans fichier restent ici.
    settings.paths.decks = Some(data.join("decks"));
    settings.i18n.locale = Some("fr".into());
    crate::Platform::with_settings(settings).unwrap_or_else(|e| panic!("plateforme : {e}"))
}

/// Un espace de travail neuf sur `data`, sans reprise.
pub(crate) fn workspace_on(data: &std::path::Path) -> std::sync::Mutex<crate::Workspace> {
    let ws = crate::Workspace::new(crate::Fonts::default(), &workspace_platform(data), None);
    std::sync::Mutex::new(ws.unwrap_or_else(|e| panic!("espace de travail : {e}")))
}

/// L'espace de travail repris de `dir` (neuf s'il n'y a rien), sur `data`.
pub(crate) fn resumed_on(data: &std::path::Path, dir: &std::path::Path) -> std::sync::Mutex<crate::Workspace> {
    let ws = crate::Workspace::resume(crate::Fonts::default(), &workspace_platform(data), dir);
    std::sync::Mutex::new(ws.unwrap_or_else(|e| panic!("reprise : {e}")))
}

/// Applique une requête JSON de l'espace de travail.
pub(crate) fn apply_ws(
    ws: &std::sync::Mutex<crate::Workspace>,
    v: serde_json::Value,
) -> crate::Result<crate::WorkspaceOutcome> {
    crate::Workspace::apply(ws, serde_json::from_value(v).unwrap_or_else(|e| panic!("requête : {e}")))
}

/// Le `.pptx` de la fixture `fixtures/import/<name>` copié ou zippé dans `dir` ; son chemin.
///
/// Un fichier `<name>.pptx` est copié tel quel ; un dossier `<name>` est zippé.
pub(crate) fn pptx_fixture(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    use std::io::Write;
    // The files under `dir`, recursively, without `.DS_Store`.
    fn files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
            let p = e.unwrap_or_else(|e| panic!("{e}")).path();
            if p.file_name().is_some_and(|n| n == ".DS_Store") {
                continue;
            }
            if p.is_dir() { files(&p, out) } else { out.push(p) }
        }
    }
    let fixtures = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/import"));
    let file = fixtures.join(format!("{name}.pptx"));
    if file.is_file() {
        let path = dir.join(format!("{name}.pptx"));
        std::fs::copy(&file, &path).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        return path;
    }
    let root = fixtures.join(name);
    let mut paths = Vec::new();
    files(&root, &mut paths);
    // `[Content_Types].xml` en tête, comme l'écrit PowerPoint.
    paths.sort_by_key(|p| (p.file_name() != Some("[Content_Types].xml".as_ref()), p.clone()));
    let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for p in paths {
        let rel = p.strip_prefix(&root).unwrap_or_else(|e| panic!("{e}")).to_string_lossy().replace('\\', "/");
        w.start_file(rel, zip::write::SimpleFileOptions::default()).unwrap_or_else(|e| panic!("zip : {e}"));
        w.write_all(&std::fs::read(&p).unwrap_or_else(|e| panic!("{e}"))).unwrap_or_else(|e| panic!("zip : {e}"));
    }
    let bytes = w.finish().unwrap_or_else(|e| panic!("zip : {e}")).into_inner();
    let path = dir.join(format!("{name}.pptx"));
    std::fs::write(&path, bytes).unwrap_or_else(|e| panic!("{e}"));
    path
}

/// Un PNG blanc de `width` × `height` pixels.
pub(crate) fn white_png(width: u32, height: u32) -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().unwrap_or_else(|e| panic!("png : {e}"));
    writer.write_image_data(&vec![255; width as usize * height as usize * 4]).unwrap_or_else(|e| panic!("png : {e}"));
    writer.finish().unwrap_or_else(|e| panic!("png : {e}"));
    out
}

/// Un témoin de test aux pages blanches, de proportion 16:9 (celle des fixtures d'import) ; compte ses appels.
#[derive(Debug, Default)]
pub(crate) struct WhiteWitness {
    pub(crate) calls: std::sync::atomic::AtomicUsize,
}

impl WhiteWitness {
    /// Les tranches rendues jusqu'ici.
    pub(crate) fn calls(&self) -> usize {
        self.calls.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// Fait la référence de l'import de `s` par `witness`, comme l'appelant le fait hors du verrou.
pub(crate) fn witnessed(s: &mut Session, witness: &WhiteWitness) {
    let job = s.import_reference_job().unwrap_or_else(|e| panic!("travail de référence : {}", e.one_line()));
    if let Some(job) = job {
        job.run_with(witness);
    }
}

impl crate::import::WitnessRenderer for WhiteWitness {
    fn name(&self) -> &'static str {
        "témoin"
    }

    fn limits(&self) -> oku_slides_import::metrics::Witness {
        oku_slides_import::metrics::Witness::Office2Pdf
    }

    fn render(
        &self,
        _: &[u8],
        _: &Fonts,
        span: oku_slides_reference::SlideSpan,
        width: u32,
    ) -> Result<crate::import::WitnessPages, oku_slides_render::RenderError> {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let count = (span.last - span.first + 1) as usize;
        let height = oku_slides_core::num::round_u32(f64::from(width) * 9.0 / 16.0);
        let pages = (0..count).map(|_| white_png(width, height)).collect();
        Ok(crate::import::WitnessPages { witness: "témoin 1.0".to_owned(), count, pages })
    }
}

/// La cible hi-fi des tests, témoin office2pdf.
pub(crate) fn hifi() -> crate::import::ImportTarget {
    crate::import::ImportTarget::Hifi { witness: Some(oku_slides_platform::config::ImportWitness::Office2pdf) }
}

/// Une session sur `data`, chargée de l'import de la fixture `name` vers `target` ; réglages retouchés par `tweak`.
pub(crate) fn imported_fixture(
    data: &std::path::Path,
    name: &str,
    target: crate::import::ImportTarget,
    tweak: impl FnOnce(&mut Settings),
) -> Session {
    let mut settings = settings_on(data);
    tweak(&mut settings);
    let platform = Platform::with_settings(settings.clone()).unwrap_or_else(|e| panic!("plateforme : {e}"));
    let request = crate::import::ImportRequest {
        pptx: pptx_fixture(data, name),
        target,
        system_fonts: false,
        packs: crate::import::PackLookup::Library,
    };
    let imported = crate::import::import(&platform, &Fonts::default(), &request)
        .unwrap_or_else(|e| panic!("import : {}", e.one_line()));
    let mut s = session_with(settings);
    s.load_import(imported, name).unwrap_or_else(|e| panic!("chargement : {}", e.one_line()));
    s
}

/// Le journal écrit pendant `f`, sans couleurs.
pub(crate) fn logged(f: impl FnOnce()) -> String {
    let buffer = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let writer = buffer.clone();
    let subscriber = tracing_subscriber::fmt().with_ansi(false).with_writer(move || Log(writer.clone())).finish();
    tracing::subscriber::with_default(subscriber, f);
    let bytes = buffer.lock().unwrap().clone();
    String::from_utf8(bytes).unwrap()
}

/// Un écrivain du journal dans un tampon partagé.
struct Log(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for Log {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// L'op qui renomme le deck (en mode pack : le pack édité) en `t`.
pub(crate) fn rename_title(t: &str) -> oku_slides_core::Op {
    serde_json::from_value(serde_json::json!({ "op": "set_deck_param", "key": "title", "value": t }))
        .unwrap_or_else(|e| panic!("op : {e}"))
}

/// L'édition en cours (mode pack) de `s`.
pub(crate) fn edit_of(s: &Session) -> crate::PackMode {
    s.snapshot().editing.map_or_else(|| panic!("mode pack attendu"), |e| e.mode.clone())
}

/// Le deck mis de côté par le mode pack de `s`.
pub(crate) fn stash_of(s: &Session) -> &crate::Document {
    let crate::Mode::Pack { stash, .. } = &s.mode else { panic!("mode pack attendu") };
    stash
}

/// La source d'un layout `name` des tests : un widget de slide au seul paramètre `title`.
pub(crate) fn slide_widget(name: &str) -> String {
    format!(
        "#import \"/oku-slides/lib.typ\": widget, field, params as p\n\
         #let widget = widget(\"{name}\", category: \"slide\", params: (title: p.plain()), \
         (args, ctx) => field(ctx, \"title\", args.title))\n"
    )
}

/// Une session sur `data` dont la bibliothèque a le pack de layouts `namespace` (besoin `base`),
/// aux layouts `layouts` ; en mode pack sur lui. Son id.
pub(crate) fn editing_layout_pack(data: &std::path::Path, namespace: &str, layouts: &[&str]) -> (Session, uuid::Uuid) {
    let mut s = session_on(data);
    let m = manifest_ns(uuid::Uuid::new_v4(), namespace, &["base"]);
    let sources: Vec<(String, String)> = layouts.iter().map(|n| ((*n).to_owned(), slide_widget(n))).collect();
    let files: Vec<(&str, &str)> = sources.iter().map(|(n, src)| (n.as_str(), src.as_str())).collect();
    add_version_of(&s.library, oku_slides_core::PackKind::Layouts, &m, &files);
    s.edit_pack(m.id).unwrap_or_else(|e| panic!("mode pack : {}", e.one_line()));
    (s, m.id)
}

/// Pose `slide` en tête du deck de travail de `s` (mode pack), sans étape ni garde des slides, et rend.
///
/// Une slide hôte pour montrer un widget d'élément sur une slide : le mode pack refuse `add_slide`.
pub(crate) fn host_slide(s: &mut Session, slide: serde_json::Value) {
    let slide: oku_slides_core::Slide = serde_json::from_value(slide).unwrap_or_else(|e| panic!("slide : {e}"));
    let mut deck = s.doc.sources.deck().clone();
    deck.slides.insert(0, slide);
    s.doc.sources.apply_silent(deck).unwrap_or_else(|e| panic!("slide hôte : {e}"));
    s.rerender().unwrap_or_else(|e| panic!("rendu : {}", e.one_line()));
}

/// Un PNG valide de `w` × `h` pixels gris, de contenu propre à `seed`.
pub(crate) fn png(w: u32, h: u32, seed: u8) -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    let mut e = png::Encoder::new(&mut out, w, h);
    e.set_color(png::ColorType::Grayscale);
    let mut writer = e.write_header().unwrap_or_else(|e| panic!("png : {e}"));
    let pixels: Vec<u8> = (0..w * h).map(|i| i.to_le_bytes()[0].wrapping_add(seed)).collect();
    writer.write_image_data(&pixels).unwrap_or_else(|e| panic!("png : {e}"));
    drop(writer);
    out.into_inner()
}
