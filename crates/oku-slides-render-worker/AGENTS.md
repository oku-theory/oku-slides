# crates/oku-slides-render-worker — agent rules

Read `ARCHITECTURE.md` first for what this code is. This file is how to work on it.

## Invariants

- Drop only the widget files that fail to compile, with their diagnostics, and keep the pack's other valid widgets. (plan:2026-10-02-creation-widgets)
- Never take the link lock in a Drop; queue Release and CloseSlot in the separate deferred mutex. (plan:2026-09-29-compilation-bornee)
- Refuse to render a slot whose pack is faulty with pack_disabled; never silently render with core widgets only. (plan:2026-09-29-compilation-bornee)
- Never blame a pack for failures during widget editing or preview; turn a probe timeout into a file diagnostic. (plan:2026-10-02-creation-widgets)
- Do not retry a faulty pack in the same session until its source fingerprint changes. (spec:2026-09-29-compilation-bornee-design)
- Apply the watchdog memory limit only during a request; detect parent loss by a PPID change, not PPID 1, on Windows by pid and start time. (t-340, t-1471)
- Bump PROTOCOL_VERSION on any change to Request, Response or frames; the host refuses a worker of another version. (t-341)
- Refuse a Render that lists an asset the worker lacks with MissingAssets; the host resends once, never draws its missing frame. (t-1295)
- Clear every slot's last_assets on reconnect, as the replayed worker has none, so host and worker evict alike. (t-1295)
- Drop fonts that no open slot uses from the replay state and from the worker with DropFonts. (t-341)
- Spawn a macOS worker only through sandbox-exec with worker.sb, and refuse it unless its Hello canary is denied. (spec:2026-10-17-render-worker-sandbox-design, t-912)
- Never hardlink a file into the worker's work dir; Seatbelt checks the path, so copy it. (spec:2026-10-17-render-worker-sandbox-design, t-912)
- Call with_soffice with the ReferenceWitness::Soffice program before a soffice worker's first request; its sandbox execs only that path. (spec:2026-10-17-render-worker-sandbox-design, t-931)
- Extract documents only in a child worker built for it with the `[knowledge]` bounds, never in a deck's render worker or in process (plan:2026-11-25-deck-knowledge-okf-lot4-documents, t-1755)
- Parse Parquet bytes only in the worker; the host never decodes a base. (t-1690)
- Read a base only by a path inside the Hello bases folder, opened without following links. (t-1690)
- Write a compaction only directly inside the bases folder or the worker's work dir, and remove its output on any refusal. (t-1690)
- Send a cached data whose cells alone changed as PatchData, so a one-cell edit stays under 64 KiB; a patch for an unknown id is ignored and the next Render's MissingData resends it whole. (t-1690)

## Pitfalls

- Do not rely on the InProcess transport to stop runaway Typst; a timed-out worker thread cannot be killed. (plan:2026-09-29-compilation-bornee)
- Mark a widget whose probe timed out as broken, not unprobed, or it will be re-probed forever. (spec:2026-10-02-creation-widgets-design)
- Do not trust one-time pack validation; bound every render, since pack code may loop only for some params. (spec:2026-09-29-compilation-bornee-design)
- Do not rely on RLIMIT_AS or RLIMIT_DATA for memory limits; macOS ignores them, so poll worker memory instead. (spec:2026-09-29-compilation-bornee-design)
- Keep render worker stdout free of anything but protocol frames; the worker redirects stdout to stderr. (t-341)
- Keep allow default (LibreOffice needs the launchservicesd lookup) and the /tmp OSL_PIPE socket rule in worker.sb. (spec:2026-10-17-render-worker-sandbox-design, t-912)
- Keep (deny network*) in worker.sb: the received dialog's offline line relies on it, and sandbox tests pin it. (t-1499)
- Expect the extraction test hooks (OKU_SLIDES_TEST_EXTRACT_SLEEP_MS, OKU_SLIDES_TEST_EXTRACT_ALLOC_MIB) in every debug worker; they compile out of release builds, so never rely on them outside tests. (t-1760)
- Read a Windows process identity with a fresh sysinfo System; a cached entry keeps the handle and a stale start time. (t-1471)
- Pass every Seatbelt path parameter through storage::canonical; Seatbelt matches real paths, so /var never matches /private/var. (spec:2026-10-17-render-worker-sandbox-design, t-912)

## Settled decisions

- Never package the oku-slides-render-worker binary; it exists only for tests. (plan:2026-09-29-compilation-bornee)
- Keep widget fallback policy in the session; the render client flags only a slot's lone pack not exempted by is_never_blamed, never one beside a shipped base or essentiel. (plan:2026-10-18-packs-lot2-base, plan:2026-10-19-packs-lot3-essentiel, t-1053)
- Resolve the current shipped base and essentiel in-process (in_process_pack), so a deck on built-in packs alone never starts the render worker. (plan:2026-10-18-packs-lot2-base, plan:2026-10-19-packs-lot3-essentiel)
