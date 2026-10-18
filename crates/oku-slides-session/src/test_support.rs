//! Fabrique de session pour les tests, ouverte sur `fixtures/roadmap.json`.

use std::sync::Arc;

use oku_slides_core::WidgetRegistry;

use crate::{Fonts, Platform, Session, Settings, builtin_versions};

/// Le registre de base figé (`fixtures/widgets/base-registry.json`).
pub(crate) fn registry() -> Arc<WidgetRegistry> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/widgets/base-registry.json");
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

/// Un manifeste sans modèles : pack `id`, nom `name`, parente `parent`, créé à `created`.
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
        templates: Vec::new(),
        examples: Default::default(),
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

/// L'épinglage du deck de `s` sous l'espace de noms `namespace`.
pub(crate) fn pin(s: &Session, namespace: &str) -> Option<oku_slides_core::PackPin> {
    s.deck().widgets.get(&ns(namespace)).cloned()
}

/// Un manifeste sans modèles du pack `id` sous l'espace de noms `namespace`, avec ces besoins.
pub(crate) fn manifest_ns(id: uuid::Uuid, namespace: &str, requires: &[&str]) -> crate::pack::PackManifest {
    crate::pack::PackManifest {
        namespace: ns(namespace),
        requires: requires.iter().map(|r| ns(r)).collect(),
        ..manifest(id, namespace, None, "2026-10-01T10:00:00Z")
    }
}

/// Les fichiers de `manifest`, sans widgets.
pub(crate) fn files_of(manifest: &crate::pack::PackManifest) -> oku_slides_core::PackFiles {
    manifest.to_files(Default::default()).unwrap_or_else(|e| panic!("manifeste : {e}"))
}

/// Ajoute `manifest` (et ses fichiers de widgets) à `library` ; son épinglage.
pub(crate) fn add_version(
    library: &crate::library::PackLibrary,
    manifest: &crate::pack::PackManifest,
    widgets: &[(&str, &str)],
) -> oku_slides_core::PackPin {
    let files = manifest.to_files(widget_files(widgets)).unwrap_or_else(|e| panic!("manifeste : {e}"));
    library.add_version(manifest.id, &files).unwrap_or_else(|e| panic!("version : {}", e.one_line()));
    oku_slides_core::PackPin { id: manifest.id, version: files.version(), requires: manifest.requires.clone() }
}

/// L'op qui épingle `pin`, version de `manifest`, sous son espace de noms.
pub(crate) fn add_pack(manifest: &crate::pack::PackManifest, pin: &oku_slides_core::PackPin) -> oku_slides_core::Op {
    oku_slides_core::Op::AddPack { namespace: manifest.namespace.clone(), pin: pin.clone() }
}

/// Une bibliothèque sur un dossier neuf, gardé tant qu'elle vit.
pub(crate) struct TestLibrary {
    _dir: tempfile::TempDir,
    library: crate::library::PackLibrary,
}

impl std::ops::Deref for TestLibrary {
    type Target = crate::library::PackLibrary;

    fn deref(&self) -> &Self::Target {
        &self.library
    }
}

/// Une bibliothèque qui a, pour chaque `(espace de noms, besoins)`, un pack sans widgets.
pub(crate) fn library_with(packs: &[(&str, &[&str])]) -> TestLibrary {
    let dir = fresh_dir("oku-slides-library-with-");
    let storage = crate::Storage::local().unwrap_or_else(|e| panic!("stockage : {e}"));
    let library = crate::library::PackLibrary::new(Some(dir.path().to_owned()), storage);
    for (namespace, requires) in packs {
        add_version(&library, &manifest_ns(uuid::Uuid::new_v4(), namespace, requires), &[]);
    }
    TestLibrary { _dir: dir, library }
}

/// L'épinglage de la version courante du pack `namespace` de `library`.
pub(crate) fn current_pin(library: &crate::library::PackLibrary, namespace: &str) -> oku_slides_core::PackPin {
    crate::deck_closure::current_pin(library, &ns(namespace)).unwrap_or_else(|e| panic!("{}", e.one_line()))
}

/// L'op qui épingle la version courante du pack `namespace` de `library`.
pub(crate) fn add_current(library: &crate::library::PackLibrary, namespace: &str) -> oku_slides_core::Op {
    oku_slides_core::Op::AddPack { namespace: ns(namespace), pin: current_pin(library, namespace) }
}

/// Un deck vide qui épingle la version courante des packs `namespaces` de `library`.
pub(crate) fn deck_pinning(library: &crate::library::PackLibrary, namespaces: &[&str]) -> oku_slides_core::Deck {
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
pub(crate) fn library_of(s: &Session) -> &crate::library::PackLibrary {
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

/// Le deck de `s` épingle sous `deck` une copie du deck seul de son pack `essentiel`
/// (nouvel id), embarquée ; son épinglage.
///
/// Comme le pack d'un import : la bibliothèque ne l'a pas.
pub(crate) fn adopt_deck_only_pack(s: &mut Session) -> oku_slides_core::PackPin {
    let t = s.widget_target(Some(&ns("essentiel"))).unwrap_or_else(|e| panic!("pack du deck : {}", e.one_line()));
    let mut m = crate::pack::PackManifest::from_files(&t.files).unwrap_or_else(|e| panic!("manifeste : {e}"));
    m.id = uuid::Uuid::new_v4();
    m.name = "Importé".into();
    m.namespace = oku_slides_core::Namespace::deck();
    m.parent = Some(t.version);
    m.scope = oku_slides_core::PackScope::Deck;
    let files = m.to_files(t.files.widgets().clone()).unwrap_or_else(|e| panic!("manifeste : {e}"));
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

/// Les fichiers de la bibliothèque de thèmes sous `data` (`themes/`), avec leurs octets, triés.
pub(crate) fn theme_library_files(data: &std::path::Path) -> Vec<LibraryFile> {
    files_under(&data.join("themes"))
}

/// Le deck de `s` passe à un thème du deck seul (jetons de Pro, nouvel id), embarqué ; sa référence.
pub(crate) fn adopt_deck_only_theme(s: &mut Session) -> oku_slides_core::ThemeRef {
    use crate::library_kind::LibraryKind;
    let pro = crate::theme::Themes::builtins().iter().find(|b| b.manifest.id == crate::PRO_THEME);
    let pro = pro.unwrap_or_else(|| panic!("thème Pro absent"));
    let manifest = crate::ThemeManifest {
        id: uuid::Uuid::new_v4(),
        name: "Importé".into(),
        scope: oku_slides_core::PackScope::Deck,
        ..pro.manifest.clone()
    };
    let files = manifest.to_files().unwrap_or_else(|e| panic!("theme.json : {e}"));
    let r = oku_slides_core::ThemeRef { id: manifest.id, version: files.version() };
    s.doc.sources.add_theme(files);
    let op = oku_slides_core::Op::SetTheme { theme: Some(r.clone()) };
    s.apply("Thème importé", vec![op], None).unwrap_or_else(|e| panic!("thème du deck seul : {}", e.one_line()));
    r
}

/// La référence de la version intégrée du thème nommé `name` (sans casse).
pub(crate) fn theme_ref(name: &str) -> Option<oku_slides_core::ThemeRef> {
    use crate::library_kind::LibraryKind;
    crate::theme::Themes::builtins()
        .iter()
        .find(|b| b.manifest.name.eq_ignore_ascii_case(name))
        .map(|b| oku_slides_core::ThemeRef { id: b.manifest.id, version: b.version.clone() })
}

/// Le nom court (`papier`, `pro`) du thème intégré de même id que `r`.
pub(crate) fn theme_name(r: &oku_slides_core::ThemeRef) -> Option<&'static str> {
    [(crate::PAPIER_THEME, "papier"), (crate::PRO_THEME, "pro")].into_iter().find(|(id, _)| *id == r.id).map(|(_, n)| n)
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
pub(crate) fn resumed_on(data: &std::path::Path, dir: std::path::PathBuf) -> std::sync::Mutex<crate::Workspace> {
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
    let fixtures = std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/import"));
    let file = fixtures.join(format!("{name}.pptx"));
    if file.is_file() {
        let path = dir.join(format!("{name}.pptx"));
        std::fs::copy(&file, &path).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        return path;
    }
    fn files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display())) {
            let p = e.unwrap_or_else(|e| panic!("{e}")).path();
            if p.file_name().is_some_and(|n| n == ".DS_Store") {
                continue;
            }
            if p.is_dir() { files(&p, out) } else { out.push(p) }
        }
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
    fn name(&self) -> &str {
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
        let height = (f64::from(width) * 9.0 / 16.0).round() as u32;
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
