# Packs portables : identité, versions et pack embarqué — design

Date : 2026-10-01 · Tâche : t-328

## Objectif

Un deck se rend partout comme chez son auteur. Le fichier embarque la
version exacte du pack de modèles qu'il utilise (modèles de slides et
widgets) ; la bibliothèque de l'utilisateur devient versionnée et
identifie chaque pack par un id unique.

Programme « widgets portables » : 0. compilation bornée (t-327, livrée) ;
1. **packs portables** (cette spec) ; 2. création de widgets par l'agent et
éditeur Typst (t-329).

Hors périmètre : le ménage des versions, le choix d'une version arbitraire
d'un pack (au-delà des deux actions du bandeau), la création de widgets.

## Décisions

- **Référence et cache.** Le deck référence un pack (id) dans une version ;
  le fichier embarque cette version. Écarté : le pack vivant dans le
  replica Loro (code Typst fusionné au caractère).
- **Le cache fait foi au rendu.** Le deck se rend avec la version qu'il
  embarque, jamais avec celle de la bibliothèque. Une autre version dans la
  bibliothèque donne un bandeau, jamais un rendu différent.
- **Tout deck embarque son pack**, intégré ou non. Un pack intégré est un
  pack préinstallé, d'id connu.
- **Bibliothèque versionnée.** Un pack est un ensemble de versions
  adressées par contenu sous un même id ; chaque version connaît sa
  parente. Une version **courante** par pack, qui ne change que par une
  action de l'utilisateur.
- **Les versions reçues s'ajoutent en silence** à la bibliothèque, sans
  toucher la version courante.
- **Id = UUID v4** : dépendance `uuid` (features `v4`, `serde`), déjà
  présente de façon transitive dans `Cargo.lock`, déclarée au workspace.
- **Pas de rétrocompatibilité.** `DECK_VERSION` passe à 3,
  `SOURCES_VERSION` à 4 ; les decks v2 et les packs au format slug ne sont
  plus lus.

## Modèle (core)

```rust
pub struct PackRef { pub id: Uuid, pub version: PackVersion }
pub struct PackVersion(String);   // "sha256:<hex>", contenu canonique
pub struct Deck { …, pub templates: Option<PackRef>, … }
```

- `templates: None` : le pack par défaut (`essentiel`, id constant), dans
  la version courante de la bibliothèque à la création du deck, puis
  embarquée et référencée explicitement au premier enregistrement.
- **Contenu canonique** d'une version : `pack.json` sérialisé en JSON à
  clés triées, un octet `0`, puis `widgets.typ` tel quel (vide s'il est
  absent). `PackVersion` est son sha-256. Nom et description sont dans
  `pack.json` : les changer crée une version.
- `pack.json` : `{ id, name, description, parent: Option<PackVersion>,
  created, templates }`. Le slug disparaît.
- Dans le replica Loro, `templates` est un atome (dernier écrivain gagne).

## Fichier (zip des sources, version 4)

```
manifest.json     { format: "oku-slides-sources", version: 4 }
deck.loro
assets/<hex>
fonts/<nom>
packs/<hex>/pack.json
packs/<hex>/widgets.typ    (absent si le pack n'a pas de widgets)
```

- `SourceBundle.packs: BTreeMap<PackVersion, PackFiles>`, en ajout seul,
  comme `assets` et `fonts`. `merge` fait l'union ; `deck.templates`
  suit la fusion du replica.
- À l'enregistrement, le bundle garde toute version référencée par le deck
  courant ou par l'historique du replica (le mode historique restaure aussi
  le pack d'une version passée).
- À la lecture, une version dont le hash ne correspond pas au contenu, ou
  dont le chemin n'est pas `<hex>` de 64 caractères, est écartée et
  signalée (`Issue::PackVersionMismatch`). Les packs comptent dans
  `max_sources_mib`.

## Bibliothèque (session)

```
<data>/templates/<uuid>/current            # PackVersion de la version courante
<data>/templates/<uuid>/versions/<hex>/pack.json
<data>/templates/<uuid>/versions/<hex>/widgets.typ
```

- `Library` : `packs()` (id, nom de la version courante, intégré ou non),
  `versions(id)`, `version(id, v) -> Option<PackFiles>`,
  `current(id) -> Option<PackVersion>`, `add_version(id, files)` (sans
  toucher `current`), `set_current(id, v)`, `relation(id, a, b) ->
  {Same, Ancestor, Descendant, Sibling}` (par la chaîne des `parent`).
  Écriture atomique, via `Storage`.
- **Packs intégrés** (`templates/essentiel`, `templates/grilles` du dépôt,
  embarqués dans l'app) : id constant dans leur `pack.json`, version lue en
  lecture seule. Leur version courante est celle de l'app tant que
  l'utilisateur n'a pas de version à lui (pas de fichier `current`) ; dès
  qu'il en a une, c'est la sienne, et une nouvelle version de l'app n'ajoute
  qu'une version sœur.
- `current` qui pointe vers une version absente : la plus récente par
  `created`, avertissement journalisé. Un dossier de version illisible est
  journalisé et ignoré.

## Flux

### Rendu

- `resolve_widgets` lit le pack dans les sources du deck :
  `bundle.packs[templates.version]`, et le charge par
  `RenderWorker::probe(PackWidgets::new(<hex>, widgets.typ))` — l'id de
  pack dans le monde Typst devient le hash.
- Version référencée absente du bundle (deck JSON de développement,
  fichier abîmé) : widgets de base et avis `session-pack-missing`.
- Pas de `widgets.typ` : widgets de base, sans avis.

### Ouverture d'un fichier

1. Chaque version embarquée absente de la bibliothèque y est ajoutée
   (`add_version`). Un id inconnu crée le pack, avec cette version comme
   courante.
2. Si la version du deck diffère de la version courante de son pack, la
   session pose un **bandeau** (vue `pack_notice`) : pack, relation
   (`Ancestor` : plus ancienne ; `Descendant` : plus récente ; `Sibling` :
   parallèle), et deux actions :
   - **Passer à ma version** : `SetTemplates` vers la version courante,
     dans une transaction annulable ; la version est ajoutée au bundle.
   - **Faire de la sienne ma version courante** : `Library::set_current`.
   Le bandeau se ferme pour la session ; il ne bloque jamais l'ouverture.

### Mode modèles

- Il modifie la version **courante** du pack du deck. Enregistrer crée une
  version (`parent` = l'ancienne courante, `created` = maintenant), la rend
  courante, l'ajoute au bundle du deck mis de côté, qui y passe
  (`SetTemplates`).
- **Une version par session d'édition.** Le premier enregistrement (manuel
  ou automatique) d'une session du mode modèles crée la version (`parent` =
  la courante au début de la session) et la rend courante ; chaque
  enregistrement suivant de la même session la **remplace** (même `parent`) :
  la version intermédiaire est supprimée de la bibliothèque (le bundle du deck
  reste en ajout seul), jusqu'à la sortie du mode modèles.
- « Enregistrer comme copie » : un nouvel id, sans `parent` ; la session
  continue sur la version de la copie.
- Modifier un pack intégré crée la version de l'utilisateur sous l'id du
  pack intégré (plus de copie séparée).

### Nouveau deck

Il prend la version courante du pack par défaut, l'ajoute à son bundle et
la référence.

### CLI

`render` et `save` d'un deck dont le pack a des widgets passent par le
processus de rendu (`RenderWorker`) : le code d'un pack reçu y est isolé
(spec t-327). Sans widgets de pack, le `Renderer` en mémoire reste.

### MCP et agent

L'outil `widgets` décrit les widgets de la version embarquée. Aucun nouvel
outil.

## Erreurs

- core : `Issue::PackVersionMismatch { version }`,
  `Issue::PackVersionMissing { version }` ; erreur de lecture du zip
  existante pour un chemin invalide.
- session : `UnknownPack { id }`, `PackVersionMissing { id, version }`
  (Snafu, backtrace, minuscules).
- Clés Fluent, 20 langues : erreurs ci-dessus, `session-pack-missing`,
  bandeau (`pack-notice-older`, `pack-notice-newer`, `pack-notice-parallel`,
  `pack-notice-use-mine`, `pack-notice-make-current`).

## Dépôt

- `templates/essentiel/pack.json`, `templates/grilles/pack.json` : id
  constant, `parent: null`, `created` fixe.
- Fixtures, tests CLI, MCP, session, agent et README de `typst/oku-slides`
  réécrits en v3 (`templates` : `PackRef`, bundles avec `packs/`).
- `Library` réécrite ; `free_id`, `slug`, `is_builtin` par slug
  disparaissent.
- UI : `api.ts` (`PackRef`, `pack_notice`), bandeau dans la vue du deck,
  liste des packs par id.

## Tests

Dans l'ordre de construction, en TDD :

1. **Core** : `PackVersion` du contenu canonique stable et indépendant de
   l'ordre des clés ; `PackRef` aller-retour ; zip : version intègre lue,
   hash faux écarté et signalé, chemin invalide refusé ; fusion de deux
   bundles = union des versions ; versions référencées par l'historique
   gardées à l'enregistrement.
2. **Bibliothèque** : `add_version` ne change pas `current` ;
   `set_current` ; `relation` (même, ancêtre, descendante, sœur) ; version
   intégrée en lecture seule et courante par défaut ; `current` vers une
   version absente.
3. **Session** : le rendu utilise la version embarquée même si la
   bibliothèque en a une autre ; ouverture : version reçue ajoutée en
   silence ; bandeau et ses deux actions (dont annuler « Passer à ma
   version ») ; mode modèles : nouvelle version dont `parent` est
   l'ancienne courante ; copie : nouvel id ; historique : restaurer une
   version passée restaure son pack ; version absente du bundle : base et
   avis.
4. **CLI** : `render` d'un PDF à pack embarqué passe par le processus de
   rendu ; un pack qui boucle ne la gèle pas.
5. **Régression** : un deck reçu avec le même id de pack mais un autre
   contenu ne change jamais la version courante de la bibliothèque.
6. **i18n** : nouvelles clés dans les 20 langues.
