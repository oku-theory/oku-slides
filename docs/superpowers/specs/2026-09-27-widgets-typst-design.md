# Widgets Typst à paramètres nommés — design

Date : 2026-09-27 · Tâche : t-270

## Objectif

Les composants de slide (carte, texte, image, forme, graphique, groupe) et
les slides elles-mêmes deviennent des **widgets déclarés en Typst**, chacun
avec un schéma de paramètres nommés et typés. Le modèle du deck passe d'un
enum Rust fermé (`ElementBody`, `SlideKind`) à une forme générique
`{ id, kind, params }`, validée contre ce schéma.

C'est la fondation de l'édition par zones : une carte expose `kicker`,
`title` et `body`, trois paramètres Typst, que l'édition en place pourra
cibler séparément.

Programme en trois sous-projets, chacun avec sa spec :

1. **Widgets Typst et modèle générique** (cette spec, t-270).
2. Zones éditables en place par widget (t-271) : un éditeur par zone, à sa
   position exacte, placeholders des zones vides, Tab entre zones.
3. Quasi-rendu en ligne de l'éditeur (t-272) : marques markdown masquées
   hors du curseur, style fidèle à la zone.

Hors périmètre ici : l'éditeur par zone et le quasi-rendu (sous-projets 2
et 3), les widgets embarqués dans un deck, la lecture des decks v1.

## Décisions

- **Schéma déclaré en Typst, lu par introspection.** Une seule source de
  vérité : le fichier `.typ` qui rend le widget déclare aussi ses
  paramètres. Écartés : un manifeste JSON à côté (deux fichiers qui
  divergent) et la déduction depuis la signature de la fonction (Typst
  n'expose ni les noms ni les types des paramètres).
- **Tout devient widget**, slides comprises. `image`, `shape`, `chart` et
  `group` aussi ; l'enum `ElementBody` disparaît.
- **Widgets fournis par la bibliothèque et par les packs de templates.** Un
  deck ne porte pas de code Typst ; il référence un pack par son nom.
- **Une slide est un widget racine.** Ses conteneurs d'éléments sont des
  paramètres `slot`.
- **JSON explicite** : `{"id", "kind", "params": {…}}`.
- **Pas de rétrocompatibilité.** `DECK_VERSION` passe à 2 ; un deck v1
  (y compris ses sources Loro) n'est pas lu. Fixtures et templates sont
  réécrits.
- **Vocabulaire.** Une *zone* est une région de texte éditable en place ;
  un *slot* est un paramètre qui contient des éléments. L'enum
  `Zone { Blocks, Elements }` actuelle disparaît au profit des slots.

## Côté Typst

### Pack

La bibliothèque `typst/oku-slides` fournit le pack de base. Un pack de
templates devient un dossier (intégré ou dans le dossier de données de
l'utilisateur) :

```
templates/grilles/
  pack.json        # id, nom, description, slides modèles (ex-grilles.json)
  widgets.typ      # optionnel : widgets du pack, ajoutés à la base
```

### Déclaration

`src/widget.typ` exporte `widget`, `zone` et `p` :

```typst
#let card = widget("card",
  label: "Carte", category: "element",
  params: (
    kicker:    p.plain(label: "Surtitre"),
    title:     p.plain(label: "Titre"),
    body:      p.rich(label: "Corps"),
    highlight: p.flag(label: "Mise en avant", default: false),
  ),
  (args, ctx) => { … zone("kicker", args.kicker, style: …) … })
```

Descripteurs `p.*`, tous sérialisables en dictionnaire
`(type: …, label: …, default: …, bind: …)` :

| Descripteur | Valeur |
|---|---|
| `p.plain()` | texte sur une ligne, sans mise en forme |
| `p.rich()` | paragraphes à runs (`Paragraph`) |
| `p.flag()` | booléen |
| `p.number(min:, max:)` | nombre |
| `p.choice(..values)` | une valeur parmi une liste |
| `p.asset()` | identifiant d'asset `sha256:…` |
| `p.slot(mode: "flow" \| "free", accepts: (..kinds) \| "any")` | liste d'éléments |
| `p.grid()` | grille `{columns, rows, gap}` |
| `p.host("chart")` | type fourni et validé par l'hôte Rust |

`bind: "<clé>"` (sur `plain`) lie le paramètre à `deck.params.<clé>` : la
valeur y est lue et écrite. La couverture s'en sert pour `title` et
`subtitle`.

`widget(name, label:, category:, params:, render)` renvoie
`(name, schema, render)`. `category` vaut `"slide"` ou `"element"`. Le
registre d'un pack est le tableau `registry` exporté par son `widgets.typ` ;
un nom déjà présent dans la base **remplace** le widget de base (voir
« Surcharge »).

### Zones

`zone(name, content, style:)` généralise `tracked()` : elle émet
`#metadata((id: "<élément>.<zone>", page, x, y, w, h, ax, aw, style))<oku-slides-box>`
avec la sonde de style (`text-probe`), puis le contenu. L'id de l'élément
vient du contexte de rendu posé par le rendu générique. Chaque widget
enveloppe ses paramètres `plain` et `rich` dans `zone()`. Le sous-projet 2
consomme ces boîtes ; ce sous-projet se contente de les émettre et de les
tester.

### Rendu générique

`data.typ` ne teste plus le `kind`. Pour chaque objet :

1. `registry.at(kind)` ; absent → l'encart « élément inconnu » actuel.
2. Complète `params` avec les `default` du schéma ; résout les `bind`.
3. Convertit chaque `rich` en contenu (`_paragraphs`, interpolation des
   `vars`), chaque `plain` par `interp`.
4. Rend chaque `slot` récursivement en un tableau de contenus. Dans un slot
   `free`, chaque enfant est déjà placé (`place` sur son `frame` ou sa
   `cell`) ; dans un slot `flow`, l'agencement (pile, colonnes, grille)
   revient au widget.
5. Appelle `render(args, ctx)` ; `ctx` porte le thème, les vars, l'id.
6. Emballe l'élément dans sa boîte `<oku-slides-box>` et son label `e-<id>`
   comme aujourd'hui.

Les slides actuelles (`title-slide`, `content-slide`, `chapter-slide`,
`toc-slide`, `end-slide`, `blank-slide`) deviennent des widgets de
catégorie `slide`. Le layout de `content` (`default`, `columns`, `split`,
`center`, `quote`) devient un paramètre `choice`. Leurs éléments vivent
dans `blocks: p.slot(mode: "flow")` et `elements: p.slot(mode: "free")`.

Les éléments actuels deviennent :

| Widget | Paramètres |
|---|---|
| `text` | `body: rich` |
| `card` | `kicker: plain`, `title: plain`, `body: rich`, `highlight: flag` |
| `image` | `asset: asset`, `fit: choice("cover", "contain", "stretch")` (défaut `cover`) |
| `shape` | `shape: choice("rect", "ellipse", "line")` |
| `chart` | `option: host("chart")` |
| `stack` | `layout: choice("rows", "columns", "grid")`, `count: number`, `gap: choice(jetons)`, `children: slot(flow)` |
| `canvas` | `children: slot(free)` |

`image` et `chart` gardent le chargeur d'images de l'hôte (`image-loader`,
`world.rs`) : leur `render` le reçoit par `ctx`.

Le `group` actuel se scinde en `stack` (ex-layouts `rows`, `columns`,
`grid`) et `canvas` (ex-layout `free`) : le mode d'un slot est fixé par le
schéma, pas par la valeur d'un autre paramètre. Dans un slot `free`, un
enfant sans `frame` ni `cell` est une issue de validation ; dans un slot
`flow`, un `frame` ou une `cell` aussi.

### Sonde de registre

L'hôte compile un document minimal par pack :

```typst
#import "widgets.typ": registry
#metadata(registry.map(w => (name: w.name, schema: w.schema)))<oku-widgets>
```

## Registre et modèle Rust

### Registre (`oku-slides-core`, sans dépendance à Typst)

```rust
pub struct WidgetRegistry { widgets: BTreeMap<String, WidgetSchema>, fingerprint: String }
pub struct WidgetSchema  { name: String, label: String, category: Category,
                           params: IndexMap<String, ParamSchema> }
pub enum   Category      { Slide, Element }
pub struct ParamSchema   { label: String, ty: ParamType,
                           default: Option<Value>, bind: Option<String> }
pub enum   ParamType     { Plain, Rich, Flag, Number { min: Option<f64>, max: Option<f64> },
                           Choice(Vec<String>), Asset,
                           Slot { mode: SlotMode, accepts: Accepts }, Grid, Host(HostType) }
pub enum   SlotMode      { Flow, Free }
pub enum   Accepts       { Any, Kinds(Vec<String>) }
pub enum   HostType      { Chart }
```

### Chargement (`oku-slides-render`)

`load_registry(pack) -> Result<WidgetRegistry, RegistryError>` compile la
sonde avec le `World` du rendu, lit `<oku-widgets>`, fusionne avec la base
(règle de surcharge), désérialise. Cache par empreinte (hash de la
bibliothèque et de `widgets.typ`).

**Surcharge.** Un pack peut redéfinir un widget de base si chaque paramètre
de base existe avec le même type ; les ajouts sont permis. Sinon le pack ne
se charge pas (`IncompatibleOverride`). Un deck reste ainsi valide quel que
soit le pack.

### Modèle

```rust
pub struct Slide   { id: String, kind: String, params: Params,
                     name: Option<String>, description: Option<String>, notes: String }
pub struct Element { id: String, kind: String, frame: Option<Frame>, cell: Option<Cell>,
                     style: Style, params: Params }
pub struct Params(BTreeMap<String, ParamValue>);
pub enum   ParamValue { Plain(String), Rich(Vec<Paragraph>), Scalar(Value),
                        Slot(Vec<Element>), Host(HostValue) }
pub enum   HostValue  { Chart(Box<oku_slides_chart::ChartOption>) }
```

JSON :

```json
{ "id": "c1", "kind": "card",
  "params": { "kicker": "Pilier 1", "title": "Fiabilité",
              "body": [ { "list": "bullet", "runs": [ { "t": "Diviser par deux les incidents" } ] } ] } }
```

- **Désérialisation guidée par le schéma** (`DeserializeSeed` portant
  `&WidgetRegistry`) : le schéma dit que `body` est `Rich`, `children`
  `Slot`, `option` `Host(Chart)` (validé par `oku_slides_chart::normalize`).
- **Rien ne se perd.** Un paramètre absent du schéma, ou tout le `params`
  d'un `kind` inconnu, est conservé en `ParamValue::Scalar` et ressort à
  l'identique. `validate` le signale ; le rendu l'ignore ou affiche
  l'encart d'erreur.
- Disparaissent : `ElementBody`, `SlideKind`, `SlideLayout`, `GroupLayout`,
  `Zone`, `ShapeKind` (devient un `choice`), les champs `title`,
  `subtitle`, `kicker`, `layout`, `text`, `blocks`, `elements`, `grid` de
  `Slide`.
- Restent : `Style`, `Frame`, `Cell`, `Grid`, `Paragraph`, `Run` —
  placement et présentation communs à tous les widgets. `Deck.params` garde
  ses champs.
- `DECK_VERSION = 2`.

## Replica Loro et ops

### Replica

`replica/schema.rs` cesse d'être une table statique :

- `Ctx::Slide` et `Ctx::Element` ne couvrent plus que les attributs communs
  (`id`, `kind`, `frame`, `cell`, `style`, `name`, `description`, `notes`).
  Leur clé `params` est une `LoroMap` de contexte `Ctx::Params(&WidgetSchema)`.
- Dans `Ctx::Params`, le type d'une clé vient du schéma : `Plain` → `Prose`,
  `Rich` → `Rich`, `Slot` → `Collection(Element)`, `Host` et scalaires →
  `Atom`. Clé inconnue → `Atom`.
- Créés dès la création de l'objet (parade t-188) : tous les paramètres
  `plain`, `rich` et `slot` du schéma.
- `reconcile` et la lecture `Deck ← LoroDoc` prennent `&WidgetRegistry` ;
  `merge` non (fusion au niveau Loro), la relecture qui suit si.
- `SetKind` concurrent d'une édition de paramètre : le paramètre survit,
  inconnu du nouveau widget, conservé et signalé.

### Ops

| Aujourd'hui | Demain |
|---|---|
| `SetText`, `SetSlideField`, `SetParam` (couverture) | `SetParam { target, param, value }`, validé contre le schéma ; un paramètre `bind` écrit dans `deck.params` |
| `SetParam { key, value }` (deck) | `SetDeckParam { key, value }` |
| `AddElement { slide, zone, parent, index, element }` | `AddElement { slot: SlotRef { owner: Target, param }, index, element }` |
| `SetGrid` | `SetParam` sur le paramètre `grid` du widget de slide |
| `SetChartOption`, `SetChartData` | conservées, adressées par `(target, param)` |
| — | `SetKind { target, kind }` : change de widget, garde les paramètres homonymes |
| `ReplaceElement`, `RemoveElement`, `SetStyle`, `ReplaceStyle`, `SetFrame`, `SetCell`, `AddSlide`, `RemoveSlide`, `MoveSlide`, `SetNotes`, `SetTheme`, `SetTemplates`, `SetToken`, `SetVar` | inchangées (adressage `slide`/`element` conservé) |

`Target` : `Slide(id)` ou `Element { slide, element }`. `locate` devient un
parcours générique des slots.

`Op::apply` prend `&WidgetRegistry`.

### Validation

`validate(deck, registry)` renvoie des `Issue` : `kind` inconnu, paramètre
inconnu, type incompatible, `choice` hors liste, `slot` qui refuse un
`kind`, placement incohérent avec le mode du slot (voir « Rendu
générique »), widget de catégorie `element` à la place d'une slide (et
inversement), `bind` vers une clé de `deck.params` inconnue. Un deck
incohérent s'ouvre ; l'app affiche les problèmes. Les ops, elles, refusent
une valeur mal typée.

## Consommateurs

### Session

- Détient `Arc<WidgetRegistry>` du pack actif ; le recharge sur
  `SetTemplates`. Pack en erreur → diagnostic affiché, retour au pack de
  base.
- `views.rs` : `Target` porte `widget` et la liste des paramètres du schéma
  (nom, type, label). Les ids `@title`, `@subtitle`, `@kicker`, `@text`
  disparaissent : les zones s'appellent `<id>.<param>` (slide comprise).
- Nouvelle vue `widgets()` : le registre (nom, label, catégorie,
  paramètres).

### MCP et agent

- Outil `widgets` : le registre en JSON Schema, un schéma par widget avec
  ses `params` typés. Le prompt de l'agent y renvoie avant toute création
  d'élément.
- Les schémas d'arguments des ops décrivent `params` comme un objet ; la
  vérification a lieu à l'application, et l'erreur localisée nomme le
  paramètre et le type attendu.

### UI

- `api.ts` : `Element`, `Slide` génériques, type `WidgetSchema`.
- `deck.ts` : `textOps` produit `set_param`.
- `slideTools.ts`, `a2ui/panelActions.ts` : listes de widgets lues dans le
  registre.
- L'édition en place actuelle continue de fonctionner : sur une carte elle
  édite `body`, sur un texte `body`, sur une slide les zones `plain`
  existantes. Le positionnement par zone arrive avec t-271.

### Dépôt

- `fixtures/*.json`, `templates/*` (au format dossier), tests CLI, MCP,
  session, agent et README de `typst/oku-slides` réécrits en v2.
- Régressions dont l'objet disparaît (p. ex. `issue_0164`) : adaptées au
  nouveau modèle si le risque subsiste (poids d'un élément), sinon
  supprimées.
- i18n : clés Fluent des nouvelles erreurs dans les 20 langues.

## Erreurs

- `RegistryError` (render) : `Compile { diagnostics }`, `MissingRegistry`,
  `BadSchema { widget, reason }`,
  `IncompatibleOverride { widget, param, expected, found }`.
- `OpError` étendu : `UnknownWidget`, `UnknownParam`,
  `ParamType { param, expected }`, `SlotRejects { slot, kind }`,
  `ChoiceOutOfRange`, `WrongCategory`.
- Snafu avec backtrace, messages en minuscules, localisés via Fluent.

## Tests

Dans l'ordre de construction, en TDD :

1. **Typst** : la sonde du pack de base produit le JSON attendu (snapshot
   `insta`) ; `zone()` émet `c1.title` avec boîte et style ; le rendu
   générique de `fixtures/roadmap` est identique au rendu actuel
   (comparaison des pages rendues).
2. **Registre** : descripteurs `p.*` ; cache par empreinte ; surcharge
   (`rstest` : ajout accepté, renommage refusé, changement de type refusé).
3. **Modèle** : désérialisation guidée ; paramètres et `kind` inconnus
   conservés à l'identique en aller-retour.
4. **Ops** : `SetParam` pour chaque type ; `bind` vers `deck.params` ;
   `SetKind` garde les homonymes ; `AddElement` refusé par un slot.
5. **Loro** : aller-retour `Deck → Loro → Deck` ; fusions titre ‖ corps de
   la même carte, `SetKind` ‖ édition de paramètre ; `convergence.rs` porté
   en v2.
6. **Session, MCP, UI** : outil `widgets` ; `set_param` depuis l'édition en
   place ; rechargement du registre sur `SetTemplates`.
