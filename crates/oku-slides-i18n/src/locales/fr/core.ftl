op-slide-not-found = Slide introuvable : { $subject }
op-element-not-found = Élément introuvable : { $subject }
op-duplicate-id = Identifiant déjà utilisé : { $subject }
op-index-out-of-range = Position hors limites
op-invalid-value = Valeur invalide pour { $subject }
op-unknown-widget = Widget inconnu : { $subject }
op-unknown-param = Paramètre inconnu : { $subject }
op-param-type = Valeur invalide pour { $subject }
op-item-not-found = Aucun item { $subject }
op-list-full = La liste { $subject } est pleine ({ $max ->
   [one] { $max } item
  *[other] { $max } items
} au plus)
op-list-minimum = La liste { $subject } doit garder au moins { $min ->
   [one] { $min } item
  *[other] { $min } items
}
op-list-cell-too-long = Une cellule de la liste { $subject } est trop longue (colonne { $column })
op-list-id-too-long = Un identifiant d'item de la liste { $subject } est trop long
op-list-too-many-items = La liste { $subject } a trop d'items ({ $max } au plus)
op-slot-rejects = { $subject } n'est pas accepté ici
op-wrong-category = { $subject } ne peut pas être utilisé ici
op-choice-out-of-range = Valeur non permise pour { $subject }
transaction-refused = Opération { $index } refusée : { $reason }

sources-unreadable = Sources illisibles
sources-not-sources = Ce n'est pas un zip de sources Oku Slides
sources-unsupported-version = Ces sources viennent d'une version plus récente d'Oku Slides
sources-bad-name = Nom de fichier refusé dans les sources
sources-missing-deck = Le deck manque dans les sources
sources-too-large = Les sources de ce fichier sont trop volumineuses pour être ouvertes
sources-too-deep = Ce deck imbrique des éléments trop profondément

issue-empty-slide-id = identifiant de slide vide
issue-duplicate-slide-id = identifiant de slide en double
issue-duplicate-element-id = identifiant d'élément en double
issue-empty-element-id = identifiant vide
issue-cell-outside-grid = cellule hors de la grille
issue-empty-frame = cadre de taille nulle
issue-free-without-frame = élément libre sans cadre
issue-unknown-widget = widget inconnu : { $kind }
issue-wrong-category = { $kind } ne peut pas être utilisé ici
issue-unknown-param = paramètre inconnu : { $param }
issue-param-type = valeur invalide pour { $param }
issue-choice-out-of-range = valeur non permise pour { $param }
issue-slot-rejects = { $kind } n'est pas accepté ici
issue-placed-in-flow = élément placé avec un cadre dans une liste mise en page
issue-unknown-bind = { $param } est lié à un réglage du deck inconnu
issue-slot-too-deep = éléments imbriqués trop profondément
issue-missing-asset = asset manquant : { $asset }
issue-pack-version-mismatch = version de pack altérée, écartée : { $version }
issue-pack-version-missing = version de pack absente des sources : { $version }
issue-theme-version-mismatch = version de thème altérée, écartée : { $version }
issue-theme-version-missing = version de thème absente des sources : { $version }
issue-unsupported-link = lien non pris en charge : { $link }
issue-highlight-zero = les couleurs de pastille se numérotent à partir de 1
issue-unreadable-element = élément illisible, mis de côté
issue-unreadable-item = item de liste illisible, mis de côté
issue-unreadable-slide = slide illisible, mise de côté
issue-unreadable-field = champ { $field } du deck illisible, réinitialisé
op-invalid-chart = Graphique { $subject } : { $reason }
op-invalid-table = Tableau { $subject } : { $reason }

replica-corrupted = L'historique de la présentation dans ce fichier est abîmé
replica-too-large = L'historique de la présentation dans ce fichier est trop volumineux pour être ouvert
replica-write = L'historique de la présentation n'a pas pu être mis à jour
replica-unrelated = Ces deux présentations n'ont pas d'origine commune
replica-unsupported-version = Cette présentation utilise un format de deck que cette version d'Oku Slides ne sait pas lire

widget-unreadable = Les déclarations de widgets sont illisibles
widget-fault-empty-name = Widget { $widget } : un widget ou un paramètre n'a pas de nom
widget-fault-duplicate-widget = Le widget { $widget } est déclaré deux fois
widget-fault-duplicate-param = Le widget { $widget } déclare deux fois le même paramètre
widget-fault-empty-choice = Widget { $widget } : un choix n'a aucune valeur
widget-fault-default-not-in-choice = Widget { $widget } : la valeur par défaut d'un choix n'en fait pas partie
widget-fault-default-wrong-type = Widget { $widget } : une valeur par défaut ne correspond pas au type de son paramètre
widget-fault-bind-not-plain = Widget { $widget } : seul un texte simple peut être lié au deck
widget-fault-unknown-slot-kind = Widget { $widget } : un slot accepte un widget d'élément inconnu
widget-fault-empty-list = Widget { $widget } : une liste a besoin d'au moins une colonne
widget-fault-column-type = Widget { $widget } : une colonne de liste est plain, rich, number, flag, choice ou asset ; pour un contenu libre par item, utilisez p.slot avec un widget enfant
widget-fault-reserved-name = Widget { $widget } : nom réservé : id, kind, ou un nom contenant /
widget-fault-list-bounds = Widget { $widget } : le max d'une liste est sous son min
widget-fault-list-default-too-large = Widget { $widget } : le défaut d'une liste dépasse les bornes de taille des listes
widget-incompatible-override = Le widget { $widget } remplace celui de base mais modifie ou retire son paramètre { $param }
widget-incompatible-category = Le widget { $widget } remplace celui de base avec une autre catégorie
issue-empty-item-id = la liste { $param } a un item sans identifiant
issue-duplicate-item-id = la liste { $param } a un identifiant d'item en double
issue-list-bounds = la liste { $param } a trop peu ou trop d'items
