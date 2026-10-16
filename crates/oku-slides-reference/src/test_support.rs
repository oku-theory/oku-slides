//! Outils des tests (feature `test-support`) : un PDF blanc fait main, un faux `soffice`.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// Un PDF de `pages` pages blanches de `w` × `h` pt.
pub fn blank_pdf(pages: usize, w: u32, h: u32) -> Vec<u8> {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        format!(
            "<< /Type /Pages /Kids [{}] /Count {pages} >>",
            (0..pages).map(|i| format!("{} 0 R", i + 3)).collect::<Vec<_>>().join(" ")
        ),
    ];
    objects.extend((0..pages).map(|_| format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w} {h}] >>")));
    let mut out = String::from("%PDF-1.4\n");
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{body}\nendobj\n", i + 1);
    }
    let xref = out.len();
    let _ = write!(out, "xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for o in offsets {
        let _ = writeln!(out, "{o:010} 00000 n ");
    }
    let _ = write!(out, "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objects.len() + 1);
    out.into_bytes()
}

/// Écrit dans `dir` un faux `soffice` exécutable dont le corps est `body`
/// (shell), après la réponse à `--version` (`LibreOffice 99.1.0.1`) ; rend son chemin.
#[cfg(unix)]
pub fn fake_soffice(dir: &Path, body: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("soffice");
    let script = format!(
        "#!/bin/sh\nfor a in \"$@\"; do [ \"$a\" = --version ] && echo 'LibreOffice 99.1.0.1 abcdef' && exit 0; done\n{body}\n"
    );
    std::fs::write(&path, script).unwrap_or_else(|e| panic!("faux soffice : {e}"));
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
        .unwrap_or_else(|e| panic!("faux soffice : {e}"));
    path
}

/// Corps d'un faux `soffice` qui convertit : il copie `pdf` où LibreOffice
/// écrirait le sien et, avec `log`, y note ses arguments (`args`) et les
/// polices du profil (`fonts`).
pub fn converting(pdf: &Path, log: Option<&Path>) -> String {
    let notes = log.map_or(String::new(), |log| {
        format!(
            "printf '%s\\n' \"$@\" > '{log}/args'\nls \"$profile/user/fonts\" > '{log}/fonts'\n",
            log = log.display()
        )
    });
    format!(
        r#"prev=; out=; last=; profile=
for a in "$@"; do
  [ "$prev" = --outdir ] && out="$a"
  case "$a" in -env:UserInstallation=file://*) profile="${{a#-env:UserInstallation=file://}}";; esac
  prev="$a"; last="$a"
done
{notes}mkdir -p "$out" && cp '{pdf}' "$out/$(basename "$last" .pptx).pdf""#,
        pdf = pdf.display()
    )
}
