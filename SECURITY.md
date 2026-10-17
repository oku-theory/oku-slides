# Security policy

## Reporting a vulnerability

Please report vulnerabilities **privately**, never in a public issue, pull request or
discussion.

Use GitHub's private reporting:
[**Report a vulnerability**](https://github.com/oku-theory/oku-slides/security/advisories/new)
(Security tab ▸ Advisories). Include:

- the affected version or commit, and the platform (macOS, Windows, Linux);
- what an attacker gains, and what they need (a file the victim opens, a web page open
  in the same browser, local access…);
- steps or a minimal file that reproduces it. Keep proofs of concept harmless: write to
  a temporary directory, not to real user data.

## What to expect

- An acknowledgement within **7 days**.
- An assessment (accepted, needs more information, or out of scope) within **30 days**.
- A fix in a release within **90 days** of the report for accepted issues, sooner for
  severe ones. We agree the disclosure date with you and publish a GitHub Security
  Advisory, with credit unless you prefer otherwise.

## Supported versions

Only the **latest release** receives security fixes. Please check that the issue still
reproduces on it, or on `main`.

## Scope

In scope, with the trust boundaries we care most about:

- **Files from others**: imported `.pptx` files and `.oku-slides.pdf` documents
  (embedded deck sources, assets, fonts, template sets, themes) are untrusted input.
  Path traversal, decompression bombs, crashes of the app or the render worker, or code
  execution from such a file are in scope.
- **Local servers**: the UI server (`oku-slides serve`) and the agent's MCP tool server
  listen on `127.0.0.1` and must only answer their own UI or tab
  ([ADR-0003](docs/adr/0003-regle-d-origine-des-serveurs-locaux.md)). A web page,
  another local process or another tab reading or editing a deck is in scope.
- **Rendering**: Typst rendering runs in a bounded worker process; escaping its limits
  or reading files outside the deck is in scope.
- **The desktop app** (Tauri host) and its release packages.

Out of scope:

- Vulnerabilities in a dependency that Oku Slides never reaches. Report them upstream;
  tell us if Oku Slides does reach them. Dependencies are scanned daily
  (`.github/workflows/audit.yml`).
- What the configured agent (Claude, Codex, Gemini…) does with the permissions you give
  it, and the agent's own security.
- Attacks that require admin rights or physical access to an unlocked machine.
