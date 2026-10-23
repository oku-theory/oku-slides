# crates/oku-slides-render-worker — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants

- Drop only the widget files that fail to compile, with their diagnostics, and keep the pack's other valid widgets. (plan:2026-10-02-creation-widgets)
- Never take the link lock in a Drop; queue Release and CloseSlot in the separate deferred mutex. (plan:2026-09-29-compilation-bornee)
- Refuse to render a slot whose pack is faulty with pack_disabled; never silently render with core widgets only. (plan:2026-09-29-compilation-bornee)
- Never blame a pack for failures during widget editing or preview; turn a probe timeout into a file diagnostic. (plan:2026-10-02-creation-widgets)
- Do not retry a faulty pack in the same session until its source fingerprint changes. (spec:2026-09-29-compilation-bornee-design)
- Apply the watchdog memory limit only during a request; detect parent loss by a PPID change, not PPID 1. (t-340)
- Bump PROTOCOL_VERSION on any change to Request, Response or frames; the host refuses a worker of another version. (t-341)
- Drop fonts that no open slot uses from the replay state and from the worker with DropFonts. (t-341)
- Spawn a macOS worker only through sandbox-exec with worker.sb, and refuse it unless its Hello canary is denied. (spec:2026-10-17-render-worker-sandbox-design, t-912)
- Never hardlink a file into the worker's work dir; Seatbelt checks the path, so copy it. (spec:2026-10-17-render-worker-sandbox-design, t-912)
- Call with_soffice with the ReferenceWitness::Soffice program before a soffice worker's first request; its sandbox execs only that path. (spec:2026-10-17-render-worker-sandbox-design, t-931)

## Pitfalls

- Do not rely on the InProcess transport to stop runaway Typst; a timed-out worker thread cannot be killed. (plan:2026-09-29-compilation-bornee)
- Mark a widget whose probe timed out as broken, not unprobed, or it will be re-probed forever. (spec:2026-10-02-creation-widgets-design)
- Do not trust one-time pack validation; bound every render, since pack code may loop only for some params. (spec:2026-09-29-compilation-bornee-design)
- Do not rely on RLIMIT_AS or RLIMIT_DATA for memory limits; macOS ignores them, so poll worker memory instead. (spec:2026-09-29-compilation-bornee-design)
- Keep render worker stdout free of anything but protocol frames; the worker redirects stdout to stderr. (t-341)
- Keep allow default (LibreOffice needs the launchservicesd lookup) and the /tmp OSL_PIPE socket rule in worker.sb. (spec:2026-10-17-render-worker-sandbox-design, t-912)
- Pass every Seatbelt path parameter through storage::canonical; Seatbelt matches real paths, so /var never matches /private/var. (spec:2026-10-17-render-worker-sandbox-design, t-912)

## Settled decisions

- Never package the oku-slides-render-worker binary; it exists only for tests. (plan:2026-09-29-compilation-bornee)
- Keep widget fallback policy in the session; the render client flags only a slot's lone pack not exempted by is_never_blamed, never one beside a shipped base or essentiel. (plan:2026-10-18-packs-lot2-base, plan:2026-10-19-packs-lot3-essentiel, t-1053)
- Resolve the current shipped base and essentiel in-process (in_process_pack), so a deck on built-in packs alone never starts the render worker. (plan:2026-10-18-packs-lot2-base, plan:2026-10-19-packs-lot3-essentiel)
