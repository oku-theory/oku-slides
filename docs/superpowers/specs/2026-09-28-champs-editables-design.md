# Champs éditables en place par widget — design

Date : 2026-09-28 · Tâche : t-271 · Suite de : t-270 (widgets Typst)

## Objectif

Chaque paramètre texte (`plain`, `rich`) d'un widget, slide comprise,
s'édite en place, à la position exacte de son texte rendu. Les champs vides
montrent un placeholder. Tab et Shift-Tab parcourent les champs de la slide.

Sous-projet 2 du programme de la spec `2026-09-27-widgets-typst-design.md`.
Le sous-projet 3 (t-272) masque les marques markdown et reprend le style
par run.

## Décisions

- **Champs vides sans place au rendu.** `field()` émet toujours sa boîte,
  même vide. Un champ vide ne change pas la mise en page : l'export reste
  identique. L'UI dessine le placeholder par-dessus.
- **Tab sur toute la slide, dans l'ordre du modèle** : les champs de la
  slide dans l'ordre du schéma, puis les éléments dans l'ordre des slots
  (profondeur d'abord), et pour chacun ses champs dans l'ordre du schéma.
  Shift-Tab revient en arrière. L'ordre ne dépend pas de la position à
  l'écran.
- **Un commit par champ, une étape d'annulation par session de saisie.**
  Chaque sortie de champ applique `set_param` et re-rend. Les applications
  d'une même session fusionnent en une étape d'annulation.
- **Double-clic pour entrer, clic pour passer.** Un clic sélectionne
  l'objet. Un double-clic édite le champ visé. En édition, un clic sur un
  autre champ y passe.
- **L'éditeur recouvre exactement le texte rendu.** t-271 garantit la
  géométrie : origine, ligne de base de chaque ligne, largeur de retour à
  la ligne, typographie. Un champ `plain`, ou un `rich` sans marque,
  coïncide au pixel. Le texte qui suit une marque markdown glisse jusqu'à
  t-272.
- **Deux couches dans la vue** : `targets` (boîtes d'objets, pour
  sélectionner et déplacer) et `fields` (champs, pour éditer).

## Typst

### `field()` et champs vides

`field(ctx, name, body, style:)` accepte `body == none` ou `""`. Elle
n'émet alors que la métadonnée `<oku-slides-field>`, sans contenu visible,
avec :

- `empty: true`, `h: 0` ;
- `style` : la sonde `text-probe(style)` du champ ;
- `line` : hauteur de ligne attendue en pt (taille et interligne du style) ;
- `side` : `"before"` ou `"after"`, posé par `fields-stack`.

### `fields-stack`

Un enfant de `stack`, même un `place()` de taille nulle, reçoit le
`spacing` (mesuré : 34,48 pt sans marqueur, 54,48 pt avec un marqueur en
item à part, 34,48 pt avec le marqueur collé à l'item suivant).

`fields-stack(spacing:, ..items)` (dans `widget.typ`, exporté) fusionne
chaque item vide (un marqueur seul) en tête de l'item suivant, avec
`side: "before"`. Un marqueur en dernière position passe en queue du
précédent, avec `side: "after"`. L'ancre d'un champ vide est donc le haut
(ou le bas) de son voisin : `y` de cet item, `x`/`ax`/`aw` de la zone
disponible.

La carte (`slides.typ`, `card`), `_slide-title`, `title-slide`, `chapter`
et `end-slide` remplacent leurs `if x != none { wrap(…) }` par un `wrap`
inconditionnel passé à `fields-stack`. Le README de `typst/oku-slides`
documente la règle pour les packs.

### `FieldBox`

`oku_slides_render::FieldBox` gagne :

```rust
pub empty: bool,              // #[serde(default)]
pub side: Option<FieldSide>,  // Before | After ; seulement si `empty`
pub line: Option<f64>,        // hauteur de ligne attendue (pt)
```

## Session

### `Target`

`views::Target` redevient une boîte d'objet : `id`, `kind`
(`Element(kind)` ou `Slide`), `free`, `x`, `y`, `w`, `h`. Il perd `param`,
`text`, `ax`, `aw`, `style`. `Target::field` et les ids `@<paramètre>`
disparaissent.

### `FieldView`

```rust
pub struct FieldView<'a> {
    pub owner: &'a str,           // id de la slide ou de l'élément
    pub owner_kind: OwnerKind,    // Slide | Element
    pub field: &'a str,           // nom du paramètre
    pub kind: FieldKind,          // Plain | Rich
    pub label: String,            // placeholder, localisé
    pub text: Cow<'a, str>,       // texte tel que saisi (bind résolu)
    pub empty: bool,
    pub side: Option<FieldSide>,
    pub line: Option<f64>,
    pub x: f64, pub y: f64, pub w: f64, pub h: f64,
    pub ax: f64, pub aw: f64,
    pub style: Option<&'a TextStyle>,
}
```

`SlideView.fields: Vec<FieldView>` est rangé dans l'ordre de Tab. Le
parcours d'ordre de lecture (slide, puis slots en profondeur) est une
fonction du core, à côté de `Slide::locate`, réutilisable par l'agent et le
panneau. Le texte d'un `plain` vient de `field_text` (qui résout `bind`),
celui d'un `rich` de `paragraphs_to_text`.

Un `FieldBox` sans paramètre `plain` ni `rich` du même nom dans le schéma
de son `owner` est ignoré (`tracing::debug!`).

### Libellés

Le placeholder vient, dans l'ordre : de la clé Fluent
`widget-param-<widget>-<param>` ; du `label` du schéma ; du nom du
paramètre. Les clés des widgets de base sont ajoutées dans les 20 langues.

### Fusion des étapes d'annulation

`Request::Apply` gagne `merge: Option<String>`, une clé de session de
saisie créée par l'UI. Dans `replica/undo.rs`, une application de clé `k`
s'ajoute à l'étape du sommet si cette étape porte la clé `k` **et**
qu'aucune autre étape n'a été empilée depuis ; sinon, elle ouvre une
nouvelle étape, marquée `k`. Le libellé de l'étape fusionnée devient
`ui-undo-edit-slide`.

La fusion n'utilise pas `begin_group` / `end_group` : un groupe ouvert par
l'UI pourrait avaler un tour d'agent ou rester ouvert si la fenêtre se
ferme. La clé rend chaque `Apply` autonome. Un `Apply` intercalé (agent,
autre geste) coupe la fusion : ⌘Z ne défait jamais une saisie et un
changement d'agent ensemble.

## UI

### Couches de la scène (`Stage.tsx`)

1. SVG rendu.
2. Couche objets (`targets`) : sélection, glisser, poignées, étiquette. Le
   tri par aire (`ordered`) disparaît.
3. Couche champs (`fields`), au-dessus : zones de clic transparentes. Un
   champ vide a la hauteur de sa `line`, au-dessus de l'ancre (`before`) ou
   au-dessous (`after`). Placeholder grisé, à la typographie du champ,
   dessiné pour les champs vides de l'objet sélectionné ou en édition ;
   partout en mode modèles.
4. Éditeur : un `TextEditor`, `key` = `owner/field`, et son masque.

L'historique ouvert (lecture seule) ne monte pas la couche champs.

### Gestes

| Geste | Effet |
|---|---|
| Clic (hors édition) | sélectionne l'objet, même sur un champ |
| Double-clic sur un champ | l'édite |
| Double-clic sur un objet, hors champ | édite son premier champ (ordre de Tab) |
| Clic sur un autre champ (en édition) | valide, y passe, même session |
| Clic hors champ (en édition) | valide et sort |
| Tab / Shift-Tab | valide, champ suivant / précédent ; au bout, valide et sort |
| Entrée (`plain`) | valide, champ suivant |
| ⌘Entrée | valide et sort |
| Échap | annule le champ courant et sort |

### Enchaînement

1. À l'entrée en édition, l'UI crée une clé de session.
2. En quittant un champ : si le brouillon diffère du texte d'origine,
   `onApply(label, textOps, merge)` (`set_param`).
3. Le champ visé est retenu par sa clé `(owner, field)`, pas par un index.
4. Si un `Apply` est parti, l'éditeur visé s'ouvre au `SlideView` suivant,
   sur des boîtes à jour. Sinon, tout de suite. Entre-temps, la frappe est
   mise en tampon puis rejouée dans le nouvel éditeur.
5. À la sortie, la clé de session est oubliée.

### Géométrie

`editorGeometry` part toujours d'un `FieldView`. `plain` : largeur
naturelle du brouillon, bornée par `aw`, alignée dans `ax`/`aw`. `rich` :
largeur `w`, retour à la ligne. Première ligne sur la ligne de base Typst
(hauteur de capitale), comme aujourd'hui pour les champs de slide. La
branche « sans style » disparaît.

## Cas limites

- **Champ disparu pendant l'édition** (suppression par l'agent, fusion,
  `SetKind`) : le brouillon est commité si l'`owner` existe encore et
  déclare toujours le paramètre ; sinon il est perdu et un toast le dit
  (`ui-edit-field-gone`). L'éditeur se ferme.
- **Édition concurrente du même champ** : fusion Loro au caractère (`Prose`).
- **Champ lié** (`bind`) : écrit dans `deck.params` ; les slides qui le
  montrent se re-rendent.
- **Champ débordant** son objet : édité à sa position ; la scène ne rogne
  pas l'éditeur.
- **Canevas libre** : l'ordre de Tab reste celui du modèle.
- **Dictée** : insérée dans le champ ouvert, sans changement.
- **Pack tiers sans `fields-stack`** : ses champs vides émettent une boîte
  sans ancre fiable ; rien ne le détecte automatiquement, la règle est
  documentée dans le README.

## Hors périmètre

- Masquage des marques markdown, style par run (t-272).
- Édition sur la scène des paramètres non textuels (`flag`, `choice`,
  `number`…) : inspecteur.

## Tests

En TDD, dans l'ordre de construction :

1. **Typst** : un `field()` vide émet une boîte `empty` sans changer la
   hauteur d'un `fields-stack` ; une carte sans surtitre émet trois
   `FieldBox`, `kicker` vide ancré `before` le titre ; le test doré reste
   identique.
2. **Core** : ordre de lecture sur une slide à `stack` imbriqué.
3. **Undo** : deux `Apply` de même clé → une étape, un `undo` les défait ;
   un `Apply` intercalé coupe la fusion ; une clé différente ouvre une
   étape.
4. **Session** : `SlideView.fields` dans l'ordre de Tab ; champ vide avec
   son libellé localisé ; `bind` résolu ; `FieldBox` orphelin ignoré.
5. **UI, tests purs** (`stageFields.test.ts`) : navigation par clé, bords,
   tampon de frappe, choix du premier champ d'un objet.
6. **UI, rendu** : double-clic sur le titre d'une carte → éditeur sur le
   titre ; Tab → éditeur sur le corps après le `SlideView` suivant.
7. **Alignement** (Playwright, polices du thème chargées) : sur les
   fixtures, pour chaque champ `plain` et chaque `rich` sans marque,
   l'éditeur ouvert superposé au SVG : bord gauche et ligne de base de
   chaque ligne à ≤ 1 px à l'échelle 1, retours à la ligne identiques.
