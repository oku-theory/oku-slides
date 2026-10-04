# Paramètre liste `p.list`, saisi en tableau et éditable sur la slide — design

Date : 2026-10-04 · Tâche : t-494 · Suite : t-495 (listes imbriquées)

## Objectif

Un widget qui montre une liste (frise chronologique, rangée de KPI,
équipe) la simule aujourd'hui par des paramètres plats : `date1…date5`,
`texte1…texte5`, les items vides masqués. Le maximum est figé, le dock
montre dix champs, et l'on ne peut ni ajouter, ni retirer, ni réordonner
un item.

On ajoute un type de paramètre **liste** :

- déclaré par le widget avec des **colonnes typées** (`p.list(item: …)`) ;
- **saisi en tableau** dans le dock (la table n'est qu'une manière
  d'entrer la liste ; la slide la montre comme le widget l'entend) ;
- **éditable sur la slide** : champs de chaque item, poignées pour
  ajouter, retirer et réordonner ;
- compris par l'agent (ops, lecture, écriture de widgets).

Hors périmètre : listes imbriquées (t-495) ; slots dans un item (un item
à contenu libre est un élément : `p.slot` avec un widget enfant) ;
colonnes ajoutées par l'utilisateur (les colonnes sont celles du schéma).

## Décisions

1. **Nouveau type `p.list`**, pas un slot de widgets enfants : le widget
   reçoit les valeurs brutes et les place lui-même (date au-dessus du
   trait, libellé dessous). Un slot flow ne lui donnerait qu'un `body`
   fini par enfant.
2. **Ids d'item stables**, générés par l'hôte. Les champs, les poignées,
   l'annulation et l'agent visent un item par son id, jamais par son
   index : insérer ou réordonner ne déplace aucune adresse.
3. **Types de colonne admis** : `plain`, `rich`, `number`, `flag`,
   `choice`, `asset`. Exclus : `slot`, `list` (t-495), `grid`, `host`.
4. **Noms de colonne réservés** : `id`, `kind`, et tout nom contenant `/`.
5. **Adresse d'un champ d'item** : `<liste>/<id>/<colonne>`
   (`etapes/k3f9/date`), le propriétaire restant l'élément (ou la slide).
   `fieldKey` de l'interface ne change pas.
6. **Deux chemins d'écriture** : `Op::EditList` (insérer, retirer,
   déplacer, modifier un champ) et `Op::SetParam` avec la liste entière
   (collage, remplacement par l'agent), réconciliée par id.
7. **Annulation par Loro** : pas d'inverse écrit à la main ; appliquer
   au `Deck` puis réconcilier suffit.
8. **Dock** : un tableau typé contrôlé ; colonne `rich` éditée dans un
   popover (`RichTextField`).
9. **Slide** : poignées « + » entre les items et après le dernier, « × »
   sur l'item, ⋮⋮ pour glisser le long de l'axe, Alt+flèches au clavier.
   Montrées seulement si l'élément est sélectionné ou un de ses champs
   d'item en édition.
10. **`item-label`** optionnel sur le descripteur : nourrit « Ajouter une
    étape », « Supprimer l'étape » (argument de chaînes Fluent).

## Amendements après lecture du code (2026-10-04)

Ces points priment sur les sections qui suivent.

- **A1. Ids fournis par l'appelant.** Le cœur ne génère aucun id (ceux des
  éléments viennent de l'interface, `newId` dans `ui/src/deck.ts`).
  `ListOp::Insert` porte donc l'`id` de l'item (interface : `newId("i")` ;
  agent : un id de son choix).
- **A1 bis. Absente = vide ; le défaut amorce la création.** Un tableau
  vide est indiscernable d'un `rich` vide une fois en JSON, et
  `canonicalize` retire les valeurs vides : une liste vide est donc
  **absente**, et une liste absente est **vide** (Typst rend `()`). Le
  `default` du schéma n'amorce qu'un objet **nouveau**, déclaré par
  l'appelant — `add_element` et `add_slide` avec `defaults: true`
  (création depuis la bibliothèque, ou par l'agent), `set_kind` qui
  change de widget —, sous-arbre compris, avec des ids déterministes
  `d0`, `d1`… (`seed_lists`, cœur). Un collage, un passage en grille ou
  `replace_element` n'amorcent pas : ils feraient revenir le défaut
  d'une liste vidée. L'aperçu de
  l'éditeur de widget montre le défaut (valeurs d'essai absentes).
  `set_param` à `null` vide la liste.
- **A2. Conteneur du replica.** La lecture du replica se passe du
  registre et lit toute `LoroMovableList` de paramètres comme un slot.
  Une liste est donc une `LoroMap { list: LoroMovableList<LoroMap> }`
  (comme `rich` : `LoroMap { rich: LoroText }`), lisible sans registre,
  créée d'avance (parade t-188) ; vide, elle se relit absente.
- **A3. Validation du schéma en Rust seulement.** `check_widget`
  (`SchemaFault`, messages traduits) est l'autorité ; Typst ajoute
  seulement `"list"` à `_types`.
- **A4. Édition sur la slide construite en TS.** Les ops de champ sont
  faites par `fieldOps` (`ui/src/deck.ts`) : un champ d'item y donne
  `edit_list { set_field }`. `FieldView` gagne `list`, `item`, `column`.
- **A5. Dock : éditeur de widget seulement.** Le dock de l'hôte ne règle
  les paramètres de widget que dans l'éditeur de widget (valeurs
  d'essai) ; il n'y a pas de dock des paramètres d'élément dans le deck.
  Le tableau y modifie les valeurs d'essai par une requête
  `EditWidgetTrialList { name, param, change }`. Dans le deck, une liste
  s'édite sur la slide et par l'agent. Le collage TSV vaut pour le dock.
- **A6. Colonne `asset` en lecture seule au dock** : l'interface n'a pas
  de sélecteur d'asset et `settable` exclut déjà `asset`. Elle se règle
  par l'agent (`edit_list`).
- **A7. Agent** : il lit les slides en JSON brut (`SlideView`), items et
  ids compris ; pas de tableau compact. `ask_rewrite` reste aux champs
  de slide (`@titre`) et au premier `rich` d'un élément ; l'agent
  modifie un champ d'item par `edit_list`.
- **A8. Pas de menu contextuel** sur la scène : « × » seulement. « + »
  aussi **avant le premier** item (sinon on ne peut pas insérer en tête).
- **A9. Grille commune** dans `ui/src/table/` (`ui/src/grid.ts` existe
  déjà) : analyse TSV et conversion des cellules. `ChartDataGrid` n'a
  pas de navigation aux flèches : Tab natif, Entrée descend.
- **A10. Tests Typst** dans `crates/oku-slides-render/tests/` (pas de
  lanceur de tests Typst natif).
- **A11. Widget de base `frise`** ajouté à `typst/oku-slides/src/widgets.typ` :
  cas de recette et support de l'e2e (élément ajouté par `add_element`).

## Typst (`typst/oku-slides`)

**Descripteur** (`src/params.typ`) :

```typst
/// Liste d'items aux colonnes typées ; saisie en tableau dans le dock.
#let list(label: none, item-label: none, item: (:), min: 0, max: none, default: ()) =
  _base("list", label, default) + (item-label: item-label, item: item, min: min, max: max)
```

`widget()` (`src/widget.typ`) ajoute `"list"` à `_types` et vérifie :
chaque colonne est un descripteur d'un type admis (3), aucun nom réservé
(4), `min <= max`, `default` au plus `max` items. Message pour un type
exclu : « pour un contenu libre par item, utilisez `p.slot` avec un
widget enfant ».

**Valeur reçue par le widget** : `args.etapes` est un tableau de
dictionnaires `(id: "k3f9", date: "2019", texte: "Création")` ; une
colonne absente prend son `default`.

**Helper** `list-item(ctx, name, it, body)` (`src/widget.typ`, exporté
par `lib.typ`) :

- `body` est une fonction `(c) => content` ; `c` est `ctx` dont les
  champs s'adressent à l'item : `field(c, "date", …)` émet le champ
  `etapes/k3f9/date`. `field`, `fields-stack` et la mesure ne changent pas.
- émet, par `_measured`, une boîte `<oku-slides-item>`
  `(owner, list, item, index)` ;
- `ctx.id == none` (aperçu sans propriétaire) : rend `body(ctx)` seul.

```typst
for e in args.etapes {
  list-item(ctx, "etapes", e, c => stack(
    field(c, "date", e.date), dot, field(c, "texte", e.texte)))
}
```

## Cœur (`oku-slides-core`)

**Schéma** (`widget.rs`) :
`ParamType::List { item: Vec<ParamSchema>, item_label: Option<String>, min: usize, max: Option<usize> }`,
avec les vérifications de 3 et 4 à la lecture du schéma.

**Valeur** (`params.rs`) :

```rust
/// Items d'une liste (`list`) : objets à `id` sans `kind`.
ParamValue::List(Vec<ListItem>)
pub struct ListItem { pub id: String, pub values: Params } // serde : id + flatten
```

- Détection par la forme : tableau d'objets ayant tous un `id` chaîne et
  **aucun** `kind` (un élément en a un). Tableau vide : règle actuelle,
  et `Params::list(name)` lit un `Rich`/`Slot` vide comme une liste vide.
- Accesseurs `list()`, `list_mut()`, `lists()`, sur le modèle des slots.

**Replica** (`replica/schema.rs`, `read.rs`, `write.rs`) :

- `declared_kind(List)` → `Kind::Collection(Ctx::Item)` (nouveau
  contexte) : `LoroMovableList` de `LoroMap` à `id`, réconciliée par
  `sync_list` (suppressions, `mov` chacun dans son commit — t-263 —,
  insertions).
- Dans `Ctx::Item` : `id` → `Atom` ; colonnes typées par le schéma de
  l'item (`plain` → `Prose`, `rich` → `Rich`, autres → `Atom`), sinon par
  la forme de la valeur. Le contexte doit donc porter le schéma de
  l'item, comme `Ctx::Params` porte celui du widget.
- `fits(Collection, …)` accepte les items (`is_items`) comme les éléments.
- `declared()` crée la liste d'avance (parade t-188).

**Op** (`ops/mod.rs`, nouveau `ops/list.rs`), sur le modèle de `SetChartData` :

```rust
Op::EditList { target: Target, param: String, change: ListOp }
pub enum ListOp {
    Insert { index: Option<usize>, item: ListItemInput }, // id absent → attribué
    Remove { item: String },
    Move { item: String, to: usize },
    SetField { item: String, field: String, value: Value },
}
impl ListOp { pub fn apply(&self, items: &mut Vec<ListItem>, schema: &ListSchema) -> Result<ListChange> }
```

- `ListOp::apply` est **la** mise en œuvre, appelée par `Op::EditList`
  et par les valeurs d'essai du widget (session).
- `Insert` complète les colonnes absentes par leur `default` et renvoie
  l'id attribué (`ListChange`).
- Erreurs Snafu traduites : item inconnu, colonne inconnue, mauvais
  type, `Insert` au-delà de `max`, `Remove` sous `min`.
- `SetParam` d'une liste : valeurs vérifiées contre les colonnes ; ids
  absents attribués.

**Ids** : courts, uniques dans la liste ; les items de `default` en
reçoivent à l'instanciation du widget.

**Validation** (`validate.rs`) : ids d'item vides ou dupliqués, `min` et
`max`, types de colonne, assets des colonnes `asset` présents dans les
sources.

**Champs** (`fields.rs`) : `text_fields` insère les colonnes `plain` et
`rich` de chaque item, dans l'ordre des items (Tab passe d'une étape à
la suivante).

## Rendu (`oku-slides-render`)

`Layout` gagne `items: Vec<ItemBox>`, lus de `<oku-slides-item>` comme
`fields` ; triés par page. `SlideView` les transmet à l'interface.

## Session (`oku-slides-session`)

- Un champ d'adresse `liste/id/colonne` (édition sur la slide, dictée,
  `ask_rewrite`) se résout en `EditList { SetField }`, avec la clé de
  fusion d'une séance de frappe.
- `widget_params::field` : `List` → `ParamField { kind: List, columns, value: items }`
  (`FieldKind::List` dans `agent/forms.rs`) ; compte pour un champ des 8.
- Évènement `host_param` `{ key, change: ListOp }` : dans le deck →
  `Op::EditList` ; dans l'éditeur de widget → `ListOp::apply` sur les
  valeurs d'essai (`set_widget_trials`).
- L'insertion renvoie à l'interface l'id créé (focus, voir Slide).

## Interface (TypeScript)

**Dock** — `ui/src/a2ui/ListTable.tsx`, composant contrôlé (TanStack
Table), aucune donnée modifiée localement :

| Colonne | Cellule |
|---|---|
| `plain` | input une ligne |
| `number` | input numérique, bornes du schéma |
| `flag` | case à cocher |
| `choice` | select |
| `asset` | vignette ; clic → sélecteur d'asset existant |
| `rich` | aperçu une ligne ; clic → popover avec `RichTextField` |

- « + Ajouter <item-label> » en bas (et Entrée sur la dernière ligne),
  désactivé à `max` avec infobulle ; « × » par ligne, désactivé à `min`.
- Poignée de glisser et Alt+↑/↓ pour réordonner.
- Tab / Maj+Tab de cellule en cellule, Entrée descend.
- Collage TSV : depuis la cellule courante, colonnes dans l'ordre,
  valeurs converties au type, lignes manquantes insérées (jusqu'à
  `max`), cellules invalides laissées et signalées ; un seul `SetParam`
  (une étape d'annulation).
- Frappe : `SetField` avec clé de fusion (une séance = une étape).
- Hauteur bornée, défilement interne.

**Commun aux grilles** — `ui/src/grid/` : navigation clavier entre
cellules et analyse TSV, extraites de `ChartDataGrid`, qui les utilise
aussi. Pas de composant unique : colonnes libres et en-têtes éditables
d'un côté, colonnes typées de l'autre.

**Slide** — `ItemLayer` dans `Stage.tsx`, à côté de `GridLayer` :

- Montré si l'élément propriétaire est sélectionné ou si un champ d'un
  de ses items est en édition ; groupé par `(owner, list)`.
- « + » entre deux items et après le dernier (`Insert { index }`),
  caché à `max` ; « × » au coin de l'item survolé ou focalisé, caché à
  `min`, et « Supprimer <item-label> » au menu contextuel.
- ⋮⋮ sur l'item survolé : glisser le long de l'axe, repère de dépôt
  entre deux items, puis `Move`. Alt+←/→ ou ↑/↓ depuis un champ focalisé.
- Axe déduit des centres des items ; un seul item : « + » du côté où le
  cadre de l'élément a le plus de place.
- Après une insertion : focus du premier champ texte du nouvel item
  (`fieldKey(owner, "etapes/<id>/date")`), texte sélectionné. Un clic
  sur la boîte d'un item sans champ mesuré focalise son premier champ.

**Libellés** : chaînes Fluent nouvelles (ajouter, supprimer, limites,
erreurs) dans **toutes** les langues présentes ; `item-label` n'en est
qu'un argument, sinon libellé générique (« un élément »).

## Agent

- `edit_list` vient du schéma JSON généré des ops ; sa doc dit d'adresser
  les items par id et d'utiliser `set_param` pour tout remplacer.
- Lecture d'une slide : une liste montrée en tableau compact, une ligne
  par item avec son id (`k3f9 · 2019 · Création de l'entreprise`).
- Champs : les outils qui prennent un champ acceptent `etapes/k3f9/texte`.
- Écriture de widgets : consignes de l'éditeur de widget et
  `typst/oku-slides/README.md` documentent `p.list`, `list-item`,
  `item-label`, les types admis, avec la frise en exemple.

## Documentation

`ARCHITECTURE.md` de `oku-slides-core`, `oku-slides-render`,
`oku-slides-session` ; deck d'aide en lecture seule (section
bibliothèque Typst, t-469) ; `typst/oku-slides/README.md`.

## Tests

| Couche | Cas |
|---|---|
| Typst | `widget()` refuse un type de colonne exclu, un nom réservé, `/` ; `list-item` sans `ctx.id` rend le corps ; le ctx dérivé préfixe les champs |
| Cœur — valeur | détection `List` / `Slot` / `Rich` (item sans `kind`, tableau vide) ; aller-retour serde |
| Cœur — ops | `ListOp::apply` : insertion avec défauts et id attribué, retrait, déplacement, `SetField` ; erreurs (item ou colonne inconnus, type, `max`, `min`) ; `SetParam` de liste vérifié |
| Cœur — replica | deux insertions concurrentes gardées ; déplacement et édition concurrents ; frappe concurrente dans une même cellule `plain` fusionnée ; annuler insertion, déplacement, retrait |
| Cœur — validation, champs | ids dupliqués, `min`/`max`, asset de colonne absent, ordre de Tab à travers les items |
| Rendu | `items` collectés avec leurs boîtes ; plusieurs listes dans un widget |
| Session | adresse `liste/id/colonne` → `EditList` ; `host_param` + `ListOp` vers le deck et vers les valeurs d'essai |
| Interface (rendu) | `ListTable` : ajout, retrait, réordonnancement clavier, collage TSV converti, popover `rich`, limites ; `ItemLayer` : « + » sur le bon axe, focus après insertion |
| e2e | frise : « + » sur la slide, frappe, glisser ⋮⋮, « × » puis annuler ; données isolées (`OKU_SLIDES_PATHS__DATA` et `__DECKS` dans un dossier temporaire) |

Recette : la frise chronologique réécrite avec `p.list` s'édite au dock
et sur la slide (ajout, retrait, réordonnancement, champs, annulation).
