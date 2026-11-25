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
