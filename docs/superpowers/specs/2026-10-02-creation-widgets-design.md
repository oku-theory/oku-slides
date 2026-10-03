# Création de widgets par l'agent et éditeur Typst — design

Date : 2026-10-02 · Tâche : t-329

## Objectif

L'utilisateur fait créer ou modifier des widgets par l'agent (« fais-moi un
widget KPI »), ou les écrit lui-même dans un éditeur Typst. Un widget est un
fichier Typst d'un pack de modèles ; il voyage avec le deck (t-328) et
s'exécute dans le processus de rendu borné (t-327).

Programme « widgets portables » : 0. compilation bornée (t-327, livrée) ;
1. packs portables (t-328, livrée) ; 2. **création de widgets** (cette
spec).

Hors périmètre : autocomplétion et LSP Typst, renommage d'un widget déjà
utilisé dans des decks (on renomme dans la version éditée ; les decks
gardent la leur), partage de widgets entre packs.

## Décisions

- **Un fichier par widget** dans une version de pack, plus un fichier
  partagé facultatif `_commun`. L'app génère le `registry`.
- **Disponible en mode modèles et en mode deck.** Modifier un widget
  modifie le pack de l'utilisateur, quel que soit l'endroit : la version
  produite devient sa version courante.
- **Une version par séance** (règle de t-328) : des modifications de widget
  successives remplacent la même version ; la séance se termine en quittant
  le mode modèles, ou, en mode deck, à la première opération qui n'est pas
  une modification de widget.
- **Aperçu dédié** `preview_widget` : le widget seul sur une slide
  temporaire, sans rien ajouter au pack.
- **Erreurs isolées** : chaque widget se compile à part ; un widget cassé
  est écarté du registre avec ses diagnostics, les autres restent.
- **Rupture de format** : `SOURCES_VERSION` passe à 5 (`widgets/<nom>.typ`
  remplace `widgets.typ`) ; pas de rétrocompatibilité.

## Format du pack (core)

```rust
pub struct PackFiles { pub pack_json: Vec<u8>, pub widgets: BTreeMap<WidgetFile, String> }
pub struct WidgetFile(String);   // "[a-z][a-z0-9-]*" ou "_commun"
```

- Le nom du fichier est le nom du widget ; `_commun` n'est pas un widget.
- **Contenu canonique** : `pack.json` canonique (t-328), octet `0`, puis
  pour chaque fichier dans l'ordre des noms : nom, `0`, source, `0`. Un
  fichier vide vaut absent. `PackVersion` est le sha-256 de ce contenu.
- Zip des sources (v5) : `packs/<hex>/pack.json`,
  `packs/<hex>/widgets/<nom>.typ`. Bibliothèque :
  `<uuid>/versions/<hex>/widgets/<nom>.typ`. Un nom de fichier invalide
  sous `packs/<hex>/widgets/` refuse le zip (`BadName`), comme un chemin
  `packs/` invalide.
- Limites (`[render]`) : `max_widget_kib = 256` par fichier,
  `max_widgets = 64` par version ; les fichiers comptent dans
  `max_sources_mib`.

## Assemblage et sonde (render, render-worker)

- Chaque `widgets/<nom>.typ` exporte la liaison `widget`, résultat de
  `widget("<nom>", …)`. Il peut importer `"_commun.typ"` et la bibliothèque
  `oku-slides`. Le monde Typst sert `/packs/<hex>/widgets/<nom>.typ`.
- L'hôte génère un module d'assemblage qui importe chaque widget et forme
  `registry` ; `_commun` n'y entre pas.
- **Sonde par fichier**, dans le processus de rendu : chaque widget est
  compilé seul ; le résultat est, par fichier, son schéma ou ses
  diagnostics (fichier, ligne, colonne, message). Le registre du pack ne
  contient que les widgets valides. Un widget dont le nom déclaré diffère
  du nom de fichier est refusé avec un diagnostic. Le cache des sondes est
  indexé par (nom, hash du fichier, hash de `_commun`).
- Un délai ou un dépassement de mémoire pendant la sonde d'un fichier donne
  un diagnostic pour ce fichier. Pendant l'édition, le pack n'est pas
  écarté (on le corrige) ; l'écartement de t-327 reste celui du rendu du
  deck.

## Session

- `WidgetStatus { name, schema: Option<WidgetSchema>, diagnostics }`,
  `widget_status() -> Vec<WidgetStatus>` pour le pack en cours.
- Opérations (mode modèles et mode deck) :
  - `set_widget(name, source)` : crée ou remplace un fichier ; rend le
    statut de ce widget et celui des widgets que la modification a fait
    passer de valides à cassés (un `_commun` modifié).
  - `remove_widget(name)` : refusé si des modèles du pack l'utilisent
    (liste des modèles dans l'erreur).
  - `widget_source(name)`.
- **En mode modèles** : les opérations modifient le pack édité ; la version
  de la séance suit la règle de t-328 (première modification : nouvelle
  version, `parent` = courante au début de la séance, rendue courante ;
  suivantes : `replace_version`).
- **En mode deck** : la première modification crée une version du pack du
  deck (`parent` = version embarquée), l'ajoute à la bibliothèque, la rend
  courante, l'ajoute au bundle et fait passer le deck dessus par un
  `SetTemplates` ; les suivantes de la même séance remplacent cette version
  (`replace_version`, nouveau `SetTemplates`), fusionnées en une étape
  d'annulation (clé `templates-adopt:<id>:<parent>`). Annuler fait revenir
  le deck à sa version d'avant ; la bibliothèque garde la version créée
  (comme pour un enregistrement en mode modèles).
- `preview_widget(name, params)` : slide temporaire dans un emplacement
  dédié du processus de rendu, thème et jetons du deck ; paramètres
  manquants : `default` du schéma, sinon un exemple localisé (`plain`,
  `rich`) ou la valeur neutre du type. Rend PNG (agent) ou SVG (UI) et les
  diagnostics. Un délai dépassé donne un diagnostic.

## Agent (MCP)

Outils, disponibles dans les deux modes :

| Outil | Rôle |
|---|---|
| `widget_sources` | fichiers du pack, avec leur statut |
| `widget_source(name)` | source d'un fichier |
| `set_widget(name, source)` | crée ou remplace ; rend le statut et les widgets cassés |
| `remove_widget(name)` | supprime (refusé s'il est utilisé par un modèle) |
| `preview_widget(name, params?)` | image PNG du widget seul |

Le prompt de l'agent gagne une section « Créer un widget » : forme d'un
fichier (imports, `widget("<nom>", params: …, (args, ctx) => …)`,
`field(ctx, …)` pour les champs éditables, `fields-stack`), descripteurs
`p.*`, jetons du thème (`color-of`, `size-of`) plutôt que des valeurs en
dur, `_commun` pour le code partagé, et la boucle écrire → statut → aperçu →
corriger. Un widget exemple est fourni dans le README de `typst/oku-slides`.
Insérer le widget dans le deck reste un `add_element` ordinaire.

## Éditeur Typst (UI)

- Panneau **Widgets**, accessible en mode modèles et en mode deck : liste
  des fichiers (`_commun` en tête, libellé « Commun ») avec une pastille
  d'état (valide, erreurs) ; actions Nouveau, Renommer (dans la version
  éditée), Supprimer.
- Éditeur CodeMirror (déjà présent) du fichier choisi, coloration Typst
  minimale (commentaires, chaînes, mots-clés, `#`, `p.*`), diagnostics en
  soulignés et en marge à leur ligne et colonne.
- Aperçu à droite, mis à jour 400 ms après la dernière frappe, avec ses
  diagnostics.
- La frappe passe par `set_widget` ; l'agent et l'éditeur utilisent les
  mêmes opérations et se voient l'un l'autre.

## Erreurs

- core : nom de fichier invalide → `BadName` à la lecture du zip ;
  `WidgetFile` refuse un nom invalide.
- session : `InvalidWidgetName { name }`, `WidgetTooLarge { name, max }`,
  `TooManyWidgets { max }`, `WidgetInUse { name, templates }`,
  `UnknownWidget { name }` (Snafu, backtrace, minuscules).
- Fluent, 20 langues : ces erreurs, les exemples de l'aperçu, l'UI du
  panneau.

## Dépôt

- Tests et fixtures de t-328 qui utilisent `widgets.typ` réécrits en
  `widgets/<nom>.typ`. Les packs intégrés n'ont pas de widgets.
- README de `typst/oku-slides` : forme d'un fichier widget, `_commun`,
  exemple.

## Tests

Dans l'ordre de construction, en TDD :

1. **Core** : contenu canonique multi-fichiers (ordre des noms, fichier
   vide = absent), vecteur sha-256 figé ; zip v5 : `widgets/<nom>.typ` lu
   et vérifié, nom invalide refusé ; `WidgetFile` accepte `kpi`, `_commun`,
   refuse `Kpi`, `../x`, `_autre`.
2. **Render** : registre assemblé égal à celui d'un `widgets.typ`
   équivalent écrit à la main ; widget cassé écarté, les autres gardés ;
   `_commun` importé, absent du registre ; nom déclaré ≠ nom de fichier
   refusé ; cache de sonde : modifier un widget ne recompile que lui,
   modifier `_commun` recompile tous les widgets (t-390 : le hash de
   `_commun` entre toujours dans la clé, l'import n'est plus détecté au
   texte).
3. **Session** : mode modèles : `set_widget` ×2 puis enregistrement → une
   version avec le fichier ; mode deck : `set_widget` ×2 → une version
   courante, le deck y passe, une seule étape d'annulation, annuler rend la
   version d'avant ; `remove_widget` refusé si un modèle l'utilise ;
   `preview_widget` remplit les défauts ; widget qui boucle dans l'aperçu →
   diagnostic, pack non écarté ; `_commun` cassé → `set_widget` rend les
   widgets cassés.
4. **MCP** : les cinq outils dans les deux modes ; réponse de `set_widget`
   avec diagnostics et widgets cassés.
5. **UI** : liste et états ; diagnostics à la bonne ligne ; aperçu après le
   délai de frappe (bun).
6. **Régression** : un widget qui ne compile pas n'empêche pas les autres
   widgets du même pack de se rendre dans le deck.

## Décisions d'implémentation

- `rename_widget` existe dans la session (`widgets_edit.rs`) mais n'est pas
  exposé à MCP : l'agent écrit un nouveau widget puis retire l'ancien.
- En mode modèles, une modification de widget enregistre immédiatement la
  version de séance (`adopt_session_version`) ; pas de « enregistrer » séparé.
- Un widget cassé du pack embarqué est écarté et signalé par l'avis
  `session-widgets-broken` ; les widgets valides restent rendus.
- Un widget d'élément est prévisualisé sur une diapositive vierge, dans un
  emplacement d'essai qui n'accuse jamais le pack.
- Mode deck (revue finale) : la version de la séance est embarquée et le
  deck rendu (`apply`) avant toute écriture dans la bibliothèque. Un échec,
  y compris un corps de widget qui échoue au rendu, ne change ni la
  bibliothèque, ni la séance, ni le deck ; `set_widget` rend alors les
  diagnostics du rendu dans le statut (`WidgetChange`), pas une erreur. Une
  écriture dans la bibliothèque qui échoue après le rendu est journalisée et
  pose l'avis `session-widgets-not-saved` (t-390) : le deck embarque déjà
  la version, la bibliothèque ne l'a pas.
- Mode modèles (revue finale) : choix « diagnostics + succès ». La version
  est enregistrée, puis le document des modèles rendu ; si ce rendu échoue,
  la version reste et ses diagnostics sont rendus avec les statuts, comme un
  modèle fautif. `adopt_session_version` ne rend plus : l'appelant rend.
  `save_templates` suit le même choix (t-390) : la version adoptée, un
  rendu du document des modèles qui échoue ensuite n'est pas une erreur ;
  l'enregistrement réussit et un avis (`set_notice`) donne la cause.
- Garde no-op : une écriture dont les fichiers de widgets sont identiques à
  ceux de la version en cours n'enregistre rien (les deux modes).
- Croissance du bundle : la version de séance remplacée quitte les sources
  du document (et, en mode modèles, celles du deck mis de côté) **seulement
  si l'écriture a rejoint l'étape d'annulation qui l'avait introduite** :
  `SourceBundle::undo_steps` (étapes empilées, jamais décrémenté) est
  inchangé par l'application. Sinon (nouvelle étape, par exemple après le
  groupe d'un tour de l'agent), annuler y revient : elle reste dans les
  sources (ajout seul) et dans la bibliothèque — en mode deck, la nouvelle
  version y est ajoutée sans `replace_version` ; en mode modèles (bibliothèque
  écrite d'abord), l'ancienne y est réajoutée. L'historique qui montrerait
  une étape fusionnée retombe sur l'avis « pack manquant ». Choisi plutôt
  que « ne pas embarquer avant la fin de séance », qui laisserait le deck
  référencer une version absente de ses sources.
- La séance de widgets du mode deck finit aussi à `begin_turn` et
  `end_turn` : elle ne chevauche jamais l'étape d'un tour de l'agent. En
  mode modèles, un tour commencé en mode modèles groupe le document des
  modèles, pas le deck mis de côté : ses enregistrements s'y fusionnent par
  clé. Correction (t-390) : un tour commencé en mode deck groupe le deck,
  qui passe de côté si le tour entre en mode modèles ; `end_turn` ferme ce
  groupe. Un enregistrement après le tour fait alors une nouvelle étape : le
  chemin non fusionné est atteignable (la version du tour reste dans les
  sources et est réajoutée à la bibliothèque ; annuler y revient).
- Octet NUL : une source de widget qui en contient un est refusée, à la
  lecture (`PackFiles::from_bytes` → `None` : zip, bibliothèque, reprise
  l'écartent) et à l'écriture (`set_widget` → `WidgetHasNul`, Fluent
  `session-widget-nul`). Le `pack.json` canonique (JSON compact) échappe
  déjà NUL : rien à refuser de ce côté.
- Sonde d'un pack (revue finale) : budget total `timeout_ms` par appel de
  `probe_pack`. Épuisé, les fichiers restants hors cache ne sont pas
  exécutés (diagnostic localisé `probe-*`, dans la langue des emplacements
  du travailleur, comme les autres diagnostics de limite) et ne sont pas
  gardés : ils sont sondés au prochain appel (`PackProbe::unprobed` les
  nomme ; un échec de préparation du lien y range aussi le fichier). Tant qu'il en reste pour le pack installé du document ouvert,
  chaque rendu du deck résout de nouveau ses widgets, un budget par rendu,
  (t-390). Un fichier seulement non sondé n'est pas dit cassé : l'avis
  `session-widgets-broken` l'omet, et aucun autre avis ne le signale (il
  est sondé au rendu suivant, l'avis serait aussitôt périmé). La reprise
  qui le juge cassé pose l'avis, réduit aux fichiers jusque-là non sondés.
  Un fichier dont la sonde dépasse le délai est
  gardé cassé : il ne compte plus parmi les non sondés, pas de boucle sans
  fin. `RenderWorker::probe` (sonde
  du pack entier, sans isolation) est retiré ; ses tests passent par
  `probe_pack` ou `set_widgets`.
