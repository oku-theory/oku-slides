use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use oku_slides_platform::storage::Storage;
use rstest::rstest;
use speculoos::prelude::*;

use super::*;
use crate::test_support::{blank_pdf, converting, fake_soffice};
use crate::{ReferenceErrorKind, SlideSpan};

struct Fake {
    dir: tempfile::TempDir,
    soffice: PathBuf,
}

impl Fake {
    /// Un faux `soffice` qui rend un PDF blanc de `pages` pages.
    fn converting(pages: usize) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let pdf = dir.path().join("fixture.pdf");
        std::fs::write(&pdf, blank_pdf(pages, 960, 540)).unwrap();
        let soffice = fake_soffice(dir.path(), &converting(&pdf, Some(dir.path())));
        Self { dir, soffice }
    }

    /// Un faux `soffice` de corps `body`.
    fn with(body: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let soffice = fake_soffice(dir.path(), body);
        Self { dir, soffice }
    }

    fn witness(&self, timeout: Duration) -> Soffice {
        Soffice::new(Storage::local().unwrap(), self.soffice.clone(), timeout)
    }

    fn log(&self, name: &str) -> String {
        std::fs::read_to_string(self.dir.path().join(name)).unwrap()
    }
}

/// Délai des faux `soffice` qui finissent : large, macOS pouvant tarder à lancer
/// un exécutable qu'on vient d'écrire quand la machine est chargée.
const SLOW: Duration = Duration::from_secs(120);

const PPTX: &[u8] = b"PK pas lu par le faux soffice";

#[test]
fn render_with_soffice_reads_the_pdf_it_writes() {
    let fake = Fake::converting(3);
    let pdf = fake.witness(SLOW).render(PPTX, vec![], None).unwrap();
    assert_that!(pdf.page_count()).is_equal_to(3);
}

#[test]
fn witness_of_soffice_is_its_name_then_its_version() {
    let fake = Fake::converting(1);
    let soffice = fake.witness(SLOW);
    let before = soffice.witness().to_owned();
    soffice.render(PPTX, vec![], None).unwrap();
    assert_that!((before.as_str(), soffice.witness())).is_equal_to((SOFFICE_WITNESS, "LibreOffice 99.1.0.1"));
}

#[test]
fn render_with_soffice_asks_hidden_slides_and_the_span() {
    let fake = Fake::converting(2);
    fake.witness(SLOW).render(PPTX, vec![], Some(SlideSpan { first: 2, last: 3 })).unwrap();
    let args = fake.log("args");
    assert_that!(args.as_str()).contains(r#""ExportHiddenSlides":{"type":"boolean","value":"true"}"#);
    assert_that!(args.as_str()).contains(r#""PageRange":{"type":"string","value":"2-3"}"#);
}

#[test]
fn render_with_soffice_runs_headless_on_an_isolated_profile() {
    let fake = Fake::converting(1);
    fake.witness(SLOW).render(PPTX, vec![], None).unwrap();
    let args = fake.log("args");
    let lines: Vec<&str> = args.lines().collect();
    assert_that!(lines.contains(&"--headless")).is_true();
    let profile = lines.iter().find_map(|l| l.strip_prefix("-env:UserInstallation=file://")).unwrap();
    assert_that!(profile).contains("oku-slides-soffice-");
}

#[test]
fn render_with_soffice_gives_the_fonts_to_the_profile() {
    let fake = Fake::converting(1);
    let fonts = vec![b"OTTO police".to_vec(), b"\0\x01\0\0 police".to_vec()];
    fake.witness(SLOW).render(PPTX, fonts, None).unwrap();
    assert_that!(fake.log("fonts")).is_equal_to("oku-0.otf\noku-1.ttf\n".to_owned());
}

#[test]
fn render_with_soffice_removes_its_temporary_dir() {
    let fake = Fake::converting(1);
    fake.witness(SLOW).render(PPTX, vec![], None).unwrap();
    let args = fake.log("args");
    let profile = args.lines().find_map(|l| l.strip_prefix("-env:UserInstallation=file://")).unwrap();
    let dir = Path::new(profile).parent().unwrap();
    assert_that!(dir.exists()).is_false();
}

#[test]
fn render_with_a_failing_soffice_is_a_render_error_citing_its_stderr() {
    let fake = Fake::with("echo 'format inconnu' >&2; exit 1");
    let err = fake.witness(SLOW).render(PPTX, vec![], None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ReferenceErrorKind::Render);
    assert_that!(err.to_string()).contains("format inconnu");
}

#[test]
fn render_with_a_soffice_that_writes_no_pdf_is_a_render_error() {
    let fake = Fake::with("exit 0");
    let err = fake.witness(SLOW).render(PPTX, vec![], None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ReferenceErrorKind::Render);
}

#[test]
fn render_with_a_hanging_soffice_is_a_timeout_after_its_delay() {
    let fake = Fake::with("sleep 30");
    let start = Instant::now();
    let err = fake.witness(Duration::from_millis(300)).render(PPTX, vec![], None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ReferenceErrorKind::Timeout);
    assert_that!(start.elapsed()).is_less_than(Duration::from_secs(5));
}

#[test]
fn render_with_a_missing_soffice_is_a_render_error() {
    let soffice = Soffice::new(Storage::local().unwrap(), "/nonexistent/soffice".into(), Duration::from_secs(1));
    let err = soffice.render(PPTX, vec![], None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ReferenceErrorKind::Render);
}

/// Un faux binaire (pas un script) : `find_soffice` ne l'exécute pas.
fn fake_binary(dir: &Path) -> PathBuf {
    let path = dir.join("soffice");
    std::fs::write(&path, [0xCF, 0xFA, 0xED, 0xFE]).unwrap();
    path
}

#[test]
fn find_soffice_takes_the_first_candidate_that_exists() {
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_binary(dir.path());
    let candidates = vec![PathBuf::from("/nonexistent/soffice"), bin.clone()];
    let found = find_soffice(&Storage::local().unwrap(), &candidates, None).unwrap();
    assert_that!(found).is_equal_to(Some(std::fs::canonicalize(&bin).unwrap()));
}

#[test]
fn find_soffice_looks_a_bare_name_up_in_the_path() {
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_binary(dir.path());
    let path = std::env::join_paths(["/nonexistent", dir.path().to_str().unwrap()]).unwrap();
    let found = find_soffice(&Storage::local().unwrap(), &[PathBuf::from("soffice")], Some(&path)).unwrap();
    assert_that!(found).is_equal_to(Some(std::fs::canonicalize(&bin).unwrap()));
}

#[cfg(unix)]
#[test]
fn find_soffice_follows_a_symlink_to_the_real_binary() {
    let dir = tempfile::tempdir().unwrap();
    let bin = fake_binary(dir.path());
    let link = dir.path().join("lien");
    std::os::unix::fs::symlink(&bin, &link).unwrap();
    let found = find_soffice(&Storage::local().unwrap(), &[link], None).unwrap();
    assert_that!(found).is_equal_to(Some(std::fs::canonicalize(&bin).unwrap()));
}

#[test]
fn find_soffice_refuses_a_script_and_names_it() {
    let fake = Fake::with("exit 0");
    let err = find_soffice(&Storage::local().unwrap(), std::slice::from_ref(&fake.soffice), None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ReferenceErrorKind::Render);
    assert_that!(err.to_string()).contains("soffice");
}

#[test]
fn find_soffice_without_any_candidate_on_disk_is_none() {
    let found =
        find_soffice(&Storage::local().unwrap(), &[PathBuf::from("soffice-absent")], Some("/nonexistent".as_ref()));
    assert_that!(found.unwrap()).is_none();
}

#[test]
fn convert_args_without_span_render_every_slide() {
    let args = convert_args("-env:x".into(), None, Path::new("/o"), Path::new("/d.pptx"));
    assert_that!(args.iter().any(|a| a.to_string_lossy().contains("PageRange"))).is_false();
}

#[test]
fn profile_arg_is_a_file_url_with_its_special_bytes_escaped() {
    let arg = profile_arg(Path::new("/tmp/a b%é/profile")).unwrap();
    assert_that!(arg.to_string_lossy().into_owned())
        .is_equal_to("-env:UserInstallation=file:///tmp/a%20b%25%C3%A9/profile".to_owned());
}

#[rstest]
#[case(b"OTTO....".as_slice(), "otf")]
#[case(b"ttcf....".as_slice(), "ttc")]
#[case(b"\0\x01\0\0..".as_slice(), "ttf")]
#[case(b"".as_slice(), "ttf")]
fn font_extension_follows_the_signature(#[case] font: &[u8], #[case] ext: &str) {
    assert_that!(font_extension(font)).is_equal_to(ext);
}

#[rstest]
#[case("LibreOffice 26.8.1.1 bf6ef26\n", Some("LibreOffice 26.8.1.1"))]
#[case("Fontconfig warning\nLibreOffice 7.6.4.1 e19e193\n", Some("LibreOffice 7.6.4.1"))]
#[case("rien\n", None)]
fn parse_version_reads_name_and_version(#[case] stdout: &str, #[case] expected: Option<&str>) {
    assert_that!(parse_version(stdout.as_bytes())).is_equal_to(expected.map(str::to_owned));
}

/// Variable qui autorise les tests à lancer le vrai LibreOffice (il s'affiche dans le Dock).
const REAL_SOFFICE_VAR: &str = "OKU_SLIDES_TEST_SOFFICE";

/// Le vrai LibreOffice, si `OKU_SLIDES_TEST_SOFFICE=1` et s'il est installé ; sinon le test s'arrête là.
fn installed() -> Option<PathBuf> {
    if std::env::var(REAL_SOFFICE_VAR).as_deref() != Ok("1") {
        eprintln!("{REAL_SOFFICE_VAR}=1 absent : test du vrai LibreOffice sauté");
        return None;
    }
    let candidates = [PathBuf::from("/Applications/LibreOffice.app/Contents/MacOS/soffice"), PathBuf::from("soffice")];
    let found =
        find_soffice(&Storage::local().unwrap(), &candidates, std::env::var_os("PATH").as_deref()).ok().flatten();
    if found.is_none() {
        eprintln!("LibreOffice absent : test sauté");
    }
    found
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/import").join(name)).unwrap()
}

#[test]
fn render_with_installed_libreoffice_gives_one_page_per_slide_of_the_span() {
    let Some(program) = installed() else { return };
    let soffice = Soffice::new(Storage::local().unwrap(), program, Duration::from_secs(120));
    let span = SlideSpan { first: 2, last: 3 };
    let pages = |s| soffice.render(&fixture("libreoffice.pptx"), vec![], s).map(|p| p.page_count()).ok();
    assert_that!((pages(None), pages(Some(span)))).is_equal_to((Some(3), Some(2)));
    assert_that!(soffice.witness()).starts_with("LibreOffice ");
}

#[test]
fn render_with_installed_libreoffice_keeps_the_page_of_a_hidden_slide() {
    let Some(program) = installed() else { return };
    let soffice = Soffice::new(Storage::local().unwrap(), program, Duration::from_secs(120));
    let pptx = crate::office2pdf::office2pdf_tests::hide(&fixture("libreoffice.pptx"), "ppt/slides/slide2.xml");
    let span = SlideSpan { first: 1, last: 3 };
    let pages = |s| soffice.render(&pptx, vec![], s).map(|p| p.page_count()).ok();
    assert_that!((pages(None), pages(Some(span)))).is_equal_to((Some(3), Some(3)));
}
