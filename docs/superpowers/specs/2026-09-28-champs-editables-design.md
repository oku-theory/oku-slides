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

- **Champs vides sans place au rendu.** Un `field()` vide émet un
  marqueur ; `fields-stack` en fait sa boîte (`empty`, `side`). Un champ
  vide ne change pas la mise en page : l'export reste identique. L'UI
  dessine le placeholder par-dessus.
- **Tab sur toute la slide, dans l'ordre du modèle** : les champs de la
  slide dans l'ordre du schéma, puis les éléments dans l'ordre des slots
  (profondeur d'abord), et pour chacun ses champs dans l'ordre du schéma.
  Shift-Tab revient en arrière. L'ordre ne dépend pas de la position à
  l'écran.
- **Un commit par champ, une étape d'annulation par session de saisie.**
  Chaque sortie de champ applique `set_param` et re-rend. Les applications
  d'une même session fusionnent en une étape d'annulation.
- **Double-clic pour entrer, clic pour passer.** Un clic sur un champ
  d'élément sélectionne son élément ; sur un champ de slide, il ne
  sélectionne rien. Un double-clic édite le champ visé. En édition, un
  clic sur un autre champ y passe.
- **L'éditeur recouvre exactement le texte rendu.** t-271 garantit la
  géométrie : origine, ligne de base de chaque ligne, largeur de retour à
  la ligne, typographie. Un champ `plain`, ou un `rich` sans marque,
  coïncide au pixel. Le texte qui suit une marque markdown glisse jusqu'à
  t-272.
- **Deux couches dans la vue** : `targets` (boîtes d'objets, pour
  sélectionner et déplacer) et `fields` (champs, pour éditer).
- **Sélection** : les ids `@<paramètre>` disparaissent de `targets`. Le
  protocole de l'agent les garde pour adresser un champ de slide
  (`ask_rewrite`, ancres des fiches) : l'UI les résout, en listant
  `@<champ>` pour chaque champ de la slide parmi les ancres possibles, et
  pose la fiche sous ce `FieldView` (ruling R5).
- **Champ principal d'un objet** : là où l'on écrit sans viser un champ
  (Entrée, double-clic hors champ, dictée hors édition, copie) : le premier
  champ non vide, sinon le premier `rich`, sinon le premier (ordre de
  Tab). Le surtitre d'une carte, souvent vide, ne passe pas avant le
  titre. La dictée dans un champ `rich` non vide s'ajoute à la suite.

## Typst

### `field()` et champs vides

`field(ctx, name, body, style:)` accepte `body == none` ou `""`. Un
`field()` vide n'émet pas de boîte : il émet un marqueur `metadata` sans
étiquette (`oku-empty-field: true, owner, field, style`), inerte hors de
`fields-stack`. `fields-stack` le transforme en boîte `<oku-slides-field>`
avec `empty: true`, `h: 0` et `side`. Un champ rendu mais vide
(`field(…, empty: true)`) garde sa boîte mesurée.

### `fields-stack`

Un enfant de `stack`, même un `place()` de taille nulle, reçoit le
`spacing` (mesuré : 34,48 pt sans marqueur, 54,48 pt avec un marqueur en
item à part, 34,48 pt avec le marqueur collé à l'item suivant).

`fields-stack(spacing:, ..items)` (dans `widget.typ`, exporté) colle
chaque marqueur à un item voisin : en tête de l'item suivant, avec
`side: "before"` ; en dernière position, en queue du précédent, avec
`side: "after"`. La boîte vide est posée hors flux, à la position
verticale courante, par `place(left, layout(…))` : le `layout` ne sert qu'à
lire la largeur disponible (`aw`). L'ancre d'un champ vide est donc le haut
(ou le bas) de son voisin : `y` de cet item, `x`/`ax`/`aw` de la zone
disponible. Pas de mesure des items, et pas de `layout` autour du `stack` :
envelopper le `stack` déplaçait le rendu d'une fraction de pixel (anticrénelage
du test doré).

Plusieurs marqueurs consécutifs (carte sans surtitre ni titre, slide de
contenu sans titre, couverture sans surtitre ni titre) partagent la même
ancre : même `y`, même `side`. L'UI les empile (voir Couches).

La carte (`slides.typ`, `card`), `_slide-title`, `title-slide`, `chapter`
et `end-slide` remplacent leurs `if x != none { wrap(…) }` par un `wrap`
inconditionnel passé à `fields-stack`. Le README de `typst/oku-slides`
documente la règle pour les packs.

### `FieldBox`

`oku_slides_render::FieldBox` gagne :

```rust
pub empty: bool,              // #[serde(default)]
pub side: Option<FieldSide>,  // Before | After ; seulement si `empty`
```

## Session

### `Target`

`views::Target` redevient une boîte d'élément : `id`, `kind: &str` (le
widget de l'élément ; pas de variante slide), `free`, `x`, `y`, `w`, `h`.
Il perd `param`, `text`, `ax`, `aw`, `style`. `Target::field` et les ids
`@<paramètre>` disparaissent de `targets` (l'agent les garde, voir
Décisions).

### `FieldView`

```rust
pub struct FieldView<'a> {
    pub owner: &'a str,           // id de la slide ou de l'élément
    pub owner_kind: OwnerKind,    // Slide | Element
    pub field: &'a str,           // nom du paramètre
    pub kind: FieldKind,          // Plain | Rich
    pub widget: &'a str,          // kind de l'objet
    pub label: Option<&'a str>,   // label du schéma (le libellé localisé
                                  // se calcule dans l'UI)
    pub text: Cow<'a, str>,       // texte tel que saisi (bind résolu)
    pub empty: bool,
    pub side: Option<FieldSide>,
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

Le placeholder suit la règle : `ui-field` (UI), puis `label` du schéma,
puis le nom. `ui-field` est complétée de `body`, `author`, `date`,
`organization` dans les 20 langues.

### Fusion des étapes d'annulation

`Request::Apply` gagne `merge: Option<String>`, une clé de session de
saisie créée par l'UI. Dans `replica/undo.rs`, une application de clé `k`
s'ajoute à l'étape du sommet si cette étape porte la clé `k` **et**
qu'aucune autre étape n'a été empilée depuis ; sinon, elle ouvre une
nouvelle étape, marquée `k`. L'étape fusionnée garde le libellé de sa
première application ; l'UI passe toujours `ui-undo-edit-slide`. Une étape
fusionnée grandit avec la séance ; Loro garde `limite × 64` items, et une
étape dont il a écarté des items part tout entière.

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
   champ vide a une ligne de haut (pas des lignes de son `TextStyle`, voir
   Géométrie), au-dessus de l'ancre (`before`) ou au-dessous (`after`).
   Les champs vides consécutifs d'un même objet sur la même ancre (même
   `y`, même `side`) s'empilent de haut en bas dans l'ordre de Tab :
   avant l'ancre, les premiers montent au-dessus des suivants ; après,
   les suivants descendent sous les premiers. Placeholder grisé, à la
   typographie du champ, dessiné pour les champs vides de l'objet
   sélectionné ou en édition ; partout en mode modèles. Les poignées de
   redimensionnement passent au-dessus de cette couche.
4. Éditeur : un `TextEditor`, `key` = `owner/field`, et son masque.

L'historique ouvert (lecture seule) ne monte pas la couche champs.

### Gestes

| Geste | Effet |
|---|---|
| Clic (hors édition) | sélectionne l'élément, même sur un de ses champs ; un champ de slide ne sélectionne rien |
| Double-clic sur un champ | l'édite |
| Double-clic sur un objet, hors champ | édite son champ principal (voir Décisions) |
| Entrée (élément sélectionné, hors édition) | édite son champ principal |
| Clic sur un autre champ (en édition) | valide, y passe, même session ; la sélection suit l'élément |
| Clic hors champ (en édition) | valide et sort |
| Tab / Shift-Tab | valide, champ suivant / précédent (la sélection suit l'élément) ; au bout, valide et sort |
| Entrée (`plain`) | valide, champ suivant |
| ⌘Entrée | valide et sort |
| Échap | annule le champ courant et sort |

### Enchaînement

1. À l'entrée en édition, l'UI crée une clé de session.
2. En quittant un champ : si le brouillon diffère du texte d'origine,
   `onApply(label, textOps, merge)` (`set_param`).
3. Le champ visé est retenu par sa clé `(owner, field)`, pas par un index.
4. L'éditeur visé s'ouvre tout de suite ; sa géométrie suit chaque
   `SlideView`, il se recale quand le rendu arrive.
5. À la sortie, la clé de session est oubliée.
6. Avant un changement de slide, la scène valide la saisie ouverte (poignée
   `StageHandle.commitEditing`), puis l'édition se ferme.

### Géométrie

`editorGeometry` part toujours d'un `FieldView`. `plain` : largeur
naturelle du brouillon, bornée par `aw`, alignée dans `ax`/`aw`. `rich` :
largeur de la zone (`aw`), celle où Typst revient à la ligne. Pas des
lignes, celui de Typst : hauteur de capitale de la police, mesurée par
police (`fontMetrics`, 0,7 em sans mise en page), plus l'interligne.
Première ligne sur la ligne de base Typst (hauteur de capitale), la ligne
de base CSS mesurée pour la police du champ. Sans `TextStyle`, l'éditeur
se pose sur la boîte cliquable, dans la police de l'interface.

## Cas limites

- **Champ disparu de la même slide pendant l'édition** (suppression par
  l'agent, fusion, `SetKind`) : rien n'est commité, le brouillon est perdu
  et un toast le dit (`ui-toast-field-gone`). L'éditeur se ferme.
- **Slide changée pendant l'édition** (focus de l'agent, vignette) : le
  brouillon est commité d'abord, avec la clé de la session, l'édition se
  ferme, puis la slide change. Pas de toast.
- **Édition concurrente du même champ** : fusion Loro au caractère (`Prose`).
- **Champ lié** (`bind`) : écrit dans `deck.params` ; les slides qui le
  montrent se re-rendent.
- **Champ débordant** son objet : édité à sa position ; la scène ne rogne
  pas l'éditeur.
- **Canevas libre** : l'ordre de Tab reste celui du modèle.
- **Dictée** : insérée dans le champ ouvert, sans changement.
- **Pack tiers sans `fields-stack`** : ses champs vides
  ne sont pas atteignables (le marqueur est inerte hors de
  `fields-stack`) ; rien ne le détecte automatiquement, la règle est
  documentée dans le README.

## Hors périmètre

- Masquage des marques markdown, style par run (t-272).
- Édition sur la scène des paramètres non textuels (`flag`, `choice`,
  `number`…) : inspecteur.
- `author` et `date` de la couverture (une seule ligne, inspecteur).

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
4. **Session** : `SlideView.fields` dans l'ordre de Tab ; champ vide listé
   avec `empty` et le label de son schéma ; `bind` résolu ; `FieldBox`
   orphelin ignoré.
5. **UI, tests purs** (`stageFields.test.ts`) : navigation par clé, bords,
   champ principal d'un objet, empilement des champs vides d'une ancre.
6. **UI, rendu** : double-clic sur le titre d'une carte → éditeur sur le
   titre ; Tab → l'éditeur s'ouvre sur le corps et se recale au
   `SlideView` suivant.
7. **Alignement** (Playwright, polices du thème chargées) : sur les
   fixtures, pour chaque champ `plain` et chaque `rich` sans marque,
   l'éditeur ouvert superposé au SVG : bord gauche et ligne de base de
   chaque ligne à ≤ 1 px à l'échelle 1, retours à la ligne identiques.
