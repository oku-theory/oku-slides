data-cell-type = une valeur ne convient pas à la colonne « { $column } », qui contient { $type ->
    [number] des nombres
    [integer] des nombres entiers
    [bool] vrai ou faux
    [date] des dates (AAAA-MM-JJ)
   *[text] du texte de 16 Ko au plus
}
data-column-type = la colonne « { $column } » ne peut pas devenir { $type ->
    [number] une colonne de nombres
    [integer] une colonne d’entiers
    [bool] une colonne vrai ou faux
    [date] une colonne de dates
   *[text] une colonne de texte
} sans perte, voir les lignes { $rows }{ $more ->
    [0] {""}
    [one] {" "}et { $more } autre ligne
   *[other] {" "}et { $more } autres lignes
}
data-no-row = la ligne { $row } n’existe pas
data-no-column = la colonne { $column } n’existe pas
data-id-taken = l’identifiant { $id } est déjà utilisé dans ce tableau
data-id-invalid = l’identifiant « { $id } » n’est pas valide : utiliser de 1 à 64 lettres, chiffres, - ou _
data-id-missing = une nouvelle ligne, colonne ou table n’a pas d’identifiant
data-too-large = { $what ->
    [rows] la donnée dépasserait sa limite de { $max } { $max ->
        [one] ligne
       *[other] lignes
    }
    [columns] la donnée dépasserait sa limite de { $max } { $max ->
        [one] colonne
       *[other] colonnes
    }
    [text] une cellule de texte dépasserait sa limite de { $max } { $max ->
        [one] octet
       *[other] octets
    }
   *[overlay] la donnée dépasserait sa limite de { $max } { $max ->
        [one] cellule modifiée
       *[other] cellules modifiées
    }
}
data-invalid-name = « { $name } » n’est pas un nom de donnée valide : utiliser des minuscules, des chiffres et _, sans commencer par un chiffre
data-non-finite = { $count ->
    [one] { $count } nombre n’est pas fini (NaN ou infini)
   *[other] { $count } nombres ne sont pas finis (NaN ou infini)
}
data-columns-invalid = la liste de colonnes est vide ou cite deux fois une colonne
data-row-width = une ligne a plus de cellules que les { $max } { $max ->
    [one] colonne
   *[other] colonnes
} du tableau
data-needs-base = cette donnée garde ses lignes dans un fichier de base, que cette modification ne sait pas encore lire
data-base-row-cells = une table avec un fichier de base ne donne que les ids de ses lignes, pas de cellules
data-row-cut = { $count ->
    [one] { $count } ligne avait plus de cellules que l’en-tête ; les cellules en trop ont été retirées
   *[other] { $count } lignes avaient plus de cellules que l’en-tête ; les cellules en trop ont été retirées
}
data-base-invalid = le fichier de base des données est endommagé ou ne correspond pas à sa table
data-source-mismatch = la source des données ne correspond pas à leur type : des données manuelles portent un tableau, des données de requête une requête
data-inline-query = des données de requête ne peuvent pas être intégrées : les données intégrées viennent toujours d’un tableau
data-query-parse = la requête ne se lit pas : vérifier la ligne { $line }, colonne { $column }
data-query-construct = la requête utilise « { $construct } », que les requêtes de données ne prennent pas en charge
data-query-too-long = la requête dépasse { $max } { $max ->
    [one] octet
   *[other] octets
}
data-query-too-deep = la requête est trop imbriquée
data-query-rewrite = la requête n’a pas pu être préparée pour le moteur ; c’est un bogue d’Oku Slides
data-query-unknown-function = la requête appelle « { $function } », que les requêtes de données ne prennent pas en charge
data-query-run = la requête a échoué : { $reason }
data-query-input-failed = son entrée « { $input } » a échoué
data-query-too-large = { $what ->
    [rows] le résultat de la requête dépasse { $max } { $max ->
        [one] ligne
       *[other] lignes
    } ; il n’est jamais tronqué
    [columns] le résultat de la requête dépasse { $max } { $max ->
        [one] colonne
       *[other] colonnes
    } ; il n’est jamais tronqué
   *[text] une cellule de texte du résultat dépasse { $max } { $max ->
        [one] octet
       *[other] octets
    } ; le résultat n’est jamais tronqué
}
data-query-column-type = la colonne de résultat « { $column } » a un type que les données ne peuvent pas contenir : la convertir en texte, nombre, entier, booléen ou date
data-query-duplicate-column = deux colonnes de résultat s’appellent « { $column } » : renommer l’une d’elles avec AS
data-query-float-operand = « { $function } » ne s’applique pas à « { $operand } », qui contient des nombres décimaux : le convertir d’abord en entier
data-query-timeout = la requête a duré plus de { $seconds } { $seconds ->
    [one] seconde
   *[other] secondes
} et a été arrêtée
