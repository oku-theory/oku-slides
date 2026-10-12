---
name: run-oku-slides
description: Lancer, piloter et capturer Oku Slides (serveur web + UI) avec le faux agent ACP. Use when asked to run, start, launch or screenshot Oku Slides, to check a UI change in the real app, or to drive the agent thread (send a prompt, expand the timeline, see tool calls).
---

Oku Slides se lance en serveur web (`oku-slides serve`) avec l'UI construite et des données
isolées, via `.claude/skills/run-oku-slides/serve.sh` ; on le pilote avec
`.claude/skills/run-oku-slides/drive.cjs` (Chromium headless, Playwright de `ui/`), qui
envoie une consigne au faux agent, déplie le fil, capture la page et imprime le texte du fil.

Chemins relatifs à la racine du dépôt. Vérifié sur macOS (arm64).

## Build

Toujours juste avant de lancer : `target/` est partagé entre sessions (worktrees), un
binaire laissé par une autre peut venir d'une autre extraction. Le `touch` force la recompilation.

```bash
(cd ui && bun run build)
touch crates/*/src/lib.rs && cargo build -q -p oku-slides-cli -p oku-slides-agent --bin oku-slides --bin oku-slides-fake-agent
```

Playwright et son Chromium sont ceux des e2e (`cd ui && bunx playwright install chromium` si absent).

## Run (agent path)

Lancer (port 4991 par défaut ; refuse un port pris), attendre qu'il réponde :

```bash
(.claude/skills/run-oku-slides/serve.sh 4991 > /tmp/oku-slides-serve.log 2>&1 &)
for _ in $(seq 1 50); do curl -sf -o /dev/null http://127.0.0.1:4991/ && break; sleep 0.3; done
```

Piloter : consigne à l'agent, dépliage complet du dernier tour, capture :

```bash
node .claude/skills/run-oku-slides/drive.cjs --prompt "Corrige les trois slides showcase." --expand --shot /tmp/oku-slides-shots/tour.png
```

| option | effet |
|---|---|
| `--url U` | l'app (défaut `http://127.0.0.1:4991/`) |
| `--prompt TEXTE` | consigne à l'onglet `t-1` (`POST /api/agent`) ; attend la fin du tour |
| `--expand` | déplie la timeline du dernier tour, chaque étape, chaque appel d'outil |
| `--shot F` | capture pleine page (défaut `/tmp/oku-slides-shots/page.png`) |

Sortie : chemin de la capture, texte du fil, erreurs de la console (code 1 s'il y en a).
Regarder la capture : le texte seul ne montre ni couleurs ni alignements.

Arrêter :

```bash
lsof -ti:4991 -sTCP:LISTEN | xargs kill
```

### Le faux agent

`serve.sh [port] [scénario] [deck]` : le scénario par défaut est
`.claude/skills/run-oku-slides/agent-details.json` (plan, 5 appels `widgets` dont un en
échec, une longue note, `apply` puis `slide` sans arguments). Format complet en tête de
`crates/oku-slides-agent/src/bin/fake_agent.rs` ; une étape d'outil accepte `input`
(`raw_input`), `output` (`raw_output`), `status: "failed"` et `before` (texte envoyé avant
l'appel : il devient une note). Le n-ième prompt joue le tour n (modulo) : un second
`--prompt` rejoue le même tour.

## Run (human path)

Même lanceur, puis ouvrir `http://127.0.0.1:4991/` dans un navigateur. `just web` (HMR sur
:5173) utilise les données réelles de l'utilisateur et son agent configuré : pas pour un essai.

## Test

```bash
(cd ui && bun test)
touch crates/*/src/lib.rs && cargo nextest run -p oku-slides-a2ui -p oku-slides-session -p oku-slides-agent -p oku-slides-i18n -p oku-slides-mcp
```

## Gotchas

- **Binaires d'une autre session** : sans `touch` + build juste avant, l'app peut tourner
  avec le code d'une autre extraction (vu : noms `mcp__…` non raccourcis, aucun détail
  d'outil, alors que le dépôt avait le bon code).
- **Données isolées obligatoires** : `serve.sh` exporte `OKU_SLIDES_PATHS__DATA/DECKS` vers
  un `mktemp` ; sans ça l'app lit et écrit les réglages, onglets et conversations de
  l'utilisateur.
- **Ports** : `just e2e` prend 4872 et d'autres sessions tournent en parallèle ; garder 4991
  ou un autre port libre.
- **Pas de `timeout` sur macOS** : attendre le port avec la boucle `for … curl` ci-dessus.
- **Dépliage pas à pas** : chaque dépliage décale les lignes repliées suivantes ; le pilote
  clique la première ligne repliée tant qu'il en reste (un `locator.all()` expire).
- **Fin du tour** : le résumé de la timeline ne montre la durée (`· N s`) qu'une fois le tour
  fini ; c'est ce qu'attend `--prompt`.
- **`innerText` du fil** répète consigne et réponse : la région `aria-live` (lecteurs
  d'écran) les contient aussi.

## Troubleshooting

- **`no ToolCallDetail in the root` (ou autre symbole introuvable) au build** : cache
  incrémental d'une autre session dans `target/`. Relancer avec `touch crates/*/src/lib.rs`
  dans la même commande.
- **`ENOSPC` / compilation qui échoue sans erreur Rust** : disque plein, `target/` partagé
  peut dépasser 300 Gio ; `cargo clean` (demander d'abord à l'utilisateur).
- **`locator.click: Timeout … nth(2)`** : version du pilote qui itérait sur `locator.all()` ;
  utiliser la boucle « première ligne repliée ».
