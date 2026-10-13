# crates/oku-slides-cli — agent rules

## Invariants

- Never send Access-Control headers from a local server, and accept POST only as application/json. (ADR-0003)
- Do not add accepted Host names for wildcard bind addresses such as 0.0.0.0 or ::. (ADR-0003)
- Refuse with 403 any serve request whose Host is not loopback or the bound host; allow a missing Origin. (spec:2026-09-26-onglets-fenetres-design)
- Cap serve API request bodies at MAX_BODY with a 413, refusing at once if Content-Length is too large. (t-181, t-190)
- Serve a static file only if its URL path has only normal components: no .., root or drive prefix. (t-192)
- Give each SSE client a bounded channel and drop a client whose queue is full. (t-191)
- Check the pptx size against max_input_bytes before reading it into memory, and refuse oversized files. (plan:2026-10-10-import-pptx-lot2)
- Apply the whole CLI import to the bundle as a single history step. (plan:2026-10-10-import-pptx-lot2)
- Apply import ops in a single transaction through the normal validations, so the whole import undoes at once. (spec:2026-10-09-import-pptx-design)

## Pitfalls

- Never treat the origin guard as authentication; off loopback any network client that forges Host is served. (ADR-0003, t-193)

## Testing

- In CLI import tests, build pptx zips in memory and never launch LibreOffice or the app. (plan:2026-10-10-import-pptx-lot2)
- Run CLI tests through the oku_slides() helper so the data directory points at a temporary dir. (plan:2026-10-10-import-pptx-lot2)

## Settled decisions

- Do not count a loopback bind or a CORS origin allowlist as protection against other browser pages. (ADR-0003)
- Follow ADR-0003 for any change to the origin checks in guard.rs. (plan:2026-10-08-tri-backlog-nuit)
- Warn at startup when serve binds a non-loopback address, but do not refuse the local network. (plan:2026-10-08-tri-backlog-nuit)
- Write command results through Out, log with tracing, report errors with color-eyre; never use println! (t-12)
