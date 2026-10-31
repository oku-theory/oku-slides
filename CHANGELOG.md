# Changelog

All notable changes to Oku Slides are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [9.0.0] - 2026-10-31

### Added

- *(ui)* About entry with the version and changelog (t-1236)
- *(session,ui)* [**breaking**] deck-only packs open in their own tab linked to their deck (t-1200, t-1224, t-1226, t-1227, t-1228, t-1229, t-1230, t-1232, t-1234, t-1235)
- *(agent,platform)* OKU_SLIDES_DEBUG_ACP logs ACP messages both ways (t-1256)
- *(ui,session)* [**breaking**] pack and theme tabs, the interface (t-1157)
- *(agent)* choose the ACP agent model per preset (t-1257, t-1258, t-1259, t-1260, t-1261, t-1262, t-1263, t-1264)
- *(mcp,session,agent)* [**breaking**] each tab kind gets its own agent tools, instructions and context; a deck's agent opens pack and theme tabs (t-1191)
- *(ui)* a toolbar button opens the deck's history (t-1292)
- *(ui)* the history bar's Back button becomes Close the history (t-1294)
- *(ui,session,mcp)* [**breaking**] the stage grid shows only while dragging (t-1283)
- *(core,session,ui,typst)* # to ###### headings in editing markdown (t-1284)
- *(mcp,agent,session,local-agent,ui)* [**breaking**] lot 7 follow-ups (t-1152, t-1277, t-1278, t-1279, t-1280, t-1281, t-1282)
- *(ui,session,i18n)* t-1152 small UI batch (t-1254, t-1266, t-1252, t-1250, t-1265, t-1311)
- *(core,session,agent,mcp,cli,platform,ui)* [**breaking**] token usage ledger in each deck (t-1296)

### Fixed

- *(render,session)* a deck color the theme lacks draws in ink-muted with a deck warning instead of failing the render (t-1293)
- *(render-worker)* images no longer draw as fallback frames after a worker restart (t-1295)
- *(ui)* history slider keeps its width whatever the caption or theme (t-1375)
- *(ui)* history bar gives the slider 65% of the free width (t-1375)
- *(ui)* history bar gives the slider 75% of the free width (t-1375)

### Documentation

- *(session,ui)* AGENTS.md rules for the agent-surfaces fixture and ContextMenu (t-1252, t-1250)
- *(agents)* re-check main right before squashing a worktree branch (t-1296)

## [8.0.0] - 2026-10-28

### Added

- *(stt)* [**breaking**] dictation API through async-openai with Azure support (t-1233)

## [7.0.0] - 2026-10-27

### Added

- *(platform)* OKU_SLIDES_DEBUG_NETWORK turns on HTTP and TLS debug logs (t-1217)
- *(session,ui)* [**breaking**] themes open in their own tab from Packs du deck (t-1156, t-1218, t-1219, t-1220, t-1221, t-1222, t-1223)

### Fixed

- *(stt)* keep an unreadable dictation config.json aside (t-1216)

### Documentation

- *(aide)* document every environment variable in the help (t-1225)
- *(aide)* add OKU_SLIDES_DEBUG_NETWORK to the network help slide (t-1217, t-1225)

## [6.0.0] - 2026-10-26

### Added

- *(session,mcp,ui)* [**breaking**] library packs open in their own tab beside a preview deck (t-1155, t-1192, t-1193, t-1194, t-1195, t-1196, t-1197)

### Fixed

- *(updater)* log the bundle URL and endpoint of a failed download (t-1212)

### Documentation

- *(readme)* document the config.toml file (t-1215)

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
