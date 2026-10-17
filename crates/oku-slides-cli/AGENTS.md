# crates/oku-slides-cli — agent rules

## Invariants

- Never send Access-Control headers from a local server, and accept POST only as application/json. (ADR-0003)
- Do not add accepted Host names for wildcard bind addresses such as 0.0.0.0 or ::. (ADR-0003)
- Refuse with 403 any serve request whose Host is not loopback or the bound host; allow a missing Origin. (spec:2026-09-26-onglets-fenetres-design)
- Cap serve API request bodies at MAX_BODY with a 413, refusing at once if Content-Length is too large. (t-181, t-190)
- Serve a static file only if its URL path has only normal components: no .., root or drive prefix. (t-192)
- Give each SSE client a bounded channel and drop a client whose queue is full. (t-191)
- Write the import --report atomically through Storage, so an interrupted run never replaces the previous one. (t-817)

## Pitfalls

- Never treat the origin rules as authentication; off loopback only the launch token in guard::check stops a forged Host. (ADR-0003, t-193)

## Testing

- In CLI import tests, build pptx zips in memory and never launch LibreOffice or the app. (plan:2026-10-10-import-pptx-lot2)
- Run CLI tests through the oku_slides() helper so the data directory points at a temporary dir. (plan:2026-10-10-import-pptx-lot2)
- In reference tests, pass --fonts assets/fonts so Oku and the witness render with the app fonts. (t-692)

## Settled decisions

- Do not count a loopback bind or a CORS origin allowlist as protection against other browser pages. (ADR-0003)
- Follow ADR-0003 for any change to the origin checks in guard.rs. (plan:2026-10-08-tri-backlog-nuit)
- Warn at startup when serve binds a non-loopback address, but do not refuse the local network. (plan:2026-10-08-tri-backlog-nuit)
- Off loopback, require the per-launch token in guard::check, by Bearer header or HttpOnly SameSite=Strict cookie. (ADR-0003, t-193)
- Write command results through Out, log with tracing, report errors with color-eyre; never use println! (t-12)
