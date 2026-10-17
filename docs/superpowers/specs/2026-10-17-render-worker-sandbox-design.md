# Render worker filesystem sandbox (macOS) — design

Task: t-912 (follow-up of t-898). Date: 2026-10-17.
Status: approved 2026-10-17. The user's answers: the soffice `.app` binary comes first, scripts
are refused, and there is no switch. Plan: `docs/superpowers/plans/2026-10-17-render-worker-sandbox.md`.

## Problem

The render worker (`crates/oku-slides-render-worker`) runs Typst on deck sources and pack code,
and the reference witnesses on untrusted `.pptx` files: `office2pdf` in-process, LibreOffice
(`soffice`) as a child. It is bounded in time and memory only (`oku-slides-reference`
ARCHITECTURE.md, Isolation). t-898 showed what that leaves open: `office2pdf` 0.8.1 wrote an
embedded font under an attacker-chosen path (`~/Library/Fonts/…`, any user-writable directory).
The fix strips font parts before conversion, but that is one patch for one flaw. Every later
"a parser writes where the document says" bug in office2pdf, Typst or LibreOffice will need
its own patch. The goal here is to close the whole class: **the worker and everything it
starts can open for writing only paths inside its own temp dir.** Writes made on its behalf by
daemons it can still reach (cfprefsd, launchservicesd, fontd through `allow default`) and
writes to file descriptors inherited from the host (stderr) are not covered (see Limits).

## Threat model

| Tier | Attacker gets | In scope |
|---|---|---|
| 1. Confused deputy | A library bug makes the worker's *own* code write to a path the document picks (t-898). No code execution. | **Yes: closed completely.** |
| 2. Code execution in the worker or soffice | Memory corruption in LibreOffice's C++ parsers, for example. | Raised bar only (see Limits). |

Reads are out of scope. At tier 1 a parser bug cannot send what it reads anywhere: IP is
denied, and the pipe back to the host carries only PDFs and PNGs shown locally. Tier 2 has
other ways out (see Limits).

## What the worker actually writes (measured on macOS 27.0.1, LibreOffice 26.8.1.1)

- **Typst** (deck rendering, pack probes, charts, export): nothing. Typst has no file-write
  API; fonts arrive in memory over the pipe; system fonts are only read. Logs go to stderr
  (`telemetry::init`), and the protocol goes to a copy of stdout.
- **office2pdf**: only embedded fonts, under `std::env::temp_dir()` (`embedded_fonts.rs:168`,
  `:561`). After t-898 nothing reaches it. Under the sandbox, the t-898 write would be refused
  even without the stripping.
- **The Soffice witness (our code)**: a `tempfile` dir in `$TMPDIR` holding the profile, fonts,
  `deck.pptx` and `out/deck.pdf`.
- **soffice**:
  - its isolated profile (`-env:UserInstallation`, inside that dir);
  - `lu*.tmp` files in `$TMPDIR`;
  - **the single-instance IPC socket `/tmp/OSL_PIPE_<uid>_SingleOfficeIPC_<hash>`.** This path
    is hard-coded in `libuno_sal` (`/tmp`, then `/var/tmp`). No setting moves it: this build
    has no `OSL_SOCKET_PATH`. Creating it also needs write access on the `/private/tmp`
    directory node itself. A killed soffice leaves its socket behind: four stale ones were in
    `/tmp` on this machine.
  - Fontconfig warns about a missing `<cachedir>` but needs no writable cache.
  - soffice forks nothing and execs nothing (it ran fine with `process-fork` denied). It does
    need `mach-lookup` of `com.apple.coreservices.launchservicesd`: without it, it aborts (exit 134).

Method: bisecting Seatbelt write allowances on soffice (`--convert-to pdf`, the same arguments as
`soffice.rs`), and `find -newer` over `$HOME/Library`, `~/.cache` and `/var/folders` after an
unsandboxed run, then repeated after a sandboxed run with the final profile. That second pass
also covered `~/Library/Preferences`, `~/Library/Fonts` and the LaunchServices database:
nothing new appeared. `(with report)` logging did not show up in `log show`, so bisection
replaced it.

## Options

| Option | Verdict |
|---|---|
| **A. `sandbox-exec -p <profile> -D …` wrapping the worker spawn (Seatbelt)** | **Chosen.** No FFI, no `unsafe`. The profile applies before the worker's first instruction, and `exec` keeps the PID, so the watchdog, RSS sampling and `killpg` are unchanged. Inherited by soffice. ~9 ms per spawn. |
| B. `sandbox_init_with_parameters` called by the worker on itself | Same kernel mechanism (Chrome, Firefox, WebKit do this). It is private SPI, needs an `unsafe` FFI, and leaves a window before the call. Kept as the fallback if Apple ever removes `/usr/bin/sandbox-exec`: same profile text. |
| C. App Sandbox (entitlements) | The worker is the app binary itself (`current_exe --render-worker`), so App Sandbox would sandbox the whole app: decks anywhere, ACP agents, LibreOffice. A separate signed helper with `com.apple.security.inherit` needs signing in dev and in nextest, and a container dir. Far costlier, and an App-Sandboxed parent could no longer `sandbox_init` (nesting is refused). Rejected. |
| D. OpenDAL / `Storage` rooted in the temp dir | Confines only I/O made through `Storage`. office2pdf calls `std::fs` directly, and soffice is another process. It cannot close the class. Rejected; `Storage` stays for our own writes. |
| E. Patch each flaw (status quo) | Unbounded cost, one advisory at a time. |
| F. Other users, chroot, containers | Need root, or do not exist on macOS. |

`sandbox-exec(1)` is marked deprecated, but it ships in macOS 27.0.1 and underlies Codex CLI,
Claude Code, Bazel and Nix sandboxing. The risk is accepted, and B is the exit.

## Profile (`crates/oku-slides-render-worker/src/worker.sb`, `include_str!`)

```scheme
(version 1)
(allow default)
(deny file-write*)
(allow file-write*
  (subpath (param "WORK"))
  (literal "/dev/null") (literal "/dev/dtracehelper")
  (literal "/private/tmp")                      ; soffice IPC socket, path hard-coded by LO
  (require-all
    (regex (string-append "^/private/tmp/OSL_PIPE_" (param "UID") "_SingleOfficeIPC_[0-9A-Za-z]+$"))
    (vnode-type SOCKET)))
(deny network*)
(allow network* (local unix-socket) (remote unix-socket))       ; the OSL pipe
(deny process-exec*)
(allow process-exec (literal (param "WORKER")) (literal (param "SOFFICE")))
(deny appleevent-send)
(deny lsopen)
```

The OSL rule is anchored and restricted to sockets. A plain prefix regex let the sandboxed
process create directory trees, regular files and symlinks named `OSL_PIPE_*` in `/tmp`
(security review). With `vnode-type SOCKET`, `bind` works, while a file or a directory is
refused. `/dev/tty` is not needed. Both points were checked with real soffice.

Every rule was checked by experiment:

- `oku-slides import --mode hifi` on `fixtures/import/libreoffice.pptx`, with both
  `--reference office2pdf` and `--reference soffice`, run with the whole CLI under the profile:
  same reports as without it.
- An earlier version of the profile was used here: it differed only in the OSL rule and
  `/dev/tty`. soffice alone was re-run with this exact profile, `TMPDIR=$WORK/` and the
  `WORKER`/`SOFFICE` exec rules.
- The security review also verified, from inside the sandbox, that all of these on an outside
  file are refused:
  - `link()` into WORK
  - writing through a symlink or `..`
  - `chmod`, `utimes`, `setxattr`, `chflags`, `truncate`
  - `mkfifo`
  - `renamex_np` swap, `exchangedata`
  - `clonefile`
  - renaming another entry of `/tmp`
- Under the profile, a write to `~/Library/Fonts/zz-oku-probe-regular.ttf` is refused, a write
  to `$WORK` succeeds, and exec of `osascript`, `curl` and `touch` is refused.
- Timing over 2 runs each: office2pdf 5.11–5.58 s unsandboxed and 4.96–5.21 s sandboxed; soffice
  6.87–7.68 s unsandboxed and 6.66–6.79 s sandboxed. There is no measurable render cost.

`allow default` keeps mach lookups, reads, sysctl and IOKit open. That is why LibreOffice and
CoreText work without a long allow list, and also why tier 2 is not contained (see Limits).

## Design

1. **Spawn** (`link.rs::worker_command`, `#[cfg(target_os = "macos")]`):
   - the command becomes `/usr/bin/sandbox-exec -p <profile> -D WORK=… -D WORKER=… -D SOFFICE=…
     <program> --render-worker …`;
   - a new `sandbox.rs` module builds it, with unit tests on the arguments;
   - other platforms are unchanged and documented as unsandboxed.
2. **`WORK`**:
   - a fresh `tempfile::TempDir` per spawn, owned by the `Link` and dropped (deleted) when the
     worker exits or is restarted. Restarts never reuse a dir, so soffice leftovers go away;
   - the child gets `TMPDIR=$WORK/`, so `tempfile` in `soffice.rs` and office2pdf's
     `temp_dir()` land there with no code change in `oku-slides-reference`;
   - paths are canonical (`/private/var/…`), because Seatbelt matches resolved paths;
   - `UID` is `getuid()`.
   - **Isolation between requests:** the worker empties WORK after each `Reference` request
     (`--work-dir`). One hostile deck then cannot plant files for a later deck. A per-request
     subdir would need a per-request `TMPDIR` for office2pdf, which runs in-process: `set_var`
     in a multithreaded process, which is `unsafe`.
   - **Invariant:** the host never hardlinks anything into WORK (copy only). Seatbelt checks
     the path, not the inode, so a hardlink placed there before sandboxing writes through to
     its target (verified).
   - **Disk:** WORK has no size bound. A follow-up may add one if a zip or font bomb proves it
     matters; the time bound already caps it in practice.
3. **`WORKER`**: `program`, canonicalized.
4. **`SOFFICE`**: the soffice binary allowed to run.
   - **Problem found:** the default `[import] soffice` list tries `soffice` from `PATH` first.
     On Homebrew that is a bash wrapper (`exec /Applications/…/soffice`), and the exec of
     `/bin/bash` is refused: the witness then reports "could not read the presentation".
   - Proposal: on macOS, put `/Applications/LibreOffice.app/Contents/MacOS/soffice` first in
     the defaults.
   - Proposal: the host resolves the candidate once, at spawn, with `find_soffice` plus
     canonicalization, and sends **that same path** in `ReferenceWitness::Soffice`.
   - A candidate that is a script (`#!`) gets an explicit `Render` error naming it. It is never
     run unsandboxed.
   - With no LibreOffice found, `SOFFICE` is set to `/var/empty/none`, so nothing else can be
     executed.
5. **Fail closed.** No setting turns the sandbox off.
   - **Canary in the handshake:** `Hello` carries a canary path outside WORK, in the host's own
     temp dir. The worker tries to create it and must answer `EPERM`, or the host returns
     `WorkerLost`. A `sandbox-exec` turned into a no-op, or a widened profile, is caught at
     every spawn rather than assumed. `PROTOCOL_VERSION` is bumped.
   - Missing `/usr/bin/sandbox-exec`, or a profile it rejects (exit 65/71 on stderr), means the
     `Hello` fails, which gives `RenderError::WorkerLost` with the cause in the logs.
   - Visible behavior is the one users already know for a worker that does not start; there is
     never a silent unsandboxed fallback (root invariant).
   - `RenderLaunch::InProcess` (tests and tools only) stays unsandboxed. The existing pitfall
     already says InProcess bounds nothing.
6. **Tests** (TDD, `sandbox_tests.rs` plus regressions):
   - `sandbox_write_outside_work_is_denied` and `sandbox_write_inside_work_succeeds`: the
     wrapped command runs `/bin/bash -c 'echo > …'` with `WORKER=/bin/bash`;
   - `sandbox_exec_of_other_program_is_denied`;
   - `sandbox_ip_connect_is_denied`;
   - a worker integration test: a real `--render-worker` child renders a deck under the sandbox;
   - the soffice end-to-end test runs under the sandbox behind `OKU_SLIDES_TEST_SOFFICE=1`;
   - `fake_soffice` (a `#!/bin/sh` script) stays in the worker's unit tests, which run
     `InProcess` and unsandboxed. The CLI's fake-script test becomes a "script refused" test,
     and the sandboxed soffice path is covered by the real-LibreOffice test;
   - the canary: a worker spawned with a profile that allows the canary path is refused;
   - the hardlink invariant: `link()` from WORK to outside is denied.
7. **Docs and rules**:
   - ARCHITECTURE.md of the render worker and of `oku-slides-reference` (Isolation);
   - a render-worker invariant: "Spawn the worker only through `sandbox::command` on macOS;
     never fall back unsandboxed";
   - a pitfall: "LibreOffice needs `/tmp/OSL_PIPE_*` and `launchservicesd`; keep them in the
     profile".

## Cost

- Code: about 150 lines for `sandbox.rs` and its tests, 30 lines in `link.rs`, the `.sb`
  profile, and the `find_soffice` resolution moved to the host.
- Runtime: about 9 ms per worker spawn. No measurable render cost.
- Maintenance: the profile has to be re-checked when LibreOffice or macOS moves, and the
  `OKU_SLIDES_TEST_SOFFICE=1` test catches that. The bash-wrapper soffice is no longer
  supported.

## Limits (tier 2), deliberately left open

With `allow default`, code running inside the worker or soffice can still:

- write through inherited file descriptors (stderr is `Stdio::inherit()`);
- reach the controlling TTY through `ioctl` (dev builds run from a terminal);

- read any user file;
- talk to unsandboxed services through mach/XPC (for example, ask `launchd` to run a job);
- reach other local unix sockets, such as a user's own LibreOffice instance.

Apple Events, `LSOpen` and exec are closed. Each remaining hole needs code execution, not a
parser bug. Closing them needs a `deny default` profile with an allow list (the Chrome
renderer approach), which LibreOffice makes costly. That would be a separate task, if t-912 ships
and LibreOffice becomes a default witness. Linux (Landlock) and Windows (AppContainer) are out
of scope too.

## Follow-ups noticed

- A soffice killed at its deadline leaves `/tmp/OSL_PIPE_*` sockets (harmless, but they pile
  up). A minor ticket if it matters.
