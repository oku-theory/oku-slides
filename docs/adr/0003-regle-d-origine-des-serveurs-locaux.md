# ADR-0003: Origin rule for local HTTP servers

- **Status:** accepted
- **Date:** 2026-10-09
- **Tasks:** t-89 (first guard), t-419 (switch to axum), t-189 (this
  decision), t-193 (warning, then launch token outside the loopback)
- **Code:** `crates/oku-slides-cli/src/guard.rs`

## Context

`oku-slides serve` opens an HTTP server on the machine (by default
`127.0.0.1`) for the browser UI: reading and editing the deck, the event
stream (SSE), the agent. Listening on the loopback is not enough protection:
any page open in the same browser can target `127.0.0.1`.

- **DNS rebinding**: a name controlled by a third party first resolves to its
  server, then to `127.0.0.1`; the page becomes "same origin" for the
  browser and reads the responses.
- **Requests from another page**: `<img>`, `<script>`, `fetch` in
  `no-cors` mode, forms… including from another `localhost` port
  (another development server). Without reading the response, they can
  modify the deck or take a slot of the event stream.

Other local servers will come (preview, MCP server over HTTP, development
tools): we need a single rule, not a guard reinvented every time.

## Decision

**Every local HTTP server of Oku Slides accepts only the requests that
come from its own UI, according to four rules checked on every request,
before any route.** The reference implementation is
`oku-slides-cli::guard::check`; a future server reuses it (or moves it into
a shared crate) instead of writing another one.

1. **Local `Host`**: the `Host` header is present exactly once, well
   formed, and names the loopback (`localhost`, `127.0.0.1`, `::1`) or the
   configured listen address. An "everywhere" address (`0.0.0.0`, `::`)
   adds no accepted name. This defeats DNS rebinding: the third party's name
   gives itself away in `Host`.
2. **`Sec-Fetch-Site` is `same-origin` or `none`** when present: the
   browser itself says that a request comes from another page
   (`cross-site`, `same-site`), including from another `localhost` port.
3. **`Origin`, when present, is the request's own origin**: http(s)
   scheme, same name and same port as `Host`. This holds for the UI served
   directly as well as through the Vite proxy, which keeps the browser's
   `Host`.
4. **A POST is `application/json`**: from another origin, the browser
   sends it only after a CORS preflight, and the server never grants it
   (no `Access-Control-*` header).

A rejected request gets `403` and the broken rule is logged.
On the loopback **there is no authentication** (no token, no cookie): the
rule protects against the browser, not against a local program.

**Outside the loopback (t-193)**, `check` also requires, after the four
rules, a random token drawn at each launch (32 bytes, URL-safe base64) and
printed by `serve` in the URL to open (`/?token=…`). It is accepted from an
`Authorization: Bearer` header or from a per-port cookie (`HttpOnly`,
`SameSite=Strict`) that a `GET` carrying a valid `?token=` sets before a
`303` redirect to the same URL without it. Comparison is constant-time on
SHA-256 digests; a missing or wrong token gets `401`.

## Consequences

**Positive**

- No configuration or secret to pass to the UI: the Vite server and the
  directly served UI work as they are.
- A single rule, tested (`guard_tests.rs`), for all future local
  servers.

**Negative (known limits)**

- **No authentication on the loopback.** A non-browser client (curl,
  script) sends whatever headers it wants: `Host: 127.0.0.1` without
  `Origin` or `Sec-Fetch-Site` passes the guard; only a program on the
  machine can reach it. Outside the loopback the launch token stops network
  clients that forge `Host`; `serve` still warns at startup (t-193). The
  token travels in clear over HTTP: anyone who can read the network traffic
  can reuse it until the server stops.
- A browser too old to send `Sec-Fetch-Site` only benefits from rules 1, 3
  and 4.

**Neutral**

- An "everywhere" address still adds no accepted `Host` name: a browser on
  another device that names the machine's address is refused (`403`). Making
  network listening an intended use (presenting on another device) remains
  a separate decision; the token is already in place for it.

## Rejected alternatives

- **A token on the loopback too:** complicates the Vite proxy, the e2e
  tests and every tool that talks to the server, for no gain against the
  browser that the four rules do not already give. Kept for exposed
  listening only (t-193).
- **"Open" CORS with an origin list:** CORS governs reading responses,
  not sending simple requests; it defeats neither DNS rebinding nor
  `no-cors` requests.
- **Relying on the listen address alone:** the loopback is reachable by
  every page in the browser.
