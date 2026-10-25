# Changelog

All notable changes to Oku Slides are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [5.2.0] - 2026-10-25

### Added

- *(stt)* query parameters and custom headers for the dictation API (t-1213)

## [5.1.1] - 2026-10-25

### Fixed

- *(ci)* pass secrets to the update mirror called by Release (t-1209)
- *(ci)* refresh Cargo.lock in release-prepare.sh (t-1209)
- *(stt)* trust the system certificate store for outgoing TLS (t-1211)

## [5.1.0] - 2026-10-25

### Added

- *(updater)* serve updates from updates.okutheory.com, mirror first (t-1209)

### Documentation

- *(adr)* name the update mirror bucket oku-updates (t-1209)

## [5.0.0] - 2026-10-25

### Fixed

- *(platform)* [**breaking**] lint hardening follow-ups, pinned toolchain and four panic or validation fixes (t-1201, t-1202, t-1203, t-1204, t-1205)

### Other

- *(lints)* enable clippy pedantic, missing_docs and rust_2018_idioms workspace-wide (t-1186)

## [4.0.0] - 2026-10-25

### Added

- *(ui)* « Rechercher des mises à jour… » dans le menu principal (t-1170)
- *(agent)* coller une image dans le chat de l'agent — validée au collage, montrée à l'agent en bloc image, ajoutée au deck quand une op la cite (t-1161, t-1162, t-1163, t-1164, t-1165, t-1166, t-1167, t-1168, t-1169)
- *(packs)* brouillons de la bibliothèque figés à l'adoption par un deck, spec et plan des onglets pack et thème (t-1152, t-1153, t-1158, t-1159, t-1160)
- *(ui)* copier un élément dans une fenêtre et le coller dans une autre (t-1182)
- *(ui)* [**breaking**] réglages de la dictée et de l'agent en boîte A2UI du dock, pilotés par MCP, API réservée à l'utilisateur (t-1144, t-1146, t-1147, t-1148, t-1149, t-1150, t-1151)
- *(agent)* [**breaking**] une conversation de l'agent par onglet, AgentScope supprimé (t-1152, t-1154, t-1173, t-1174, t-1175, t-1176, t-1177)

### Fixed

- *(session)* l'asset inconnu dit à l'utilisateur, l'agent garde son message (t-1171)
- *(session)* coller dans un autre onglet un élément copié apporte les octets de ses images (t-1178)

### Documentation

- *(agents)* le premier build d'un worktree part à froid (t-1181)

### Other

- *(deps)* happy-dom 21.0.4, avis zod ignoré en attendant zod 4 (t-1172)
- *(deps)* @a2ui/react et @a2ui/web_core 0.14.0 (t-1180)
- *(harness)* [**breaking**] adopt Conventional Commits in English and enforce them (t-1183)
- *(release)* [**breaking**] release-plz release PRs and tags, scheduled scan with machete, metrics and UI security gates (t-1184, t-1185, t-1198)
- *(audit)* drop the CodeQL job, the repo's default setup already runs CodeQL (t-1198)
