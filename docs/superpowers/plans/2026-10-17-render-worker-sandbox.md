# Render worker sandbox (macOS) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** On macOS, the render worker and its LibreOffice child can open files for writing only
inside a per-launch temp dir, and the host proves it at every launch.

**Architecture:** The host spawns `/usr/bin/sandbox-exec -p <worker.sb> -D … <worker>` instead
of the worker itself. The worker's `TMPDIR` points to a fresh work dir that the `Link` owns.
`Hello` carries a canary path outside that dir: the worker must fail to create it with `EPERM`,
or the host drops it. `find_soffice` returns the real LibreOffice binary and refuses scripts,
and the CLI tells the worker which `soffice` it may exec.

**Tech Stack:** Rust (edition 2024), Seatbelt SBPL through `sandbox-exec`, `tempfile`,
OpenDAL (`Storage`), `libc`, nextest, speculoos, rstest.

**Spec:** `docs/superpowers/specs/2026-10-17-render-worker-sandbox-design.md` (approved
2026-10-17: the soffice `.app` comes first, scripts are refused, there is no switch).

## Global Constraints

- macOS only. On other platforms the spawn is unchanged; `RenderLaunch::InProcess` is never
  sandboxed.
- Fail closed. No setting, environment variable or fallback runs the worker unsandboxed on
  macOS.
- The profile text is exactly the one in Task 3 (`worker.sb`). Any later change re-runs the
  real-LibreOffice test (`OKU_SLIDES_TEST_SOFFICE=1`).
- Bump `PROTOCOL_VERSION` from 2 to 3: `Hello` changes.
- Root rules apply:
  - no `unwrap`/`expect` outside tests, and no silent fallback;
  - Snafu errors;
  - `lock().unwrap_or_else(PoisonError::into_inner)`;
  - no `std::fs` in production except inside `Storage`'s module;
  - files under 500 lines (`link.rs` is 447: new code goes in new modules);
  - every pub item documented, first sentence under 15 words;
  - tests in sibling `x_tests.rs`, named `unit_scenario_result`, asserted with speculoos.
- Code comments and commits are in French: `type(scope) : description (t-912)`. Docs are in
  English.
- Shared cargo target (user's choice): prefix each cargo run with `touch crates/*/src/lib.rs &&`
  in the same command. In an SDD worktree, `git config --worktree oku.review deferred` applies
  to the commit-review hook (see the user's memory note).
- Never run the app or a server without fresh `OKU_SLIDES_PATHS__DATA` and
  `OKU_SLIDES_PATHS__DECKS` temp dirs.

### Deviations from the first spec draft (now written into the spec)

1. Spec Design §2, "per-request subdir `WORK/r-<n>`", becomes **"the worker empties WORK after
   each `Reference` request"** (Task 4). office2pdf runs in-process and reads `TMPDIR` through
   `std::env::temp_dir()`; giving it a per-request dir would need `set_var` in a multithreaded
   process, which is `unsafe`. Isolating one deck from the next is the same either way.
2. Spec Design §6, "`fake_soffice` tests pass `/bin/sh` as `SOFFICE`", is dropped. The
   worker's unit tests use `RenderLaunch::InProcess` (unsandboxed), and the CLI's fake-script
   test becomes a "script refused" test. The sandboxed soffice path is covered by the
   real-LibreOffice CLI test (`OKU_SLIDES_TEST_SOFFICE=1`).
3. Spec Design §6, "a worker spawned with a profile that allows the canary path is refused",
   is covered by two unit tests instead of a real-worker integration test. The table test on
   `check_hello` covers every canary answer, and `sandbox_canary_is_outside_work_and_cannot_be_created`
   shows the real profile refuses the canary. The crate-private `Sandbox` cannot be bent from
   `tests/`.
4. Test-only: shell-script fake workers (`link_fake_tests.rs`) run with
   `Link::unsandboxed = true`, a `#[cfg(test)]` field. It exists in no build the user runs, so
   the "no switch" decision holds.

---

### Task 1: Storage primitives: real path, partial read, permission-denied kind

**Files:**
- Modify: `crates/oku-slides-platform/src/storage.rs` (the error enum ~l.28–60, a new free fn
  after `resolve` ~l.83, a new method after `read_limited` ~l.187)
- Test: `crates/oku-slides-platform/src/storage_tests.rs`

**Interfaces:**
- Produces:
  - `oku_slides_platform::storage::canonical(path: &Path) -> Result<PathBuf, StorageError>`;
  - `Storage::read_head(&self, path: &Path, n: u64) -> Result<Vec<u8>, StorageError>`;
  - `StorageErrorKind::PermissionDenied`.

- [ ] **Step 1: Write the failing tests** (append to `storage_tests.rs`)

```rust
#[cfg(unix)]
#[test]
fn canonical_follows_a_symlink_to_its_target() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("target");
    std::fs::write(&target, b"x").unwrap();
    let link = dir.path().join("link");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    assert_that!(canonical(&link).unwrap()).is_equal_to(std::fs::canonicalize(&target).unwrap());
}

#[test]
fn canonical_of_a_missing_path_is_not_found() {
    let err = canonical(Path::new("/nonexistent/oku-slides")).unwrap_err();
    assert_that!(err.kind()).is_equal_to(StorageErrorKind::NotFound);
}

#[test]
fn read_head_of_a_longer_file_reads_its_first_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("f");
    std::fs::write(&path, b"#!/bin/sh\n").unwrap();
    assert_that!(Storage::local().unwrap().read_head(&path, 2).unwrap()).is_equal_to(b"#!".to_vec());
}

#[test]
fn read_head_of_an_empty_file_is_empty() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("f");
    std::fs::write(&path, b"").unwrap();
    assert_that!(Storage::local().unwrap().read_head(&path, 2).unwrap()).is_empty();
}

#[cfg(unix)]
#[test]
fn write_into_a_read_only_dir_is_permission_denied() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o555)).unwrap();
    let err = Storage::local().unwrap().write(&dir.path().join("f"), b"x".to_vec()).unwrap_err();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_that!(err.kind()).is_equal_to(StorageErrorKind::PermissionDenied);
}
```

- [ ] **Step 2: Run them and check they fail**

Run: `touch crates/*/src/lib.rs && cargo nextest run -p oku-slides-platform storage`
Expected: compile errors: `canonical`, `read_head` and `PermissionDenied` do not exist.

- [ ] **Step 3: Implement**

In `StorageErrorKind`, add after `TooLarge`:

```rust
    /// Écriture ou lecture refusée par le système (droits, bac à sable).
    PermissionDenied,
```

In `StorageInner`, add:

```rust
    #[snafu(display("cannot resolve {}", path.display()))]
    Canonical { path: PathBuf, source: std::io::Error, backtrace: Backtrace },
```

In `StorageError::kind`, before the `_` arm:

```rust
            StorageInner::Access { source, .. } if source.kind() == ErrorKind::PermissionDenied => {
                StorageErrorKind::PermissionDenied
            }
            StorageInner::Canonical { source, .. } if source.kind() == std::io::ErrorKind::NotFound => {
                StorageErrorKind::NotFound
            }
```

After `resolve`:

```rust
/// Le chemin réel de `path` : absolu, liens suivis (`realpath`).
///
/// OpenDAL ne suit pas les liens ; le bac à sable du rendu compare des
/// chemins réels (t-912). Le disque n'est que lu.
pub fn canonical(path: &Path) -> Result<PathBuf, StorageError> {
    std::fs::canonicalize(path).context(CanonicalSnafu { path })
}
```

After `read_limited`:

```rust
    /// Les `n` premiers octets du fichier, moins s'il est plus court.
    pub fn read_head(&self, path: &Path, n: u64) -> Result<Vec<u8>, StorageError> {
        let (op, key) = self.at(path)?;
        let len = op.stat(&key).context(AccessSnafu { action: "read", path })?.content_length().min(n);
        if len == 0 {
            return Ok(Vec::new());
        }
        let opts = opendal::options::ReadOptions { range: (0..len).into(), ..Default::default() };
        let buffer = op.read_options(&key, opts).context(AccessSnafu { action: "read", path })?;
        Ok(buffer.to_vec())
    }
```

If the read-only-dir test shows that OpenDAL maps `EACCES`/`EPERM` to a kind other than
`opendal::ErrorKind::PermissionDenied`, stop and report it (G15). Do not widen the match by
guessing.

- [ ] **Step 4: Run the tests and check they pass**

Run: `touch crates/*/src/lib.rs && cargo nextest run -p oku-slides-platform`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add crates/oku-slides-platform/src/storage.rs crates/oku-slides-platform/src/storage_tests.rs
git commit -m "feat(platform) : Storage, chemin réel, lecture de tête et refus de droits (t-912)"
```

---

### Task 2: soffice resolved to its real binary, scripts refused

**Files:**
- Modify: `crates/oku-slides-reference/src/soffice.rs` (`find_soffice`, l.119–131)
- Modify: `crates/oku-slides-reference/src/soffice_tests.rs` (the `find_soffice_*` tests, l.135–155,
  and the real-LibreOffice helper, ~l.197)
- Modify: `crates/oku-slides-platform/src/config.default.toml:68-71`
- Modify: `crates/oku-slides-platform/src/config_tests.rs:317-323`
- Modify: `crates/oku-slides-cli/src/import_reference.rs` (`soffice`, l.42–48; a new `refused`
  next to `missing`, l.89)
- Modify: `crates/oku-slides-cli/src/import.rs:269-275`
- Modify: `crates/oku-slides-cli/tests/import_reference.rs:171-183`

**Interfaces:**
- Consumes: `storage::canonical`, `Storage::read_head` (Task 1).
- Produces:
  - `find_soffice(storage: &Storage, candidates: &[PathBuf], path: Option<&OsStr>) -> Result<Option<PathBuf>, ReferenceError>`.
    `Ok(Some(p))` means `p` is canonical and not a script.
  - `import_reference::soffice(platform) -> Result<Option<ReferenceWitness>, ReferenceError>`.

- [ ] **Step 1: Write the failing tests.** In `soffice_tests.rs`, replace the three
  `find_soffice_*` tests:

```rust
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
    let err = find_soffice(&Storage::local().unwrap(), &[fake.soffice.clone()], None).unwrap_err();
    assert_that!(err.kind()).is_equal_to(ReferenceErrorKind::Render);
    assert_that!(err.to_string()).contains("soffice");
}

#[test]
fn find_soffice_without_any_candidate_on_disk_is_none() {
    let found =
        find_soffice(&Storage::local().unwrap(), &[PathBuf::from("soffice-absent")], Some("/nonexistent".as_ref()));
    assert_that!(found.unwrap()).is_none();
}
```

In the real-LibreOffice helper `installed()` (~l.190–202), put
`/Applications/LibreOffice.app/Contents/MacOS/soffice` first in its candidates. Otherwise
Homebrew's script wrapper is found first, refused, and the test silently skips. Then change
`find_soffice(...)` to `find_soffice(...).ok().flatten()`.

In `config_tests.rs`, rename the default test and swap the order:

```rust
#[test]
fn settings_default_soffice_tries_the_macos_app_then_the_path() {
    let import = Settings::defaults().unwrap().import;
    assert_that!(import.soffice).is_equal_to(vec![
        PathBuf::from("/Applications/LibreOffice.app/Contents/MacOS/soffice"),
        PathBuf::from("soffice"),
    ]);
    assert_that!(import.soffice_timeout_ms).is_equal_to(60_000);
}
```

In `crates/oku-slides-cli/tests/import_reference.rs`, replace
`import_hifi_with_reference_soffice_scores_against_libreoffice` with:

```rust
#[cfg(unix)]
#[test]
fn import_hifi_with_a_script_soffice_still_writes_the_deck_and_says_it_failed() {
    use oku_slides_reference::test_support::{blank_pdf, converting, fake_soffice};
    let dir = workdir("import-reference-soffice-script");
    let pdf = dir.as_ref().join("blanc.pdf");
    std::fs::write(&pdf, blank_pdf(3, 960, 540)).unwrap_or_else(|e| panic!("{e}"));
    soffice_config(dir.as_ref(), &fake_soffice(dir.as_ref(), &converting(&pdf, None)));
    let (_, report) = import(dir.as_ref(), "libreoffice.pptx", &["--reference", "soffice"]);
    assert_that!(report["reference"]["unavailable"]["type"].as_str()).is_equal_to(Some("failed"));
    assert_that!(dir.as_ref().join("out.oku-slides.pdf").exists()).is_true();
}

/// LibreOffice réel, sous le bac à sable du processus de rendu (t-912) : `OKU_SLIDES_TEST_SOFFICE=1`.
#[test]
fn import_hifi_with_reference_soffice_scores_against_real_libreoffice() {
    if std::env::var_os("OKU_SLIDES_TEST_SOFFICE").is_none() {
        eprintln!("OKU_SLIDES_TEST_SOFFICE absent : test sauté");
        return;
    }
    let dir = workdir("import-reference-soffice-real");
    let (out, report) = import(dir.as_ref(), "libreoffice.pptx", &["--reference", "soffice"]);
    assert_that!(report["reference"]["unavailable"].is_null()).is_true();
    assert_that!(report["reference"]["witness"].as_str().unwrap_or_default()).starts_with("LibreOffice ");
    assert_that!(max_score(&report)).is_greater_than(0.0);
    let second = String::from_utf8_lossy(&out.stdout).lines().nth(1).unwrap_or_default().to_owned();
    assert_that!(second).starts_with("Visual gap to LibreOffice ");
}
```

(Check how `ReferenceCause::Failed` serializes before running: the `missing` test reads
`["unavailable"]["type"] == "missing"`, so `"failed"` is expected.)

- [ ] **Step 2: Run them and check they fail**

Run: `touch crates/*/src/lib.rs && cargo nextest run -p oku-slides-reference find_soffice && cargo nextest run -p oku-slides-platform settings_default_soffice`
Expected: compile errors (`unwrap` on `Option`), then the order assertion fails.

- [ ] **Step 3: Implement `find_soffice`** (replace the function in `soffice.rs`)

```rust
/// Le premier programme de `candidates` qui existe, en chemin réel : un
/// chemin tel quel, un nom seul cherché dans `path` (la variable `PATH`).
///
/// Un script (`#!`, le `soffice` de Homebrew) est refusé en le nommant : le
/// bac à sable du processus de rendu n'exécute que le binaire de LibreOffice (t-912).
pub fn find_soffice(
    storage: &Storage,
    candidates: &[PathBuf],
    path: Option<&std::ffi::OsStr>,
) -> Result<Option<PathBuf>, ReferenceError> {
    let dirs: Vec<PathBuf> = path.map(|p| std::env::split_paths(p).collect()).unwrap_or_default();
    let exists = |p: &Path| storage.exists(p).unwrap_or(false);
    let found = candidates.iter().find_map(|c| {
        if c.components().count() > 1 || c.is_absolute() {
            exists(c).then(|| c.clone())
        } else {
            dirs.iter().map(|d| d.join(c)).find(|p| exists(p))
        }
    });
    let Some(found) = found else { return Ok(None) };
    let real = canonical(&found).map_err(|e| render_err(format!("{} : {e}", found.display())))?;
    let head = storage.read_head(&real, 2).map_err(|e| render_err(format!("{} : {e}", real.display())))?;
    if head == b"#!" {
        return Err(render_err(format!(
            "{} est un script : donner le binaire de LibreOffice (…/LibreOffice.app/Contents/MacOS/soffice)",
            real.display()
        )));
    }
    Ok(Some(real))
}
```

Add `use oku_slides_platform::storage::canonical;`.

In `config.default.toml`, swap the list and update the comment:

```toml
# Témoin LibreOffice (`oku-slides import --reference soffice`) : les programmes
# `soffice` essayés dans l'ordre, un nom seul cherché dans PATH ; le binaire de
# LibreOffice, jamais un script (le bac à sable du rendu ne lance que lui). Aucun
# trouvé : l'import continue sans écart visuel (« missing »).
soffice = ["/Applications/LibreOffice.app/Contents/MacOS/soffice", "soffice"]
```

- [ ] **Step 4: Wire it into the CLI.** In `import_reference.rs`:

```rust
/// Le témoin LibreOffice, le premier programme de `[import] soffice` qui existe ; un script est refusé.
pub(crate) fn soffice(platform: &Platform) -> Result<Option<ReferenceWitness>, ReferenceError> {
    let import = &platform.settings.import;
    let path = std::env::var_os("PATH");
    let found = find_soffice(&platform.storage, &import.soffice, path.as_deref())?;
    Ok(found.map(|program| ReferenceWitness::Soffice { program, timeout_ms: import.soffice_timeout_ms }))
}

/// Pose dans `report` que le `soffice` trouvé est refusé (`e` le dit), et le journalise.
pub(crate) fn refused(report: &mut ImportReport, threshold: f64, e: &ReferenceError, l: &Localizer) {
    unavailable(report, SOFFICE_WITNESS, threshold, (ReferenceCause::Failed, e.to_string()), l);
}
```

(Import `oku_slides_reference::ReferenceError`.) In `import.rs`, replace the `Soffice` arm:

```rust
            ReferenceMode::Soffice => match import_reference::soffice(platform) {
                Ok(Some(witness)) => Some(witness),
                Ok(None) => {
                    import_reference::missing(plan.report_mut(), threshold, l);
                    None
                }
                Err(e) => {
                    import_reference::refused(plan.report_mut(), threshold, &e, l);
                    None
                }
            },
```

- [ ] **Step 5: Run the tests and check they pass**

Run: `touch crates/*/src/lib.rs && cargo nextest run -p oku-slides-reference -p oku-slides-platform -p oku-slides-cli`
Then: `touch crates/*/src/lib.rs && OKU_SLIDES_TEST_SOFFICE=1 cargo nextest run -p oku-slides-cli soffice`
Expected: all pass. The real-LibreOffice test passes here, before the sandbox exists.

- [ ] **Step 6: Commit**

```bash
git add crates/oku-slides-reference/src/soffice.rs crates/oku-slides-reference/src/soffice_tests.rs \
  crates/oku-slides-platform/src/config.default.toml crates/oku-slides-platform/src/config_tests.rs \
  crates/oku-slides-cli/src/import_reference.rs crates/oku-slides-cli/src/import.rs \
  crates/oku-slides-cli/tests/import_reference.rs
git commit -m "feat(reference) : soffice, le binaire réel de LibreOffice d'abord, un script refusé (t-912)"
```

---

### Task 3: The Seatbelt profile and the sandboxed command

**Files:**
- Create: `crates/oku-slides-render-worker/src/worker.sb`
- Create: `crates/oku-slides-render-worker/src/sandbox.rs`
- Create: `crates/oku-slides-render-worker/src/sandbox_tests.rs`
- Modify: `crates/oku-slides-render-worker/src/lib.rs` (add `mod sandbox;`)
- Modify: `crates/oku-slides-render-worker/Cargo.toml`. `tempfile` is a dev-dependency only, and
  `sandbox.rs` needs it in production on macOS. Add:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
tempfile.workspace = true
```

**Interfaces:**
- Consumes: `storage::canonical` (Task 1).
- Produces (crate-private):
  - `Sandbox::new(program: &Path, soffice: Option<&Path>) -> Result<Sandbox, RenderError>`;
  - `Sandbox::command(&self) -> Command`: `sandbox-exec … <worker>` with `TMPDIR` set and no
    worker arguments;
  - `Sandbox::work(&self) -> &Path` and `Sandbox::canary(&self) -> &Path`;
  - `sandbox_for(program: &Path, soffice: Option<&Path>) -> Result<Option<Sandbox>, RenderError>`:
    `Some` on macOS, `None` elsewhere.

- [ ] **Step 1: Write the profile** (`worker.sb`, exactly)

```scheme
;; Bac à sable du processus de rendu (t-912) : écrire seulement dans WORK.
;; Spec : docs/superpowers/specs/2026-10-17-render-worker-sandbox-design.md.
(version 1)
(allow default)
(deny file-write*)
(allow file-write*
  (subpath (param "WORK"))
  (literal "/dev/null") (literal "/dev/dtracehelper")
  ;; Le socket de LibreOffice, chemin codé en dur dans libuno_sal.
  (literal "/private/tmp")
  (require-all
    (regex (string-append "^/private/tmp/OSL_PIPE_" (param "UID") "_SingleOfficeIPC_[0-9A-Za-z]+$"))
    (vnode-type SOCKET)))
(deny network*)
(allow network* (local unix-socket) (remote unix-socket))
(deny process-exec*)
(allow process-exec (literal (param "WORKER")) (literal (param "SOFFICE")))
(deny appleevent-send)
(deny lsopen)
```

- [ ] **Step 2: Write the failing tests** (`sandbox_tests.rs`; each runs `/bin/bash` as the
  "worker")

```rust
use std::path::Path;
use std::process::Output;

use speculoos::prelude::*;

use super::*;

/// `/bin/bash -c script` sous le bac à sable d'un « travailleur » `/bin/bash`.
fn bash(sandbox: &Sandbox, script: &str) -> Output {
    sandbox.command().args(["-c", script]).output().unwrap()
}

fn sandbox() -> Sandbox {
    Sandbox::new(Path::new("/bin/bash"), None).unwrap()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn sandbox_write_inside_work_succeeds() {
    let s = sandbox();
    let out = bash(&s, &format!("echo x > '{}/f'", s.work().display()));
    assert_that!(out.status.success()).is_true();
    assert_that!(s.work().join("f").exists()).is_true();
}

#[test]
fn sandbox_write_outside_work_is_denied() {
    let s = sandbox();
    let outside = tempfile::tempdir().unwrap();
    let out = bash(&s, &format!("echo x > '{}/f'", outside.path().display()));
    assert_that!(out.status.success()).is_false();
    assert_that!(stderr(&out)).contains("Operation not permitted");
    assert_that!(outside.path().join("f").exists()).is_false();
}

#[test]
fn sandbox_canary_is_outside_work_and_cannot_be_created() {
    let s = sandbox();
    assert_that!(s.canary().starts_with(s.work())).is_false();
    let out = bash(&s, &format!("echo x > '{}'", s.canary().display()));
    assert_that!(stderr(&out)).contains("Operation not permitted");
}

#[test]
fn sandbox_tmpdir_is_its_work_dir() {
    let s = sandbox();
    let out = bash(&s, "printf %s \"$TMPDIR\"");
    assert_that!(String::from_utf8_lossy(&out.stdout).into_owned()).is_equal_to(format!("{}/", s.work().display()));
}

#[test]
fn sandbox_exec_of_another_program_is_denied() {
    let out = bash(&sandbox(), "/usr/bin/true");
    assert_that!(out.status.success()).is_false();
    assert_that!(stderr(&out)).contains("Operation not permitted");
}

#[test]
fn sandbox_ip_connection_is_denied() {
    // Hors bac à sable : « Connection refused » ; dedans, refusé avant tout paquet.
    let out = bash(&sandbox(), "exec 3<>/dev/tcp/127.0.0.1/9");
    assert_that!(stderr(&out)).contains("Operation not permitted");
}

#[test]
fn sandbox_hardlink_from_work_to_outside_is_denied() {
    let s = Sandbox::new(Path::new("/bin/ln"), None).unwrap();
    std::fs::write(s.work().join("f"), b"x").unwrap();
    let outside = tempfile::tempdir().unwrap();
    let out = s.command().arg(s.work().join("f")).arg(outside.path().join("l")).output().unwrap();
    assert_that!(out.status.success()).is_false();
    assert_that!(outside.path().join("l").exists()).is_false();
}

#[test]
fn sandbox_osl_pipe_rule_refuses_a_regular_file() {
    let s = sandbox();
    let uid = uid();
    let path = format!("/private/tmp/OSL_PIPE_{uid}_SingleOfficeIPC_okutest912");
    let out = bash(&s, &format!("echo x > '{path}'"));
    assert_that!(stderr(&out)).contains("Operation not permitted");
    assert_that!(Path::new(&path).exists()).is_false();
}

#[test]
fn sandbox_work_dir_is_removed_with_it() {
    let s = sandbox();
    let work = s.work().to_owned();
    drop(s);
    assert_that!(work.exists()).is_false();
}
```

- [ ] **Step 3: Run them and check they fail**

Run: `touch crates/*/src/lib.rs && cargo nextest run -p oku-slides-render-worker sandbox`
Expected: compile error: module `sandbox` not found.

- [ ] **Step 4: Implement `sandbox.rs`**

```rust
//! Le bac à sable Seatbelt du travailleur, sur macOS (t-912).
//!
//! `sandbox-exec -p <worker.sb>` pose le profil puis exécute le travailleur
//! (même PID : chien de garde, mémoire et `killpg` inchangés). Le travailleur
//! et LibreOffice, son fils, n'écrivent que dans `work` (leur `TMPDIR`), ne
//! joignent aucune adresse IP, n'exécutent que le travailleur et `soffice`,
//! n'envoient ni Apple Event ni `LSOpen`. Le canari, hors de `work`, prouve à
//! chaque `Hello` que le profil tient. Ailleurs que sur macOS : rien.

use std::path::Path;
use std::process::Command;

use oku_slides_render::RenderError;

/// Le bac à sable d'un travailleur lancé : son dossier, son canari.
#[cfg(target_os = "macos")]
#[derive(Debug)]
pub(crate) struct Sandbox {
    /// Gardé pour son `Drop` : supprimé avec le bac à sable, après la mort du travailleur (`Conn`).
    _dir: tempfile::TempDir,
    /// Le dossier, en chemin réel : Seatbelt compare des chemins réels.
    work: std::path::PathBuf,
    canary: std::path::PathBuf,
    worker: std::path::PathBuf,
    soffice: std::path::PathBuf,
}

/// Ailleurs que sur macOS, aucun bac à sable n'existe.
#[cfg(not(target_os = "macos"))]
#[derive(Debug)]
pub(crate) enum Sandbox {}

#[cfg(not(target_os = "macos"))]
impl Sandbox {
    /// Sans objet hors macOS.
    pub(crate) fn command(&self) -> Command {
        match *self {}
    }

    /// Sans objet hors macOS.
    pub(crate) fn work(&self) -> &Path {
        match *self {}
    }

    /// Sans objet hors macOS.
    pub(crate) fn canary(&self) -> &Path {
        match *self {}
    }
}

/// Le bac à sable du travailleur `program`, qui pourra lancer `soffice`.
#[cfg(target_os = "macos")]
pub(crate) fn sandbox_for(program: &Path, soffice: Option<&Path>) -> Result<Option<Sandbox>, RenderError> {
    Sandbox::new(program, soffice).map(Some)
}

/// Hors macOS, pas de bac à sable : le travailleur est lancé tel quel.
#[cfg(not(target_os = "macos"))]
pub(crate) fn sandbox_for(_program: &Path, _soffice: Option<&Path>) -> Result<Option<Sandbox>, RenderError> {
    Ok(None)
}

#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::{OsStr, OsString};
    use std::path::PathBuf;

    use oku_slides_platform::storage::canonical;

    use super::*;

    /// Pose le profil puis exécute le travailleur, sous le même PID.
    pub(crate) const SANDBOX_EXEC: &str = "/usr/bin/sandbox-exec";
    /// Le profil Seatbelt du travailleur.
    const PROFILE: &str = include_str!("worker.sb");
    /// `SOFFICE` sans LibreOffice : un chemin qui n'existe pas, rien d'autre ne s'exécute.
    const NO_PROGRAM: &str = "/var/empty/none";

    impl Sandbox {
        /// Un dossier neuf pour le travailleur `program`, qui pourra lancer `soffice`.
        pub(crate) fn new(program: &Path, soffice: Option<&Path>) -> Result<Self, RenderError> {
            let lost = |what: &str, e: &dyn std::fmt::Display| RenderError::worker_lost(format!("sandbox {what}: {e}"));
            let dir = tempfile::Builder::new()
                .prefix("oku-slides-worker-")
                .tempdir()
                .map_err(|e| lost("work dir", &e))?;
            let work = canonical(dir.path()).map_err(|e| lost("work dir", &e))?;
            let mut canary_name = work
                .file_name()
                .map(OsStr::to_owned)
                .ok_or_else(|| RenderError::worker_lost(format!("sandbox work dir {}", work.display())))?;
            canary_name.push(".canary");
            let canary = work.with_file_name(canary_name);
            let worker = canonical(program).map_err(|e| lost("worker", &e))?;
            let soffice = match soffice {
                Some(p) => canonical(p).map_err(|e| lost("soffice", &e))?,
                None => PathBuf::from(NO_PROGRAM),
            };
            Ok(Self { _dir: dir, work, canary, worker, soffice })
        }

        /// `sandbox-exec -p <profil> -D … <travailleur>`, `TMPDIR` posé, sans ses arguments.
        pub(crate) fn command(&self) -> Command {
            let mut cmd = Command::new(SANDBOX_EXEC);
            cmd.arg("-p").arg(PROFILE);
            for (name, value) in [
                ("WORK", self.work.as_os_str()),
                ("UID", OsStr::new(&uid())),
                ("WORKER", self.worker.as_os_str()),
                ("SOFFICE", self.soffice.as_os_str()),
            ] {
                cmd.arg("-D").arg(define(name, value));
            }
            let mut tmpdir = self.work.clone().into_os_string();
            tmpdir.push("/");
            cmd.arg(&self.worker).env("TMPDIR", tmpdir);
            cmd
        }

        /// Le seul dossier où le travailleur écrit (son `TMPDIR`).
        pub(crate) fn work(&self) -> &Path {
            &self.work
        }

        /// Un chemin hors de `work`, que le travailleur doit échouer à créer.
        pub(crate) fn canary(&self) -> &Path {
            &self.canary
        }
    }

    /// `NOM=valeur`, un paramètre du profil.
    fn define(name: &str, value: &OsStr) -> OsString {
        let mut d = OsString::from(name);
        d.push("=");
        d.push(value);
        d
    }

    /// L'utilisateur courant, dans le nom du socket de LibreOffice.
    pub(crate) fn uid() -> String {
        // SAFETY: getuid n'a ni précondition ni cas d'échec.
        unsafe { libc::getuid() }.to_string()
    }
}

#[cfg(all(test, target_os = "macos"))]
use macos::uid;

#[cfg(all(test, target_os = "macos"))]
#[path = "sandbox_tests.rs"]
mod sandbox_tests;
```

In `lib.rs`, add the module beside the other private modules. Nothing in non-test code uses it
until Task 5, so declare it with an expectation that Task 5 removes:

```rust
#[expect(dead_code, reason = "branché au lien à la tâche suivante (t-912)")]
mod sandbox;
```

- [ ] **Step 5: Run the tests and check they pass**

Run: `touch crates/*/src/lib.rs && cargo nextest run -p oku-slides-render-worker sandbox && cargo clippy -p oku-slides-render-worker --all-targets -- -D warnings`
Expected: all 9 tests pass; no clippy warning. `sandbox_ip_connection_is_denied` must say
"Operation not permitted", not "Connection refused". If it says the latter, the network rule
is not applied: stop and diagnose.

- [ ] **Step 6: Commit**

```bash
git add crates/oku-slides-render-worker/src/worker.sb crates/oku-slides-render-worker/src/sandbox.rs \
  crates/oku-slides-render-worker/src/sandbox_tests.rs crates/oku-slides-render-worker/src/lib.rs \
  crates/oku-slides-render-worker/Cargo.toml
git commit -m "feat(render-worker) : profil Seatbelt du travailleur et sa commande sandbox-exec (t-912)"
```

---

### Task 4: Worker side: canary in `Hello`, `--work-dir`, WORK emptied after each reference

**Files:**
- Modify: `crates/oku-slides-render-worker/src/protocol.rs` (`PROTOCOL_VERSION` l.9,
  `Request::Hello` l.26, `Response::Hello` l.129, a new `Canary` enum)
- Create: `crates/oku-slides-render-worker/src/confine.rs`
- Create: `crates/oku-slides-render-worker/src/confine_tests.rs`
- Modify: `crates/oku-slides-render-worker/src/serve.rs` (`Worker` l.64–76, `serve_watched`
  l.84, `handle` l.137–181, `WorkerArgs`/`worker_args` l.331–351, `exit_if_worker` l.373)
- Modify: `crates/oku-slides-render-worker/src/serve_tests.rs`
- Modify: `crates/oku-slides-render-worker/src/link.rs:88-96` (`check_hello` pattern only, to
  compile)
- Modify: `crates/oku-slides-render-worker/src/link_fake_tests.rs` (l.39, 54, 68: add
  `canary: None` to the `Request::Hello`/`Response::Hello` literals)

**Interfaces:**
- Consumes: `StorageErrorKind::PermissionDenied` (Task 1).
- Produces:
  - `Request::Hello { version: u32, canary: Option<PathBuf> }`;
  - `Response::Hello { version: u32, canary: Option<Canary> }`;
  - `pub(crate) enum Canary { Denied, Written, Failed }`;
  - `WorkerArgs { …, work: Option<PathBuf> }`, from `--work-dir <path>`;
  - `PROTOCOL_VERSION = 3`.

- [ ] **Step 1: Write the failing tests.** `confine_tests.rs`:

```rust
use speculoos::prelude::*;

use oku_slides_platform::storage::Storage;

use super::*;

#[test]
fn probe_canary_in_a_writable_dir_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let canary = probe_canary(&Storage::local().unwrap(), &dir.path().join("c"));
    assert_that!(canary).is_equal_to(Canary::Written);
}

#[cfg(unix)]
#[test]
fn probe_canary_in_a_read_only_dir_is_denied() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o555)).unwrap();
    let canary = probe_canary(&Storage::local().unwrap(), &dir.path().join("c"));
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_that!(canary).is_equal_to(Canary::Denied);
}

#[test]
fn wipe_empties_the_work_dir_and_keeps_it() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("a/b")).unwrap();
    std::fs::write(dir.path().join("a/b/f"), b"x").unwrap();
    std::fs::write(dir.path().join("g"), b"x").unwrap();
    wipe(&Storage::local().unwrap(), dir.path()).unwrap();
    assert_that!(dir.path().exists()).is_true();
    assert_that!(std::fs::read_dir(dir.path()).unwrap().count()).is_equal_to(0);
}
```

Append to `serve_tests.rs`, and add `use crate::protocol::Canary;` and
`use std::path::PathBuf;`. Change `Harness::new()` to delegate to a new
`Harness::with_work(None)`, which calls `serve_watched(req_r, resp_w, MAX, &Busy::default(), work)`.
The existing pattern `Response::Hello { version } if …` (l.293) becomes
`Response::Hello { version, .. } if …`.

```rust
#[test]
fn hello_without_canary_answers_none() {
    let mut h = Harness::new();
    let (resp, _) = h.call(&Request::Hello { version: PROTOCOL_VERSION, canary: None }, &[]);
    assert_that!(matches!(resp, Response::Hello { canary: None, .. })).is_true();
}

#[test]
fn hello_with_a_writable_canary_answers_written() {
    let dir = tempfile::tempdir().unwrap();
    let mut h = Harness::new();
    let canary = Some(dir.path().join("c"));
    let (resp, _) = h.call(&Request::Hello { version: PROTOCOL_VERSION, canary }, &[]);
    assert_that!(matches!(resp, Response::Hello { canary: Some(Canary::Written), .. })).is_true();
}

#[test]
fn worker_args_read_the_work_dir() {
    let args: Vec<OsString> = ["--render-worker", "--work-dir", "/w"].into_iter().map(OsString::from).collect();
    assert_that!(worker_args(&args).unwrap().work).is_equal_to(Some(PathBuf::from("/w")));
}

#[test]
fn reference_empties_the_work_dir_afterwards() {
    let work = tempfile::tempdir().unwrap();
    std::fs::write(work.path().join("planted"), b"x").unwrap();
    let mut h = Harness::with_work(Some(work.path().to_owned()));
    let req = Request::Reference { witness: crate::ReferenceWitness::Office2Pdf, fonts: vec![], slides: None };
    // `wipe_work` suit toute réponse : une charge vide suffit, sans rendu office2pdf.
    let _ = h.call(&req, &[]);
    assert_that!(work.path().join("planted").exists()).is_false();
    assert_that!(work.path().exists()).is_true();
}
```

Any `Hello { version: … }`
literal already in `serve_tests.rs` (l.292) or `link_tests.rs` gains `canary: None`.)

- [ ] **Step 2: Run them and check they fail**

Run: `touch crates/*/src/lib.rs && cargo nextest run -p oku-slides-render-worker`
Expected: compile errors (`Canary`, `confine`, `with_work`, `work` field).

- [ ] **Step 3: Implement.** `protocol.rs`:

```rust
pub(crate) const PROTOCOL_VERSION: u32 = 3;
```

```rust
    Hello {
        version: u32,
        /// Un chemin hors du dossier du travailleur : il doit échouer à le créer (t-912).
        #[serde(default)]
        canary: Option<PathBuf>,
    },
```

```rust
    /// La version du protocole du travailleur, en réponse à `Hello`, et ce qu'a donné son canari.
    Hello {
        version: u32,
        #[serde(default)]
        canary: Option<Canary>,
    },
```

```rust
/// Ce qu'a vu le travailleur en créant le canari de `Hello`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum Canary {
    /// Refusé par le système : le bac à sable tient.
    Denied,
    /// Créé : aucun bac à sable.
    Written,
    /// Autre échec : rien n'est prouvé.
    Failed,
}
```

`confine.rs`:

```rust
//! Le côté travailleur du bac à sable : le canari de `Hello` et le dossier vidé.

use std::path::Path;

use oku_slides_platform::storage::{Storage, StorageError, StorageErrorKind};

use crate::protocol::Canary;

/// Tente de créer le fichier vide `path` ; dit ce que le système a répondu.
pub(crate) fn probe_canary(storage: &Storage, path: &Path) -> Canary {
    match storage.write(path, Vec::new()) {
        Ok(()) => Canary::Written,
        Err(e) if e.kind() == StorageErrorKind::PermissionDenied => Canary::Denied,
        Err(e) => {
            tracing::warn!(error = %e, path = %path.display(), "canari du bac à sable");
            Canary::Failed
        }
    }
}

/// Vide `work` et le recrée : un deck ne laisse rien au suivant.
pub(crate) fn wipe(storage: &Storage, work: &Path) -> Result<(), StorageError> {
    storage.delete_dir(work)?;
    storage.create_dir_all(work)
}

#[cfg(test)]
#[path = "confine_tests.rs"]
mod confine_tests;
```

`serve.rs`:
0. Imports: `use std::path::PathBuf;` in `serve.rs` and in `protocol.rs`.
1. Add `work: Option<PathBuf>` to `Worker`, with the doc `/// Le dossier du travailleur (son TMPDIR), vidé après chaque référence.`
2. Factor the lazy storage of `reference()` (l.209–213) into
   `fn storage(&mut self) -> Result<Storage, WireError>` and use it there.
3. Give `serve_watched` a 5th parameter, `work: Option<PathBuf>`, and build
   `Worker { work, ..Worker::default() }`. `pub fn serve` passes `None`; `exit_if_worker` passes
   `args.work`.
4. In `handle`:

```rust
            Request::Hello { canary, .. } => {
                let canary = match canary {
                    Some(path) => Some(confine::probe_canary(&self.storage()?, &path)),
                    None => None,
                };
                Ok((Response::Hello { version: PROTOCOL_VERSION, canary }, Vec::new()))
            }
```

   (If `handle`'s arms do not return `Result` at that point, wrap the arm in a closure the way
   the other fallible arms do.) Then:

```rust
            Request::Reference { witness, fonts, slides } => {
                let reply = self.reference(&witness, &fonts, slides, &payload);
                self.wipe_work();
                reply
            }
```

```rust
    /// Vide le dossier du travailleur ; un échec est journalisé, la réponse reste.
    fn wipe_work(&mut self) {
        let Some(work) = self.work.clone() else { return };
        let wiped = self.storage().map_err(|e| format!("{e:?}")).and_then(|s| {
            confine::wipe(&s, &work).map_err(|e| e.to_string())
        });
        if let Err(e) = wiped {
            tracing::warn!(error = %e, work = %work.display(), "dossier du travailleur non vidé");
        }
    }
```

5. `WorkerArgs` gains `pub(crate) work: Option<PathBuf>`. In `worker_args`:

```rust
    let work = args.iter().position(|a| a == "--work-dir").and_then(|i| args.get(i + 1)).map(PathBuf::from);
```

   Add `work` to the struct literal and `--work-dir <path>` to both doc comments.

6. In `lib.rs`, add `mod confine;`.

7. In `link.rs::check_hello`, keep today's behavior by matching `Response::Hello { version, .. }`
   in both arms. Task 5 changes the logic.

- [ ] **Step 4: Run the tests and check they pass**

Run: `touch crates/*/src/lib.rs && cargo nextest run -p oku-slides-render-worker`
Expected: all pass. `serve.rs` must stay under 500 lines (`wc -l`); if it doesn't, move
`wipe_work` into `confine.rs` as `wipe_logged(storage, work)`.

- [ ] **Step 5: Commit**

```bash
git add crates/oku-slides-render-worker/src/protocol.rs crates/oku-slides-render-worker/src/confine.rs \
  crates/oku-slides-render-worker/src/confine_tests.rs crates/oku-slides-render-worker/src/serve.rs \
  crates/oku-slides-render-worker/src/serve_tests.rs crates/oku-slides-render-worker/src/link.rs \
  crates/oku-slides-render-worker/src/lib.rs crates/oku-slides-render-worker/src/link_fake_tests.rs
git commit -m "feat(render-worker) : canari dans Hello, --work-dir vidé après chaque référence, protocole 3 (t-912)"
```

---

### Task 5: Host side: spawn under the sandbox, require the canary, allow soffice

**Files:**
- Modify: `crates/oku-slides-render-worker/src/link.rs`:
  - `check_hello` l.86–96;
  - `worker_command` l.98–111;
  - `Conn` l.115–125: a new field, last;
  - `Link` struct and `new` l.147–175: a new `soffice` field;
  - `hello` l.274–278;
  - `spawn` l.281–289.
- Modify: `crates/oku-slides-render-worker/src/client.rs` (a new `with_soffice` after `new`, l.167)
- Modify: `crates/oku-slides-render-worker/src/link_tests.rs`
- Modify: `crates/oku-slides-render-worker/src/link_fake_tests.rs` (every `Link`/`RenderWorker`
  built on a `Fake` sets `unsandboxed`)
- Modify: `crates/oku-slides-render-worker/src/lib.rs` (drop the `#[expect(dead_code)]` of Task 3)
- Modify: `crates/oku-slides-cli/src/import.rs:279-280`

**Interfaces:**
- Consumes: `sandbox_for`, `Sandbox::{command, work, canary}` (Task 3); `Canary` and the new
  `Hello` (Task 4); `find_soffice`'s canonical path (Task 2).
- Produces:
  - `pub fn RenderWorker::with_soffice(self, program: PathBuf) -> Self`;
  - `check_hello(reply: &Response, canary: Option<&Path>) -> Result<(), RenderError>`;
  - `worker_command(program, settings, sandbox: Option<&Sandbox>) -> Command`.

- [ ] **Step 1: Write the failing tests** (`link_tests.rs`; replace the existing `check_hello`
  tests and keep their protocol-mismatch cases with `None` as `canary`)

```rust
#[rstest::rstest]
#[case::no_canary_asked(None, None, true)]
#[case::denied(Some("/c"), Some(Canary::Denied), true)]
#[case::written(Some("/c"), Some(Canary::Written), false)]
#[case::failed(Some("/c"), Some(Canary::Failed), false)]
#[case::not_answered(Some("/c"), None, false)]
fn check_hello_with_a_canary_requires_it_denied(
    #[case] asked: Option<&str>,
    #[case] seen: Option<Canary>,
    #[case] ok: bool,
) {
    let reply = Response::Hello { version: PROTOCOL_VERSION, canary: seen };
    let result = check_hello(&reply, asked.map(Path::new));
    assert_that!(result.is_ok()).is_equal_to(ok);
    if let Err(e) = result {
        assert_that!(e.kind()).is_equal_to(RenderErrorKind::WorkerLost);
    }
}

#[test]
fn worker_command_without_sandbox_runs_the_program() {
    let cmd = worker_command(Path::new("/bin/w"), &settings(30_000), None);
    assert_that!(cmd.get_program().to_owned()).is_equal_to(std::ffi::OsString::from("/bin/w"));
}

#[cfg(target_os = "macos")]
#[test]
fn worker_command_with_sandbox_runs_sandbox_exec_with_the_work_dir() {
    let sandbox = crate::sandbox::Sandbox::new(Path::new("/bin/bash"), None).unwrap();
    let cmd = worker_command(Path::new("/bin/bash"), &settings(30_000), Some(&sandbox));
    assert_that!(cmd.get_program().to_owned()).is_equal_to(std::ffi::OsString::from("/usr/bin/sandbox-exec"));
    let args: Vec<_> = cmd.get_args().map(|a| a.to_owned()).collect();
    let i = args.iter().position(|a| a == "--work-dir").unwrap();
    assert_that!(PathBuf::from(&args[i + 1])).is_equal_to(sandbox.work().to_owned());
}
```

Use the existing `settings(timeout_ms)` helper of `link_tests.rs` (l.13), and import
`crate::protocol::Canary` and `std::path::{Path, PathBuf}` if they are not already imported.
The existing `worker_command_passes_bounds_and_marks_the_child_as_a_worker` (l.199–216) gains
`None` as its third argument; its expected arguments are unchanged.

- [ ] **Step 2: Run them and check they fail**

Run: `touch crates/*/src/lib.rs && cargo nextest run -p oku-slides-render-worker link`
Expected: compile errors (arity of `check_hello` and `worker_command`).

- [ ] **Step 3: Implement.** `link.rs`:

```rust
/// La réponse à `Hello` dit la version du protocole de l'hôte ; sinon `Protocol`.
///
/// Avec `canary` (le bac à sable de macOS), le travailleur doit avoir échoué à
/// le créer (`Denied`) ; sinon le bac à sable ne tient pas : `WorkerLost`, jamais
/// de travailleur sans lui (t-912).
pub(crate) fn check_hello(reply: &Response, canary: Option<&Path>) -> Result<(), RenderError> {
    match reply {
        Response::Hello { version, canary: seen } if *version == PROTOCOL_VERSION => match (canary, seen) {
            (None, _) | (Some(_), Some(Canary::Denied)) => Ok(()),
            (Some(path), seen) => Err(RenderError::worker_lost(format!(
                "sandbox not enforced: canary {} answered {seen:?}",
                path.display()
            ))),
        },
        Response::Hello { version, .. } => Err(RenderError::protocol(PROTOCOL_VERSION, Some(*version))),
        _ => Err(RenderError::protocol(PROTOCOL_VERSION, None)),
    }
}
```

`worker_command`:

```rust
/// La commande d'un travailleur `program` : ses bornes (le chien de garde
/// du fils les applique si l'hôte disparaît), ses tubes, et
/// [`RENDER_WORKER_ENV`] : le fils ne lance jamais de travailleur à son tour.
/// Avec `sandbox`, `sandbox-exec` le lance et `--work-dir` lui dit son dossier.
pub(crate) fn worker_command(program: &Path, settings: &RenderSettings, sandbox: Option<&Sandbox>) -> Command {
    let mut cmd = match sandbox {
        Some(sandbox) => sandbox.command(),
        None => Command::new(program),
    };
    cmd.args(["--render-worker", "--max-frame-bytes", &settings.max_frame_bytes().to_string()])
        .args(["--timeout-ms", &settings.timeout_ms.to_string()])
        .args(["--memory-bytes", &settings.memory_bytes().to_string()]);
    if let Some(sandbox) = sandbox {
        cmd.arg("--work-dir").arg(sandbox.work());
    }
    cmd.env(RENDER_WORKER_ENV, "1").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit());
    cmd
}
```

Off macOS, `Sandbox` is the empty enum from Task 3, whose methods are `match *self {}`, so
this code compiles everywhere. Remove `#[expect(dead_code, …)]` from `mod sandbox;` in `lib.rs`.

`Conn`: add a last field, with its doc:

```rust
    /// Le dossier et le canari du travailleur ; dernier champ : supprimé après sa mort.
    sandbox: Option<Sandbox>,
```

`Link`: add `pub(crate) soffice: Option<PathBuf>` (`/// Le soffice que le bac à sable laisse lancer.`),
set to `None` in `new`. Add a test-only field, which exists in no build the user runs:

```rust
    /// Faux travailleurs des tests (scripts shell, qui exécutent `head` et `cat`) : sans bac à sable.
    #[cfg(test)]
    pub(crate) unsandboxed: bool,
```

Set it to `#[cfg(test)] unsandboxed: false,` in `new`. In `link_fake_tests.rs`, every test that
builds a `Link` from a `Fake` sets `link.unsandboxed = true;` before its first call. A test
that builds a `RenderWorker` does `worker.link().unsandboxed = true;`. `spawn` returns the sandbox with the pipes:
`type Pipes = (Box<dyn Write + Send>, Box<dyn Read + Send>, Option<Child>, Option<Sandbox>)`.
In the `Process` arm:

```rust
            RenderLaunch::Process { program } => {
                let sandbox = if self.unsandboxed() { None } else { sandbox_for(program, self.soffice.as_deref())? };
                let mut child = worker_command(program, &self.settings, sandbox.as_ref())
                    .spawn()
                    .map_err(|e| RenderError::worker_lost(format!("spawn {}: {e}", program.display())))?;
                let stdin = child.stdin.take().ok_or_else(|| RenderError::worker_lost("no stdin"))?;
                let stdout = child.stdout.take().ok_or_else(|| RenderError::worker_lost("no stdout"))?;
                Ok((Box::new(stdin), Box::new(stdout), Some(child), sandbox))
            }
```

with

```rust
    /// Vrai pour les faux travailleurs des tests seulement.
    fn unsandboxed(&self) -> bool {
        #[cfg(test)]
        return self.unsandboxed;
        #[cfg(not(test))]
        false
    }
```

(If clippy flags `needless_return`, write two `#[cfg]` versions of the method.) The `InProcess`
arm returns `None` as its 4th element. `connect` destructures the 4-tuple and
stores `sandbox` in `Conn`. `hello`:

```rust
    /// Poignée de main : version du protocole de l'hôte, et canari refusé sous bac à sable.
    fn hello(&mut self) -> Result<(), RenderError> {
        let canary = self.conn.as_ref().and_then(|c| c.sandbox.as_ref()).map(|s| s.canary().to_owned());
        let req = Request::Hello { version: PROTOCOL_VERSION, canary: canary.clone() };
        let (reply, _) = self.raw_call(&req, &[])?;
        check_hello(&reply, canary.as_deref())
    }
```

`client.rs`, after `new`:

```rust
    /// Le travailleur pourra lancer `program` (LibreOffice), son bac à sable l'autorisant.
    ///
    /// À poser avant la première requête ; `program` est le chemin de
    /// `ReferenceWitness::Soffice` (le binaire réel trouvé par `find_soffice`).
    pub fn with_soffice(self, program: PathBuf) -> Self {
        self.link().soffice = Some(program);
        self
    }
```

`crates/oku-slides-cli/src/import.rs`, where the witness worker is built:

```rust
            let mut worker = RenderWorker::new(platform.render_launch.clone(), settings);
            if let ReferenceWitness::Soffice { program, .. } = &witness {
                worker = worker.with_soffice(program.clone());
            }
```

- [ ] **Step 4: Run the tests and check they pass**

Run: `touch crates/*/src/lib.rs && cargo nextest run -p oku-slides-render-worker -p oku-slides-cli -p oku-slides-session`
Expected: all pass. On macOS, `tests/process.rs` now runs every real worker under the sandbox
and the canary: these tests passing is the integration proof. If `sandbox-exec` itself fails
(`sandbox_apply: Operation not permitted`), first check whether the shell running the tests is
already sandboxed (an agent harness). Nested `sandbox-exec` worked on this machine on
2026-10-17, so confirm that cause before changing anything. A Hello failure that mentions
"sandbox not enforced" or `sandbox-exec` means the profile or the paths are wrong. Diagnose
with `sandbox_tests` (G14); never loosen `check_hello`.

- [ ] **Step 5: Run the real LibreOffice end to end, under the sandbox**

Run: `touch crates/*/src/lib.rs && OKU_SLIDES_TEST_SOFFICE=1 cargo nextest run -p oku-slides-cli soffice && OKU_SLIDES_TEST_SOFFICE=1 cargo nextest run -p oku-slides-reference`
Expected: pass, with `import_hifi_with_reference_soffice_scores_against_real_libreoffice`
included. Then check that nothing leaked, in a temp dir of its own (parallel sessions share `$TMPDIR`):
`T=$(mktemp -d) && touch crates/*/src/lib.rs && TMPDIR=$T/ OKU_SLIDES_TEST_SOFFICE=1 cargo nextest run -p oku-slides-cli soffice && ls "$T" | grep -c oku-slides-worker-`
prints 0.

- [ ] **Step 6: Commit**

```bash
git add crates/oku-slides-render-worker/src/link.rs crates/oku-slides-render-worker/src/link_tests.rs \
  crates/oku-slides-render-worker/src/link_fake_tests.rs crates/oku-slides-render-worker/src/lib.rs \
  crates/oku-slides-render-worker/src/client.rs crates/oku-slides-cli/src/import.rs
git commit -m "feat(render-worker) : travailleur lancé sous bac à sable sur macOS, refusé sans canari refusé (t-912)"
```

---

### Task 6: Docs, rules, app check, full gate

**Files:**
- Modify: `crates/oku-slides-render-worker/ARCHITECTURE.md` (≤ 50 lines)
- Modify: `crates/oku-slides-reference/ARCHITECTURE.md` (Isolation section, `find_soffice` line)
- Modify: `crates/oku-slides-render-worker/AGENTS.md`
- Modify: `crates/oku-slides-reference/AGENTS.md`

- [ ] **Step 1: ARCHITECTURE.md.** In the render worker's file, add a bullet after `Link`:

```markdown
- `Sandbox` (`sandbox.rs`, macOS, t-912): `Link` spawns `sandbox-exec -p worker.sb` around the
  worker. It writes only in a fresh work dir (its `TMPDIR`, `--work-dir`, emptied after each
  reference, deleted with the connection), no IP, exec of itself and the allowed `soffice`
  only (`RenderWorker::with_soffice`). `Hello` carries a canary outside that dir: the worker must
  fail to create it (`Canary::Denied`) or it is refused (`WorkerLost`). No switch; other OSes and
  `InProcess` are unsandboxed.
```

The file is at 49 lines: cut about 5 lines elsewhere, for example by merging the
`Request`/`Response` and frames bullets. `oku-slides-reference/ARCHITECTURE.md` is at exactly
50 lines, so the replacement text there must not add a line. In `oku-slides-reference/ARCHITECTURE.md`,
replace the first sentence of Isolation with "The witness reads an untrusted file: it runs in
its own render worker, bounded in time and memory and, on macOS, sandboxed to write only in its
own dir (t-912)." Change the `find_soffice` text to "the first existing candidate, as its real
path; a script is refused (`Render`)".

- [ ] **Step 2: Rules.** In `crates/oku-slides-render-worker/AGENTS.md`, under Invariants:

```markdown
- Spawn a macOS worker only through sandbox-exec with worker.sb, and refuse it unless its Hello canary is denied. (spec:2026-10-17-render-worker-sandbox-design, t-912)
- Never hardlink a file into the worker's work dir; Seatbelt checks the path, so copy it. (spec:2026-10-17-render-worker-sandbox-design, t-912)
```

Under Pitfalls:

```markdown
- Keep allow default (LibreOffice needs the launchservicesd lookup) and the /tmp OSL_PIPE socket rule in worker.sb. (spec:2026-10-17-render-worker-sandbox-design, t-912)
```

In `crates/oku-slides-reference/AGENTS.md`, under Settled decisions:

```markdown
- Run soffice only as its real binary; find_soffice refuses a script, since the sandbox execs nothing else. (spec:2026-10-17-render-worker-sandbox-design, t-912)
```

- [ ] **Step 3: App check (the session worker, without soffice).** Use the `run-oku-slides`
  skill with fresh `OKU_SLIDES_PATHS__DATA`/`OKU_SLIDES_PATHS__DECKS` temp dirs and a free
  port. Open the fixture deck; the slides must render. Then run
  `ps -o pid,command -ax | grep -- --render-worker` and check that the worker carries
  `--work-dir`.

- [ ] **Step 4: Full gate**

Run: `touch crates/*/src/lib.rs && just ci`
Expected: green, including `agents-lint` (rule sources), clippy `-D warnings` and every line
cap. Then `touch crates/*/src/lib.rs && OKU_SLIDES_TEST_SOFFICE=1 cargo nextest run -p oku-slides-cli -p oku-slides-reference`.

- [ ] **Step 5: Commit**

```bash
git add crates/oku-slides-render-worker/ARCHITECTURE.md crates/oku-slides-reference/ARCHITECTURE.md \
  crates/oku-slides-render-worker/AGENTS.md crates/oku-slides-reference/AGENTS.md
git commit -m "docs(render-worker) : bac à sable macOS dans ARCHITECTURE et règles (t-912)"
```
