# Changelog

All notable changes to Oku Slides are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [10.1.0] - 2026-11-06

### Added

- *(stt,session,agent,mcp,ui,app)* talk over Oku on macOS, locked-down agent window, sources budget (t-1469, t-1472, t-1491, t-1498, t-1500, t-1513)

### Documentation

- *(import)* write the generated champ comments in English (t-909)

### Other

- *(import)* calibrate the hi-fi import on an Apache/MIT pptx corpus (t-909, t-958)

## [10.0.1] - 2026-11-05

### Other

- bump Swatinem/rust-cache from 2.9.2 to 2.9.3 in the actions group
- *(render-worker)* widen three timing budgets for Windows runners (t-1528)

## [10.0.0] - 2026-11-05

### Added

- *(stt,i18n,session,mcp,agent,ui,app)* [**breaking**] hands-free voice control, lots 2 to 8 (t-996, t-997, t-998, t-999, t-1000, t-1001, t-1002)

### Fixed

- *(ci)* pin tauri-action back to v0 for draft release signing (t-1511)
- *(ci)* ignore RUSTSEC-2026-0429, zip 2.4.2 never runs (t-1512)
- *(chart)* automatic value scale like PowerPoint and LibreOffice (t-1493)
- *(chart)* inset the plot area by 2 % like LibreOffice (t-1514)

### Changed

- *(ui)* split api.ts by concept (t-1501)

### Documentation

- *(agents)* keep the root AGENTS.md within 130 lines (t-1511)

### Other

- *(render-worker)* pin worker.sb's network denial for remote hosts (t-1499)
- *(chart)* waterfall under 0.5 labels each tick once (t-1515)
- pin the Linux voice-check job to ubuntu-24.04 (t-1490)
- *(render-worker)* give the two loop tests a 2 s budget (t-1516)

## [9.0.1] - 2026-11-03

### Fixed

- *(apple-model,ci)* build the Swift bridge with Xcode 26 release runners (t-1509)

## [9.0.0] - 2026-11-02

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
- *(session,mcp,agent,platform,i18n,ui)* [**breaking**] lot C, session and agent follow-ups of the t-988 review (t-1004, t-1007, t-1003, t-1072, t-1073, t-956, t-1005, t-1326)
- *(platform,app,ui,ci)* resumable model downloads, sturdier updates, isolated signing (t-1052, t-976, t-979, t-985, t-986, t-984, t-977, t-978, t-1210)
- *(session,core,mcp,render,import,typst,ui)* [**breaking**] lot G, theme picker and add carousel (t-1085, t-1076, t-1077, t-962, t-963, t-964, t-1086, t-1352)
- *(import,chart,session,reference,ui)* [**breaking**] lot E, PPTX import follow-ups (t-1362, t-954, t-955, t-937, t-957, t-961, t-960, t-1295)
- *(core,session,mcp,local-agent,agent,ui,i18n)* duplicate and reorder items in a pack tab's pile (t-1249)
- *(a2ui,mcp,session,local-agent,i18n,ui)* the A2UI playbook, every component in its variants (t-1312)
- *(ui,session,a2ui,core,i18n)* pack editor layout with a slim list and side-by-side form (t-1392)
- *(core,session,agent,mcp,ui,typst)* agent guides in packs and themes (t-1267)
- *(ui,session,i18n)* pack tabs stack a thumbnail strip, the preview, then Settings and Code tabs (t-1419)
- *(ui,session)* pack guides for agents as tabs of Commun, Commun tile with an icon (t-1492)
- *(a2ui,session,i18n,ui)* pack editor follow-ups: field help, base labels, labelled copies (t-1415, t-1416, t-1417)
- *(import,chart,session,render,ui,i18n)* lot J, hi-fi chart axes and bars, lot E follow-ups (t-1384, t-1385)
- *(agent,local-agent,apple-model,mcp,platform,session,ui,i18n)* local agent fallback, diagnose tool, Apple engine and eval set (t-1021, t-1023, t-1024, t-1025, t-1009)
- *(agent,session,core,render,cli,ui,i18n)* agent works in its tab's own folder, received packs and themes install only on validation (t-1413, t-1414)
- *(session,mcp,agent,ui,i18n)* [**breaking**] hub inspector, data pane, popovers and receipt traces (t-1502, t-1503, t-1504, t-1505, t-1506, t-1507, t-1508)

### Fixed

- *(render,session)* a deck color the theme lacks draws in ink-muted with a deck warning instead of failing the render (t-1293)
- *(render-worker)* images no longer draw as fallback frames after a worker restart (t-1295)
- *(ui)* history slider keeps its width whatever the caption or theme (t-1375)
- *(ui)* history bar gives the slider 65% of the free width (t-1375)
- *(ui)* history bar gives the slider 75% of the free width (t-1375)
- *(session,ui,cli,app,i18n)* [**breaking**] pack tab follow-ups, batch F (t-1329, t-1237, t-1207, t-1239, t-1238, t-1251, t-1255)
- *(i18n,core,render,stt,session,a2ui,ui)* batch B hardening (t-959, t-981, t-965, t-969, t-1142, t-1206, t-1075, t-1090, t-1343)
- *(session,agent,cli,ui,i18n)* usage ledger follow-ups, batch D (t-1313, t-1310, t-1309, t-1307, t-1308, t-1314)
- *(build)* run just web and just playbook on Vite port 5174 (t-1391)
- *(ui)* renumber the pile order e2e spec to 310 (t-1249)
- *(deps)* bump zip 9.0.4 to 9.1.3 for GHSA-m665-v7pm-7grj (t-1401)
- *(session,mcp)* mark agent-made pile copies, drop marks an undo removes (t-1386, t-1402)
- *(session)* key agent widget marks to the source the agent wrote (t-1403)
- *(i18n,session,core,render,platform,stt,chart,ui)* lot H hardening of arguments, widget caps, resume budget and model sizes (t-1379, t-1381, t-1382, t-1383, t-1376, t-1440)
- *(app,session,ui)* guarded quit, per-window command capabilities, web-watch, pack tab leftovers (t-1377, t-980, t-1387, t-1378)
- *(ui,i18n)* isolate string arguments of t() with FSI/PDI, Rust localizer stays raw (t-1380, t-1442)
- *(session,mcp,ui,i18n)* agent guide follow-ups: approval scope, shipped guides, receipt lines (t-1420, t-1421, t-1422, t-1423, t-1424)
- *(ui)* send a ColorField picker value once it settles (t-213)
- *(ci)* set release-plz git_only on oku-slides-app only (t-1184)

### Performance

- *(core,session,ui)* lot G follow-ups, shared assets, live digits, one add opener (t-1388, t-1389, t-1390)

### Documentation

- *(session,ui)* AGENTS.md rules for the agent-surfaces fixture and ContextMenu (t-1252, t-1250)
- *(agents)* re-check main right before squashing a worktree branch (t-1296)

### Other

- *(render-worker)* Windows job for the render worker, parent check by pid and start time (t-1471, t-341)
- *(deps,core)* bump loro to 1.17.1, drop the commit before each list mov (t-893, t-223)
- *(deps,import)* bump office2pdf to 0.10 for zip 9 (t-899, t-888)
- bump the actions group with 5 updates

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
