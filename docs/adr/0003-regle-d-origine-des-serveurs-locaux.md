# ADR-0003: Origin rule for local HTTP servers

- **Status:** accepted
- **Date:** 2026-10-09
- **Tasks:** t-89 (first guard), t-419 (switch to axum), t-189 (this
  decision), t-193 (warning outside the loopback)
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
**There is no authentication** (no token, no cookie): the rule protects
against the browser, not against a local program.

## Consequences

**Positive**

- No configuration or secret to pass to the UI: the Vite server and the
  directly served UI work as they are.
- A single rule, tested (`guard_tests.rs`), for all future local
  servers.

**Negative (known limits)**

- **No authentication.** A non-browser client (curl, script) sends
  whatever headers it wants: `Host: 127.0.0.1` without `Origin` or
  `Sec-Fetch-Site` passes the guard. As long as the server listens on the
  loopback, only a program on the machine can reach it; if it listens
  **outside the loopback** (`[serve] host = "0.0.0.0"` or a network
  address), **any network client that forges `Host` is served**. `serve`
  warns about it at startup (t-193).
- A browser too old to send `Sec-Fetch-Site` only benefits from rules 1, 3
  and 4.

**Neutral**

- The day network listening becomes an intended use (presenting on another
  device), authentication will have to be added (token in the URL or
  cookie): a decision to be made separately, with this rule staying in
  place on top.

## Rejected alternatives

- **Secret token in the URL or a cookie right now:** also protects against
  the network, but complicates the Vite proxy, the e2e tests and every tool
  that talks to the server, for a server that listens on the loopback by
  default. Postponed as long as network listening is not an intended use.
- **"Open" CORS with an origin list:** CORS governs reading responses,
  not sending simple requests; it defeats neither DNS rebinding nor
  `no-cors` requests.
- **Relying on the listen address alone:** the loopback is reachable by
  every page in the browser.
