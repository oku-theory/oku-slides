op-slide-not-found = Slide introuvable : { $subject }
op-element-not-found = Élément introuvable : { $subject }
op-duplicate-id = Identifiant déjà utilisé : { $subject }
op-index-out-of-range = Position hors limites
op-invalid-value = Valeur invalide pour { $subject }
op-invalid-value-because = Valeur invalide pour { $subject } : { $reason }
op-value-no-value = le paramètre n'a pas de valeur
op-value-no-dataset = le graphique n'a pas de jeu de données
op-value-grid-tracks = une grille a de 1 à 24 colonnes et lignes
op-value-bad-item-id = un identifiant d'item est un texte non vide, sans « / »
op-value-missing-item-id = chaque item doit avoir un identifiant
op-value-too-deep = les éléments s'imbriquent trop profondément
op-value-empty-path = le chemin est vide
op-value-not-text = un texte ou null est attendu
op-value-empty-frame = la largeur et la hauteur doivent être supérieures à 0
op-value-blocks-off-grid = les slots en flux sont disposés par la slide et une grille les cache ; placez d'abord chaque élément dans un slot libre
op-value-outside-grid = la cellule est hors de la grille
op-value-not-tokens = un objet ou null est attendu
op-value-not-a-slot = ce paramètre n'est pas un slot
op-value-out-of-bounds = le nombre est hors des limites permises
op-value-unreadable = la valeur n'a pas la forme attendue
op-unknown-widget = Widget inconnu : { $subject }
op-unknown-param = Paramètre inconnu : { $subject }
op-param-type-because = Valeur invalide pour { $subject } : { $reason }
op-param-shape-text = { $nullable ->
    [yes] il faut un texte ou null
   *[no] il faut un texte
}
op-param-shape-choice = { $nullable ->
    [yes] il faut l'une de ses valeurs ou null
   *[no] il faut l'une de ses valeurs
}
op-param-shape-paragraphs = { $nullable ->
    [yes] il faut une liste de paragraphes ou null
   *[no] il faut une liste de paragraphes
}
op-param-shape-elements = { $nullable ->
    [yes] il faut une liste d'éléments ou null
   *[no] il faut une liste d'éléments
}
op-param-shape-flag = { $nullable ->
    [yes] il faut un booléen ou null
   *[no] il faut un booléen
}
op-param-shape-number = { $nullable ->
    [yes] il faut un nombre ou null
   *[no] il faut un nombre
}
op-param-shape-bounded-number = { $nullable ->
    [yes] il faut un nombre dans ses bornes ou null
   *[no] il faut un nombre dans ses bornes
}
op-param-shape-grid = { $nullable ->
    [yes] il faut une grille ou null
   *[no] il faut une grille
}
op-param-shape-chart-option = { $nullable ->
    [yes] il faut une option de graphique ou null
   *[no] il faut une option de graphique
}
op-param-shape-table-option = { $nullable ->
    [yes] il faut une option de tableau ou null
   *[no] il faut une option de tableau
}
op-param-shape-chart-or-table-option = { $nullable ->
    [yes] il faut une option de graphique ou de tableau ou null
   *[no] il faut une option de graphique ou de tableau
}
op-param-shape-items = { $nullable ->
    [yes] il faut une liste d'items ou null
   *[no] il faut une liste d'items
}
op-param-shape-list = { $nullable ->
    [yes] il faut une liste ou null
   *[no] il faut une liste
}
op-param-shape-add-element-only = { $nullable ->
    [yes] les éléments s'ajoutent avec add_element, ou null
   *[no] les éléments s'ajoutent avec add_element
}
op-param-shape-set-param-only = { $nullable ->
    [yes] la valeur se donne avec set_param, ou null
   *[no] la valeur se donne avec set_param
}
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
issue-slot-repaired = élément sans slot connu, rangé dans { $slot }
issue-unreadable-item = item de liste illisible, mis de côté
issue-unreadable-row = ligne de tableau illisible, mise de côté
issue-data-truncated = données trop grandes, tronquées à la lecture
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
