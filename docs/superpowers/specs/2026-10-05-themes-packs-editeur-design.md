# Thèmes en packs versionnés et éditeur de jetons — design

Date : 2026-10-05 · Tâche : t-554

## Objectif

Les thèmes `papier` et `pro` ne sont plus codés en dur : ce sont des thèmes
intégrés d'une **bibliothèque de thèmes versionnée**, jumelle de celle des
packs de modèles (t-328). Un deck référence un thème dans une version et
l'embarque. La perspective modèles gagne un onglet **Thème** qui édite tous
les jetons de design : couleurs, polices, tailles, graisses, interlignage,
espacements, rayons, traits, page, palette des graphiques, composants,
couleurs de l'interface, repli CJK.

Deux phases dans un même plan : 1. thèmes en packs (aucun changement visible
hors du menu des thèmes) ; 2. éditeur.

Hors périmètre : import/export d'un thème seul, polices embarquées dans un
thème (elles restent dans `fonts/` du deck), ménage des versions, bandeau
« version plus récente du thème » (la relation est calculée, l'affichage
viendra avec une tâche dédiée si besoin).

## Décisions

- **Un thème est un fichier à part.** Les packs de modèles restent
  indépendants du thème (« Un ensemble ne dépend pas du thème »).
- **Même mécanique que les packs.** Id UUID v4, versions adressées par
  contenu (`sha256:<hex>`), une version courante par thème, parente de
  version (généalogie seule, sans effet sur le rendu), intégrés en lecture
  seule et d'id constant. Le cache fait foi au rendu : un deck se rend avec la
  version qu'il embarque.
- **`Library` générique** sur une sorte (`LibraryKind`) : modèles et thèmes
  partagent stockage, versions, `current`, `relation`, intégrés (G11).
- **Une version de thème est un instantané complet.** Pas d'héritage entre
  thèmes : la fusion reste `default-tokens ← thème ← deck.tokens`. Une copie
  recopie tous les jetons ; `parent` n'en garde que la généalogie.
- **Le JSON est la seule source de vérité des jetons.** `default-tokens` et
  les thèmes intégrés deviennent des fichiers JSON du paquet Typst, lus par
  Typst (`json(...)`) et embarqués par Rust (`include_str!`).
- **Enregistrer un thème intégré crée une copie** (nouvel id, nom « <nom>
  (copie) », parente = la version intégrée). Un thème de l'utilisateur gagne
  une version, qui devient courante. Le deck mis de côté adopte la version
  enregistrée.
- **Panneau des jetons à droite** de la scène, onglet « Thème » fixe après
  « Modèles ».
- **Pas de rétrocompatibilité** : `DECK_VERSION` 3 → 4, `SOURCES_VERSION`
  5 → 6 ; les anciens decks et zips ne sont plus lus.

## Modèle (core)

```rust
pub struct ThemeRef { pub id: Uuid, pub version: PackVersion }
pub struct Deck { …, pub theme: Option<ThemeRef>, … }   // était Option<String>
```

- `PackVersion` et `PackFiles` servent aux deux sortes. Une version de thème
  est un `PackFiles` dont le manifeste est `theme.json` et qui n'a aucun
  fichier de widget : même contenu canonique, même empreinte.
- `theme: None` : le thème par défaut (`papier`, id constant), dans sa version
  courante, embarqué et référencé au chargement (`adopt_default_theme`,
  comme `adopt_default_pack`).
- Dans le replica Loro, `theme` est un atome (dernier écrivain gagne), comme
  `templates`.
- **Ops.** `SetTheme { theme: Option<ThemeRef> }`. `SetToken { path, value }`
  pose `value`, `null` compris (« vider » une clé qui l'accepte) ; nouvelle
  `UnsetToken { path }` retire la clé (retour à la couche inférieure). Le
  schéma MCP et l'agent suivent.

### `theme.json`

```json
{ "id": "<uuid>", "name": "Papier", "description": "", "parent": null,
  "created": "2026-10-05T00:00:00Z", "tokens": { "color": { … }, … } }
```

`ThemeManifest { id, name, description, parent, created, tokens: Map }`
(session), à côté de `PackManifest`.

## Fichier (zip des sources, version 6)

```
themes/<hex>/theme.json
```

- `SourceBundle.themes: BTreeMap<PackVersion, PackFiles>`, en ajout seul,
  unie par `merge`, gardée si référencée par le deck ou son historique.
- À la lecture, une version dont l'empreinte ne correspond pas est seule
  écartée (`Issue::ThemeVersionMismatch`) ; la version du deck absente donne
  `Issue::ThemeVersionMissing`, et le deck se rend avec le thème par défaut.
  Un chemin `themes/` invalide (dossier autre que 64 hex minuscules,
  sous-dossier, fichier autre que `theme.json`) refuse tout le zip.

## Bibliothèque (session)

```text
<data>/themes/<uuid>/current
<data>/themes/<uuid>/versions/<hex>/theme.json
```

- `Library<K: LibraryKind>` : `K::DIR` (`templates` / `themes`),
  `K::MANIFEST_FILE` (`pack.json` / `theme.json`), `K::Manifest` (trait
  `VersionManifest` : `id`, `name`, `parent`, `created`, `from_files`),
  `K::builtins()`. Alias : `PackLibrary`, `ThemeLibrary`.
- Intégrés : `PAPIER_THEME` (défaut) et `PRO_THEME`, UUID constants.
- `ThemeInfo { id, version, name, builtin, default }` pour la liste.

## Rendu

- `typst/oku-slides/src/default-tokens.json` ; `tokens.typ` :
  `#let default-tokens = json("default-tokens.json")`.
- `typst/oku-slides/themes/papier.json`, `pro.json` (le `theme.json` complet) ;
  `lib.typ` : `#let papier = json("themes/papier.json").tokens` (usage
  `theme: papier` du paquet Typst inchangé). `papier.typ`, `pro.typ`,
  `THEMES` disparaissent.
- `Renderer::set_theme(Option<serde_json::Map>)` (et `Request::SetTheme` du
  worker) : le monde sert `/theme.json`, la source principale passe
  `theme: json("/theme.json")` ou `none`.
- Police des graphiques : lue dans les jetons résolus (`font.body`, première
  famille), en second passage comme la palette ; `charts.rs::body_font`
  disparaît.

## Session

- `Session::deck_theme()` : la version embarquée ; `embed_theme_from_library`
  comme `embed_from_library`. À chaque changement de version, le renderer
  reçoit `set_theme`.
- Snapshot : `theme: Option<ThemeRef>`, `themes: Vec<ThemeInfo>`.
- Requête `set_theme { id }` : embarque la version courante de `id` et applique
  `SetTheme` (annulable).

## Éditeur (phase 2)

### Mode modèles

- À l'entrée, le deck de travail a `theme: None` et `tokens` = **tous les
  jetons du thème édité** (le thème du deck mis de côté). Les surcharges
  propres au deck ne s'y appliquent plus : les modèles se voient avec le thème
  pur, comme dans tout deck qui l'utilise.
- `Mode::Templates` garde `theme_edit: ThemeEdit { base: ThemeRef, name }`.
- Chaque réglage est un `SetToken` / `UnsetToken` sur le deck de travail :
  rendu, miniatures, apparence de l'app et annuler/rétablir suivent d'office.
- Choisir un autre thème dans l'onglet : `ThemeEdit.base` change et les jetons
  du deck de travail sont remplacés en une étape annulable (`SetTokens {
  tokens }`, nouvelle op qui remplace tout `deck.tokens`).
- À l'enregistrement (`save_templates`), si les jetons ou le nom diffèrent de
  la version de base : nouvelle version de thème (copie pour un intégré),
  embarquée, courante, et le deck mis de côté passe à ce `ThemeRef`. Sinon, si
  `base` diffère du thème du deck, le deck adopte `base`.
- Fermer sans enregistrer abandonne le brouillon, comme pour les modèles.

### Schéma des jetons

`crates/oku-slides-session/src/theme_schema.json` : pour chaque chemin, un
contrôle et ses contraintes.

| Contrôle | Chemins | Valeur |
|---|---|---|
| `color` | `color.*`, `app*.*` couleurs, composants couleur | `#rrggbb[aa]` ou nom d'un jeton `color` |
| `pt` | `size`, `space`, `radius`, `stroke`, `page` (sauf `width`/`height`), composants dimension | nombre borné, ou nom d'un jeton de la catégorie référencée |
| `em` | `leading.*` | nombre 0 à 2 |
| `weight` | `weight.*`, composants graisse | 100 à 900 par 100, ou nom d'un jeton `weight` |
| `fonts` | `font.*` | liste ordonnée de familles |
| `choice` | `script-style.*`, `app.scheme`, `layout`, `case`… | une des valeurs listées |
| `bool` | `bar`, `rule`… | booléen |
| `text` | gabarits (`footer.left`…) | chaîne |
| `palette` | `chart.palette` | liste de couleurs ou de noms de jetons `color` |
| `number` | `opacity`, `ratio`, `tracking`, `elevation` | nombre borné |

Chaque entrée a `nullable` (« vider » permis) et une catégorie d'affichage.
`page.width` et `page.height` ne sont pas éditables (grille 1920 × 1080).
Les couleurs propres à un thème (`brand-ink`…) sont éditables comme `color.*`
et on peut en ajouter ou en retirer (nom `[a-z][a-z0-9-]*`).

**Validation** côté session à chaque `SetToken`/`UnsetToken` en mode
modèles : valeur conforme au contrôle, référence vers un jeton existant de la
bonne catégorie, retrait refusé si d'autres jetons y renvoient (le message les
nomme). Hors mode modèles, la validation des couleurs s'applique aussi aux
surcharges du deck (règle t-194).

**Couverture** : un test vérifie que le schéma et `default-tokens.json` ont
exactement les mêmes chemins (hors couleurs libres de `color.*`).

### Surface `host-theme`

Construite en Rust (`theme_panel.rs`, à côté de `panel.rs`), rendue par
`HostSettings` avec les contrôles A2UI existants (`Color`, `Stepper`,
`Slider`, `Select`, `Text`, `List`), complétés au besoin. Une section par
catégorie (Couleurs, Typographie, Espacements, Formes, Page, Graphiques,
Composants — un sous-groupe par composant —, Interface clair, Interface
sombre, Écritures CJK). Chaque ligne montre son état :

| État | Sens | Action |
|---|---|---|
| Par défaut | absent du brouillon, vient de `default-tokens` | modifier → la clé entre |
| Défini | dans le brouillon, égal à la version de base | « Retirer du thème » (`UnsetToken`) |
| Modifié | différent de la version de base | ↺ → valeur de la version de base |

### UI

- `TemplateTabs` : onglet fixe « Thème » après « Modèles ».
- Onglet actif : pile des modèles à gauche, scène du modèle choisi au centre,
  panneau `ThemePanel` à droite (en-tête : sélecteur de thème, nom, champ de
  filtre ; puis la surface `host-theme` en sections repliables).
- Le sélecteur de thème du menu ⋯ disparaît du mode modèles.
- Libellés : Fluent, `ui-theme-*` (interface) et `theme-token-*` (libellés de
  la surface, côté session) dans les 20 langues.

## Erreurs

Snafu, messages traduits : `UnknownTheme`, `ThemeVersionMissing`,
`ThemeIdMismatch`, `ThemeJson`, `InvalidTokenValue { path, reason }`,
`TokenInUse { path, users }`, `NoThemesDir`. Une version illisible de la
bibliothèque est ignorée avec un `warn` ; les intégrés restent toujours
disponibles.

## Tests

- Core : `ThemeRef` sérialisé, contenu canonique d'un thème, aller-retour du
  zip `themes/`, chemins invalides, `ThemeVersionMismatch`, `SetToken null`,
  `UnsetToken`, `SetTokens`, replica Loro (atome `theme`, jetons `null`).
- Session : `Library` générique (tests existants des packs + jumeaux thèmes),
  adoption du thème par défaut, `set_theme`, enregistrement en mode modèles
  (copie d'un intégré, version d'un thème utilisateur, rien si inchangé,
  abandon à la fermeture), schéma (couverture, validation, `TokenInUse`),
  surface `host-theme` (états par défaut/défini/modifié).
- Rendu : snapshots golden de `papier` et `pro` **inchangés** après le passage
  en JSON ; régression : police des graphiques lue dans `font.body`.
- UI : rendu de `ThemePanel` et de l'onglet ; e2e (données isolées) : modifier
  `color.accent` de Papier, voir la scène changer, enregistrer, « Papier
  (copie) » existe et le deck l'utilise.
- i18n : `i18n-keys.test.ts` (en/fr), 18 autres langues complétées.
