# Compilation Typst bornée dans un processus de travail — design

Date : 2026-09-29 · Tâche : t-327

## Objectif

Aucune compilation Typst ne peut plus geler ni faire tomber l'app. Tout ce
qui exécute du Typst venu d'un pack — rendu du deck, sonde du registre,
graphiques, export — tourne dans un **sous-processus persistant**, borné en
temps et en mémoire ; au-delà, l'hôte le tue, le relance et se replie sur
les widgets de base.

Programme « widgets portables », trois sous-projets, chacun avec sa spec :

0. **Compilation bornée** (cette spec, t-327).
1. Packs portables (t-328) : identité unique et version des packs, copie
   du pack embarquée dans le fichier, qui fait foi au rendu.
2. Création de widgets (t-329) : l'agent écrit et corrige `widgets.typ`,
   éditeur Typst avancé dans le mode modèles.

Le sous-projet 1 fait exécuter à l'app le Typst d'un PDF reçu ; le 2, du
Typst écrit par essais et erreurs. Ce sous-projet est leur prérequis.

Hors périmètre : une porte de confiance (« faire confiance à ce pack »),
l'isolation réseau et fichiers au niveau de l'OS (le `World` Typst ne sert
déjà que son système de fichiers virtuel), un pool de travailleurs.

## Constats

- `typst::compile` n'a pas d'API d'annulation (0.15.1). Ses gardes internes
  (itérations d'un `while`, profondeur d'appel, passes d'introspection) ne
  couvrent ni `for _ in range(1e12)` ni l'allocation : ni le temps ni la
  mémoire ne sont bornés.
- Le code d'un pack s'exécute à **chaque rendu** (le `render` de chaque
  widget), pas seulement dans la sonde : valider un pack une fois ne suffit
  pas, un widget peut ne boucler qu'avec certains paramètres.
- Un thread Rust ne se tue pas : l'abandonner laisse le CPU consommé
  jusqu'à la sortie de l'app, et la mémoire reste non bornée.
- `Compiled` enveloppe un `PagedDocument`, non sérialisable. `Layout` et
  ses boîtes le sont déjà (`serde`).
- macOS n'applique pas `RLIMIT_AS`/`RLIMIT_DATA` : la limite mémoire se
  mesure depuis l'hôte.

## Décisions

- **Un sous-processus par session**, persistant pour garder le cache
  incrémental (`comemo`) entre deux rendus ; une requête en vol à la fois,
  comme aujourd'hui sous la session.
- **Ré-exécution de soi-même** : `std::env::current_exe()` lancé avec
  `--render-worker`. Pas de binaire à part : l'hôte et le travailleur sont
  toujours de la même version.
- **Chien de garde dans l'hôte** : délai par requête, relevé de la mémoire
  résidente (RSS) du travailleur toutes les 100 ms ; au-delà, `kill`. Même
  mécanisme sur les trois OS.
- **Le registre de base reste dans l'hôte**, chargé une fois par processus
  (`registry.rs`) : code embarqué, de confiance ; la session en a besoin
  pour valider sans aller-retour. Seuls les packs passent par le
  travailleur.
- **`Renderer` reste le moteur**, public et inchangé dans
  `oku-slides-render` ; le travailleur l'héberge.

## Architecture

Nouveau crate `oku-slides-render-worker` (dépend de `oku-slides-render`,
`oku-slides-core`, `oku-slides-platform`, `oku-slides-i18n`) :

- `frame` : trames à longueur préfixée.
- `serve(reader, writer, fonts)` : boucle du travailleur. Lit les requêtes,
  les exécute sur un `Renderer`, répond. Garde le dernier `Compiled` sous un
  numéro de **génération** (`u64`, croissant) et les assets reçus par
  identifiant.
- `RenderClient` : côté hôte. Même surface que `Renderer` : `set_language`,
  `set_widgets`, `set_deck`, `render`, `registry`, `chart_palette`.
  `render` renvoie un `RemoteCompiled`.
- `RemoteCompiled` : poignée d'une génération. Locaux : `page_count`,
  `page_hash`, `layout()`. Par requête : `svg`, `png`, `pdf`.
- `Limits { timeout, memory }` : lus dans la configuration.

Le point d'entrée des deux binaires (CLI `oku-slides`, app Tauri) teste
`--render-worker` en tête de `main`, avant toute autre initialisation, et
appelle `oku_slides_render_worker::run_from_args()` qui ne rend pas la main.

## Protocole

Trame : longueur de l'en-tête (`u32` big-endian), en-tête JSON
(`serde_json`), longueur de la charge (`u32`), charge binaire brute
(éventuellement vide). PNG, PDF, assets et polices voyagent tels quels. Une
longueur au-delà de `render.max_frame_mib` est refusée (`Frame`), des deux
côtés.

| Requête | Charge | Réponse |
|---|---|---|
| `SetLanguage { locale }` | — | `Ok` |
| `PutFont { name }` | octets | `Ok` |
| `PutAsset { id }` | octets | `Ok` |
| `SetWidgets { pack: Option<{ id, source }> }` | — | `Registry { registry }` |
| `Render { deck }` | — | `Compiled { generation, page_count, page_hashes, layout, chart_palette }` |
| `Svg { generation, page }` | — | `Bytes` (UTF-8) |
| `Png { generation, page, pixel_per_pt }` | — | `Bytes` |
| `Pdf { generation }` | — | `Bytes` |

Toute requête peut répondre `Error { kind, message, diagnostics }`, qui
transporte un `RenderError` ou un `RegistryError` sérialisé. `Svg`, `Png`
et `Pdf` d'une génération qui n'est plus la dernière répondent `Stale`.

L'hôte n'envoie un asset que si le travailleur ne l'a pas (ensemble des ids
envoyés tenu par le client, vidé à la relance). `Registries` (mémoïsation
par empreinte) vit dans le travailleur.

## Configuration

`config.default.toml` de `oku-slides-platform` :

```toml
[render]
# Durée maximale d'une compilation ou d'un export Typst (ms) ; au-delà, le
# processus de rendu est relancé et le deck se rend avec les widgets de base.
timeout_ms = 10000
# Mémoire maximale du processus de rendu (Mio).
memory_mib = 2048
# Taille maximale d'un message échangé avec le processus de rendu (Mio).
max_frame_mib = 512
```

Surchargeable par `OKU_SLIDES_RENDER__TIMEOUT_MS`, etc.

## Échecs et repli

`RenderErrorKind` gagne `Timeout`, `OutOfMemory` et `WorkerLost` (fin
inattendue du travailleur, y compris une panique de Typst, qui aujourd'hui
ferait tomber l'app), et `Frame` (message illisible ou trop grand). Snafu
avec backtrace, messages en minuscules, localisés via Fluent dans les 20
langues.

Sur `Timeout`, `OutOfMemory` ou `WorkerLost` :

1. **Pendant `SetWidgets` ou un `Render` avec un pack** : l'empreinte
   (sha-256 de la source du pack) est marquée **fautive pour la session**.
   Le client relance le travailleur, rejoue l'état (`SetLanguage`, polices,
   `SetWidgets { pack: None }`, `Render`) et la session affiche un
   diagnostic qui nomme le pack et la cause. Le deck se rend avec les
   widgets de base : ses éléments de widgets propres au pack montrent
   l'encart « élément inconnu », leurs paramètres sont conservés.
2. **Avec les widgets de base** (deck démesuré) : l'erreur remonte telle
   quelle, comme une erreur de compilation aujourd'hui ; le deck reste
   éditable, le dernier rendu réussi reste affiché. Le travailleur est
   relancé pour la requête suivante.
3. **Un pack fautif n'est pas retenté** dans la session. Le modifier change
   son empreinte : il est retenté.

**Poignée périmée.** Après un nouveau rendu ou une relance, une
`RemoteCompiled` ancienne reçoit `Stale` ; la session re-rend une fois et
ne remonte l'erreur à l'UI que si ce re-rendu échoue.

## Consommateurs

- **Session** : `new_renderer` construit un `RenderClient`.
  `deck_widgets.rs` appelle `set_widgets` du client ; le registre du pack
  revient du travailleur, fusionné avec la base comme aujourd'hui.
  L'arrêt de la session ferme le travailleur (fermeture de stdin, puis
  `kill` après un court délai).
- **CLI** : `render` et `export` passent par le client ; `bench` garde le
  `Renderer` en mémoire (il mesure le moteur, pas l'IPC).
- **MCP** : passe par la session, rien à changer.
- **Tauri** : drapeau en tête de `main.rs`.
- Les ARCHITECTURE.md de `render`, `session` et du nouveau crate sont mis à
  jour.

## Tests

Dans l'ordre de construction, en TDD :

1. **Trames** : aller-retour en-tête et charge ; trame tronquée ;
   longueur au-delà de `max_frame_mib` refusée.
2. **`serve` en mémoire** (tubes, sans sous-processus) : `Render` puis `Svg`
   de la même génération ; `Stale` après un second `Render` ; `PutAsset`
   puis un deck qui l'utilise.
3. **`RenderClient` avec un vrai sous-processus** (binaire de test lancé
   avec `--render-worker`) :
   - rendu identique octet pour octet au rendu en mémoire pour
     `fixtures/roadmap` (SVG de chaque page, `Layout`) ;
   - pack qui boucle (`for _ in range(1000000000000) {}` dans un widget) :
     `Timeout`, repli sur la base, empreinte fautive, second rendu sans
     nouvelle tentative ;
   - pack qui alloue sans fin, sous un `memory_mib` de test bas :
     `OutOfMemory` ;
   - travailleur tué de l'extérieur : `WorkerLost`, relance, rejeu, rendu
     identique.
4. **Régression** (`tests/regressions/`) : un pack qui boucle ne gèle plus
   la session — `Session::render` rend la main en moins de `timeout_ms`
   plus une marge.
5. **i18n** : clés des nouvelles erreurs dans les 20 langues.
