//! Le témoin LibreOffice : `soffice --headless --convert-to pdf`, un
//! programme externe lancé sans shell, borné par son délai.
//!
//! Chaque rendu a son dossier temporaire : un profil isolé
//! (`-env:UserInstallation`, jamais celui de l'utilisateur) dont
//! `user/fonts` reçoit les polices fournies, la présentation et le PDF. Le
//! filtre PDF rend les slides masquées (`ExportHiddenSlides`) et la tranche
//! demandée (`PageRange`, LibreOffice 7.4 ou plus). Le témoin tourne hors de
//! la mémoire bornée du processus de rendu : seul son délai le borne ; passé
//! lui, il est tué avec son groupe de processus.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use oku_slides_platform::storage::Storage;

use crate::error::{RenderSnafu, TimeoutSnafu};
use crate::process::{self, Finished, RunError};
use crate::{ReferenceError, ReferencePdf, ReferenceRenderer, SlideSpan};

/// Nom du témoin LibreOffice, avant que sa version soit connue.
pub const SOFFICE_WITNESS: &str = "LibreOffice";

/// Taille maximale du PDF rendu.
const MAX_PDF_BYTES: u64 = 512 * 1024 * 1024;

/// La présentation dans le dossier du rendu ; LibreOffice nomme le PDF d'après elle.
const DECK: &str = "deck.pptx";
const PDF: &str = "deck.pdf";

/// Octets de stderr cités par une erreur.
const STDERR_TAIL: usize = 600;

/// Le témoin LibreOffice (`soffice`).
#[derive(Debug)]
pub struct Soffice {
    storage: Storage,
    program: PathBuf,
    timeout: Duration,
    /// `LibreOffice 26.8.1.1`, lu au premier rendu (`--version`).
    version: OnceLock<String>,
}

impl Soffice {
    /// Le témoin `program`, chaque rendu borné par `timeout` (version comprise).
    pub fn new(storage: Storage, program: PathBuf, timeout: Duration) -> Self {
        Self { storage, program, timeout, version: OnceLock::new() }
    }

    /// Lance `soffice` avec `args` avant `deadline` ; un échec dit `what`.
    fn run(&self, args: &[OsString], deadline: Instant) -> Result<Finished, ReferenceError> {
        process::run(&self.program, args, deadline).map_err(|e| match e {
            RunError::Timeout => TimeoutSnafu { ms: millis(self.timeout) }.build().into(),
            RunError::Spawn(e) => render_err(format!("{} : {e}", self.program.display())),
            RunError::Wait(e) => render_err(format!("{} : {e}", self.program.display())),
        })
    }

    /// La version du témoin, lue une fois.
    fn version(&self, profile: &OsString, deadline: Instant) -> Result<(), ReferenceError> {
        if self.version.get().is_some() {
            return Ok(());
        }
        let out = self.run(&[profile.clone(), "--version".into()], deadline)?;
        let version =
            parse_version(&out.stdout).ok_or_else(|| render_err(format!("--version : {}", tail(&out.stdout))))?;
        // Un autre fil a pu la poser : c'est la même.
        let _ = self.version.set(version);
        Ok(())
    }

    /// Écrit `bytes` à `path`, créant ses dossiers.
    fn write(&self, path: &Path, bytes: Vec<u8>) -> Result<(), ReferenceError> {
        self.storage.write(path, bytes).map_err(|e| render_err(format!("{} : {e}", path.display())))
    }
}

impl ReferenceRenderer for Soffice {
    fn witness(&self) -> &str {
        self.version.get().map_or(SOFFICE_WITNESS, String::as_str)
    }

    fn render(
        &self,
        pptx: &[u8],
        fonts: Vec<Vec<u8>>,
        slides: Option<SlideSpan>,
    ) -> Result<ReferencePdf, ReferenceError> {
        let deadline = Instant::now() + self.timeout;
        let dir = tempfile::Builder::new()
            .prefix("oku-slides-soffice-")
            .tempdir()
            .map_err(|e| render_err(format!("dossier temporaire : {e}")))?;
        let profile = dir.path().join("profile");
        let font_dir = profile.join("user").join("fonts");
        for (i, font) in fonts.into_iter().enumerate() {
            let name = format!("oku-{i}.{}", font_extension(&font));
            self.write(&font_dir.join(name), font)?;
        }
        let deck = dir.path().join(DECK);
        self.write(&deck, pptx.to_vec())?;
        let profile = profile_arg(&profile)?;
        self.version(&profile, deadline)?;
        let out_dir = dir.path().join("out");
        let finished = self.run(&convert_args(profile, slides, &out_dir, &deck), deadline)?;
        if !finished.status.success() {
            return Err(render_err(format!("{} : {}", finished.status, tail(&finished.stderr))));
        }
        let pdf = self
            .storage
            .read_limited(&out_dir.join(PDF), MAX_PDF_BYTES)
            .map_err(|e| render_err(format!("pas de PDF ({e}) : {}", tail(&finished.stderr))))?;
        ReferencePdf::new(pdf)
    }
}

/// Le premier programme de `candidates` qui existe : un chemin tel quel, un
/// nom seul cherché dans `path` (la variable `PATH`), dans l'ordre.
pub fn find_soffice(storage: &Storage, candidates: &[PathBuf], path: Option<&std::ffi::OsStr>) -> Option<PathBuf> {
    let dirs: Vec<PathBuf> = path.map(|p| std::env::split_paths(p).collect()).unwrap_or_default();
    let exists = |p: &Path| storage.exists(p).unwrap_or(false);
    candidates.iter().find_map(|c| {
        if c.components().count() > 1 || c.is_absolute() {
            exists(c).then(|| c.clone())
        } else {
            dirs.iter().map(|d| d.join(c)).find(|p| exists(p))
        }
    })
}

/// Les arguments de la conversion de `deck` en PDF dans `out_dir`.
fn convert_args(profile: OsString, slides: Option<SlideSpan>, out_dir: &Path, deck: &Path) -> Vec<OsString> {
    let range = slides
        .map_or(String::new(), |s| format!(r#","PageRange":{{"type":"string","value":"{}-{}"}}"#, s.first, s.last));
    let filter =
        format!(r#"pdf:impress_pdf_Export:{{"ExportHiddenSlides":{{"type":"boolean","value":"true"}}{range}}}"#);
    let mut args: Vec<OsString> =
        ["--headless", "--invisible", "--nologo", "--norestore", "--nolockcheck", "--nodefault"]
            .into_iter()
            .map(OsString::from)
            .collect();
    args.push(profile);
    args.extend([OsString::from("--convert-to"), filter.into(), "--outdir".into()]);
    args.extend([out_dir.as_os_str().to_owned(), deck.as_os_str().to_owned()]);
    args
}

/// `-env:UserInstallation=file:///…` : le profil isolé, en URL.
fn profile_arg(profile: &Path) -> Result<OsString, ReferenceError> {
    let path = profile.to_str().ok_or_else(|| render_err(format!("chemin non UTF-8 : {}", profile.display())))?;
    let path = path.replace('\\', "/");
    let slash = if path.starts_with('/') { "" } else { "/" };
    Ok(format!("-env:UserInstallation=file://{slash}{}", percent_encode(&path)).into())
}

/// `s` en chemin d'URL : tout octet hors des non réservés et de `/` en `%XX`.
fn percent_encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' | b':' => {
                char::from(b).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// L'extension d'une police, d'après sa signature.
fn font_extension(font: &[u8]) -> &'static str {
    match font.get(..4) {
        Some(b"OTTO") => "otf",
        Some(b"ttcf") => "ttc",
        _ => "ttf",
    }
}

/// `LibreOffice 26.8.1.1` de la sortie de `--version`.
fn parse_version(stdout: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(stdout);
    let line = text.lines().find(|l| l.starts_with(SOFFICE_WITNESS))?;
    let mut words = line.split_whitespace();
    Some(format!("{} {}", words.next()?, words.next()?))
}

/// La fin de `bytes`, lisible, pour un message.
fn tail(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim();
    let start = text.char_indices().rev().nth(STDERR_TAIL).map_or(0, |(i, _)| i);
    text[start..].to_owned()
}

fn millis(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

fn render_err(message: String) -> ReferenceError {
    RenderSnafu { message }.build().into()
}

#[cfg(test)]
#[path = "soffice_tests.rs"]
mod soffice_tests;
