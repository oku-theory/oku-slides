# Oku Slides

`echo "une idée" > deck` : ce que vous dites devient des slides. Oku Slides crée et
modifie des présentations à la voix ; le document est un PDF (`.oku-slides.pdf`) qui
contient ses propres sources : n'importe quel lecteur l'affiche, Oku Slides le rouvre
pour continuer l'édition.

Dans le code : crates `oku-slides-*`, outil `oku-slides`, bibliothèque Typst
`oku-slides`.

Plan d'implémentation : [Oku Slides — plan d'implémentation](https://claude.ai/code/artifact/fca9973c-7aff-454e-b40b-9a900dedaf57).

## Contenu du dépôt

| Chemin | Rôle |
| --- | --- |
| `crates/oku-slides-core` | Modèle du deck, opérations réversibles, historique, validation, zip des sources |
| `crates/oku-slides-render` | Typst embarqué : SVG par slide, PDF, boîtes des éléments |
| `crates/oku-slides-pdf` | Blob des sources dans le PDF (`/PieceInfo`) : écriture, lecture, retrait |
| `crates/oku-slides-session` | Session d'édition (deck ouvert, historique, rendu, fichier), partagée par l'app et le serveur de dev |
| `crates/oku-slides-stt` | Dictée : micro, détection de la voix, Whisper (CTranslate2) en local, modèles téléchargés à la demande |
| `crates/oku-slides-a2ui` | Surfaces A2UI v0.9 (catalogue de composants), partagées par le panneau et l'agent |
| `crates/oku-slides-mcp` | Outils MCP de l'agent (HTTP local, jeton) : lecture, `apply`, surfaces, confirmations |
| `crates/oku-slides-agent` | Agent vocal : client ACP (agent en sous-processus), tours, pile pour l'app et le serveur ; faux agent de test |
| `crates/oku-slides-cli` | Outil `oku-slides` : rendu, enregistrement, relecture, mesures, serveur de dev |
| `src-tauri` | Application de bureau Tauri 2 |
| `ui` | Interface React + TypeScript (design system Papier) |
| `assets/fonts` | Polices livrées (Newsreader, Instrument Sans, Inter) |
| `typst/oku-slides` | Bibliothèque Typst de slides (structure, jetons par défaut et thèmes intégrés `papier` et `pro`, en JSON) |
| `fixtures/` | Decks d'exemple |
| `docs/aide/` | Aide de l'app : un deck JSON par langue (fr, en), compilé dans l'app, ouvert en lecture seule (⌘?) |

## Outil en ligne de commande

```sh
just oku-slides check fixtures/roadmap.json                       # valider un deck
just oku-slides render fixtures/roadmap.json --out out --theme pro # SVG + layout.json ; thème par nom
just save fixtures/roadmap.json deck.oku-slides.pdf                # enregistrer en .oku-slides.pdf
just oku-slides open deck.oku-slides.pdf --out extrait/                # deck.json + assets
just oku-slides strip deck.oku-slides.pdf copie.pdf                    # copie sans sources
just bench                                                     # latences
just oku-slides schema                                             # schéma JSON des opérations
```

`--fonts <dossier>` ajoute des polices (TTF/OTF) ; sans elles, Typst utilise
ses polices intégrées (Libertinus Serif, New Computer Modern, DejaVu Sans Mono).

## Lancer l'application

Tout se lance depuis la racine du dépôt avec [just](https://just.systems).
Prérequis : Rust, [Bun](https://bun.sh), just, cmake (pour CTranslate2), la CLI Tauri
(`cargo install tauri-cli --version "^2"`) et [Git LFS](https://git-lfs.com) pour les
polices de `assets/fonts` (`git lfs install` avant de cloner, sinon `git lfs pull`).
`just doctor` vérifie tout.

```sh
just dev      # app de bureau en développement (installe les dépendances de l'interface)
just build    # paquet .app / .dmg
just web      # interface dans le navigateur avec rechargement à chaud (localhost:5173)
just serve    # interface compilée dans le navigateur (127.0.0.1:4870)
just test     # tests Rust
just check    # format, clippy, types TypeScript
just          # toutes les commandes
```

### Édition à la main

| Geste | Effet |
| --- | --- |
| Clic | Sélectionne un élément ou un champ (titre, surtitre…) |
| Glisser, poignées | Déplace ou redimensionne un élément libre |
| Double-clic ou Entrée | Modifie le texte (une ligne par paragraphe, `- ` pour une puce) |
| Flèches (Maj : ×10) | Déplace l'élément libre sélectionné de 1 pt |
| Suppr | Supprime l'élément sélectionné |
| ⌘Z, ⇧⌘Z | Annuler, rétablir (un geste = une annulation) |
| ⌘S, ⇧⌘S, ⌘O | Enregistrer, enregistrer sous, ouvrir |
| ⌘? | Aide de l'app (deck en lecture seule, `docs/aide/`) |


### Grille, modèles, ordre des slides

- **Grille** : chaque slide a une grille (12 × 6 par défaut). Son nombre de
  colonnes et de lignes se règle dans le modèle (post-it « Grille » en mode
  Modèles) ; les slides créées depuis ce modèle la reprennent. L'accrochage
  est automatique et ne se désactive pas : un élément occupe un bloc de
  cases, se déplace de case en case (souris ou flèches), et chaque coin
  s'accroche aux nœuds. Le bouton à points de la barre du haut (ou ⌘')
  affiche ou masque la grille, dans les slides comme dans les modèles : un
  petit point sur chaque nœud ; pendant un glisser, elle s'affiche de toute
  façon. La géométrie est calculée à l'identique par `ui/src/grid.ts`
  (édition) et `grid-frame` dans `data.typ` (rendu).
- **Ensembles de modèles** : les modèles de « Nouvelle slide » sont groupés en
  ensembles (`templates/<id>/pack.json` intégrés : Essentiel, Grilles ; ceux de
  l'utilisateur dans le dossier de données de l'app). Un ensemble ne dépend
  pas du thème : le deck choisit son ensemble et son thème séparément.
- **Édition des modèles** (bouton « Modèles ») : chaque modèle devient une
  slide qu'on modifie comme une autre (grille, éléments, textes), avec son nom
  et sa description. Un ensemble intégré s'enregistre comme copie ; le deck
  est mis de côté et retrouvé avec « Terminer ».
- **Thèmes** : un thème est un jeu de jetons de design (couleurs, polices,
  tailles, marges, composants) versionné comme les ensembles de modèles :
  Papier et Pro sont intégrés (`typst/oku-slides/themes/*.json`), ceux de
  l'utilisateur vont dans le dossier `themes/` des données de l'app. Le menu ⋯
  choisit le thème du deck ; le deck embarque sa version, et un fichier reçu
  garde le sien (« Thème du fichier »). En mode modèles, l'onglet « Thème »
  règle les jetons un par un (filtre, état de chaque jeton, ↺) : la scène suit
  chaque changement, et « Terminer » enregistre le brouillon, en copie
  (« Papier (copie) ») pour un thème intégré.
- **Ordre des slides** : glisser une miniature dans la pile (une ligne montre
  où elle se posera), ou ⌥↑ / ⌥↓.

### Dictée

Tout se passe sur la machine : Whisper sur CTranslate2 (le moteur de
faster-whisper, via [ct2rs](https://crates.io/crates/ct2rs)), poids quantifiés
en int8, calcul par Accelerate sur Mac. Au premier appui sur le micro, le
modèle choisi est téléchargé une fois, puis l'écoute démarre toute seule.

| Modèle | Taille | Usage |
| --- | --- | --- |
| Large v3 Turbo (par défaut) | 818 Mo | Rapide et précis, multilingue |
| Large v3 | 3,1 Go | Le plus précis, multilingue, plus lent |
| Base | 148 Mo | Léger, pour les petites machines |

- **Détection de la voix** ([earshot](https://crates.io/crates/earshot)) :
  seuls les passages parlés sont transcrits ; l'écoute s'arrête seule après un
  silence (1,5 s par défaut, réglable, désactivable), ou au bout de 10 s si
  rien n'est dit.
- **Phrase par phrase** : à chaque pause, la phrase qui vient de finir est
  transcrite en qualité finale, avec le texte déjà dicté et le vocabulaire du
  deck en contexte. L'aperçu ne retranscrit que la phrase en cours. À l'arrêt,
  il ne reste que la dernière phrase à traiter.
- **Langue** : imposée, ou détectée sur la première phrase puis gardée.

Les modèles vont dans le dossier de données de l'app
(`~/Library/Application Support/me.dartiguenave.oku-slides/models` sur Mac) ;
`OKU_SLIDES_PATHS__MODELS` le remplace (voir [Réglages](#réglages)). Les anciens fichiers `ggml-*.bin` (whisper.cpp)
peuvent y être supprimés.

| Geste | Effet |
| --- | --- |
| ⌘D ou le micro | Commencer, puis terminer la dictée |
| ⌘⇧D | Dicter dans le brouillon, même pendant l'édition d'un texte |
| Échap | Abandonner l'écoute ou le téléchargement |

Pendant l'édition d'un texte, ⌘D écrit au curseur. Ailleurs, la dictée va dans
un **brouillon** du panneau de droite : on la relit, la retouche ou la complète
(chaque dictée s'y ajoute), puis on choisit :

- **Insérer** (⌘⇧↩) l'écrit tel quel dans la zone sélectionnée (un champ est
  remplacé, un bloc est complété), sinon dans un nouveau bloc ;
- **Envoyer à l'agent** (⌘↩) en fait une consigne (voir [Agent](#agent)).

Une pause ne décide donc jamais à votre place. Chaque insertion s'annule avec ⌘Z.

```sh
just transcribe voix.wav turbo fr   # découpe en phrases, transcrit, mesure
OKU_SLIDES_STT__FAKE_MIC=voix.wav just serve   # simule le micro dans l'interface
```

### Agent

Un agent modifie la présentation sur consigne. La consigne se dicte comme un
texte, dans le [brouillon](#dictée) ; **Envoyer à l'agent** (⌘↩) la donne par
[ACP](https://agentclientprotocol.com) à un agent lancé par Oku Slides. Il
n'a ni fichiers ni terminal : il n'agit que par les outils d'Oku Slides,
servis en MCP sur `127.0.0.1` avec un jeton propre à chaque onglet.

- **Tout s'annule** : un tour de l'agent est une seule entrée ⌘Z.
- **Ce qui ne s'annule pas** (enregistrer, ouvrir, supprimer un ensemble de
  modèles, changer un réglage) attend votre confirmation, comme les
  permissions que l'agent demande : un clic, jamais Entrée, et jamais
  « toujours autoriser » par défaut.
- **Décisions** : l'agent pose ses questions dans une fiche sous l'élément
  (choix, reformulations : `1`–`4`, Entrée, `E` pour retoucher, Échap), propose
  des variantes rendues dans un tiroir sous la scène, et des réglages en direct
  dans un post-it. On peut répondre à une question à la voix, par le brouillon.
- **Conversation** : une par onglet, jamais enregistrée ; les onglets
  travaillent en même temps. « Nouvelle conversation » repart de zéro. ⌘⇧D
  pendant un tour l'arrête.

| Réglage | Défaut |
| --- | --- |
| `[agent] command` / `OKU_SLIDES_AGENT__COMMAND` | `npx` |
| `[agent] args` / `OKU_SLIDES_AGENT__ARGS` (séparés par des espaces) | `-y @agentclientprotocol/claude-agent-acp@0.81.2` |

N'importe quel agent ACP qui accepte des serveurs MCP en HTTP convient.

```sh
just agent-smoke fixtures/roadmap.json "Rends le titre de la slide 2 plus percutant"   # vrai agent
```

## Réglages

Les réglages ont trois couches, la dernière l'emporte :

1. les défauts, embarqués ([`config.default.toml`](crates/oku-slides-platform/src/config.default.toml),
   commentés) ;
2. `config.toml` dans le dossier de données de l'app, facultatif ;
3. les variables d'environnement `OKU_SLIDES_<SECTION>__<CLÉ>`, avec un double
   soulignement pour descendre d'un niveau.

| Variable | Effet |
| --- | --- |
| `OKU_SLIDES_PATHS__DATA` | Dossier de données de l'app |
| `OKU_SLIDES_PATHS__MODELS` | Dossier des modèles de dictée |
| `OKU_SLIDES_PATHS__FONTS` | Polices supplémentaires (TTF/OTF) |
| `OKU_SLIDES_STT__FAKE_MIC` | Fichier WAV rejoué à la place du micro |
| `OKU_SLIDES_STT__VAD__PAUSE` | Pause qui clôt une phrase (s), comme toute clé de `[stt.vad]` |
| `OKU_SLIDES_SERVE__PORT` | Port du serveur de développement |
| `OKU_SLIDES_I18N__LOCALE` | Langue de l'interface (`fr`, `en`) |

