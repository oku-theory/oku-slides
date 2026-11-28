data-cell-type = a value does not fit column “{ $column }”, which holds { $type ->
    [number] numbers
    [integer] whole numbers
    [bool] true or false
    [date] dates (YYYY-MM-DD)
   *[text] text of at most 16 KB
}
data-column-type = column “{ $column }” cannot become { $type ->
    [number] a number column
    [integer] a whole-number column
    [bool] a true-or-false column
    [date] a date column
   *[text] a text column
} without loss, see rows { $rows }{ $more ->
    [0] {""}
    [one] {" "}and { $more } more row
   *[other] {" "}and { $more } more rows
}
data-no-row = row { $row } does not exist
data-no-column = column { $column } does not exist
data-id-taken = id { $id } is already used in this table
data-id-invalid = id “{ $id }” is not valid: use 1 to 64 letters, digits, - or _
data-id-missing = a new row, column or table has no id
data-too-large = { $what ->
    [rows] the data would pass its limit of { $max } { $max ->
        [one] row
       *[other] rows
    }
    [columns] the data would pass its limit of { $max } { $max ->
        [one] column
       *[other] columns
    }
    [text] a text cell would pass its limit of { $max } { $max ->
        [one] byte
       *[other] bytes
    }
   *[overlay] the data would pass its limit of { $max } { $max ->
        [one] edited cell
       *[other] edited cells
    }
}
data-invalid-name = “{ $name }” is not a valid data name: use lowercase letters, digits and _, not starting with a digit
data-non-finite = { $count ->
    [one] { $count } number is not finite (NaN or infinity)
   *[other] { $count } numbers are not finite (NaN or infinity)
}
data-columns-invalid = the column list is empty or names a column twice
data-row-width = a row has more cells than the table’s { $max } { $max ->
    [one] column
   *[other] columns
}
data-needs-base = this data keeps its rows in a base file, which this edit cannot read yet
data-base-row-cells = a table with a base file gives its rows ids only, not cells
data-row-cut = { $count ->
    [one] { $count } row had more cells than the header; the extra cells were dropped
   *[other] { $count } rows had more cells than the header; the extra cells were dropped
}
data-base-invalid = the data's base file is damaged or does not match its table
data-source-mismatch = the data's source does not match its kind: a manual data holds a table, a query data holds a query
data-inline-query = a query data cannot be inline: inline data always comes from a table
data-query-parse = the query does not parse: check line { $line }, column { $column }
data-query-construct = the query uses “{ $construct }”, which data queries do not support
data-query-too-long = the query is longer than { $max } { $max ->
    [one] byte
   *[other] bytes
}
data-query-too-deep = the query is nested too deeply
data-query-rewrite = the query could not be prepared for the engine; this is a bug in Oku Slides
data-query-unknown-function = the query calls “{ $function }”, which data queries do not support
data-query-run = the query failed: { $reason }
data-query-input-failed = its input “{ $input }” failed
data-query-too-large = { $what ->
    [rows] the query result has more than { $max } { $max ->
        [one] row
       *[other] rows
    }; it is never cut
    [columns] the query result has more than { $max } { $max ->
        [one] column
       *[other] columns
    }; it is never cut
   *[text] a text cell of the query result is longer than { $max } { $max ->
        [one] byte
       *[other] bytes
    }; it is never cut
}
data-query-column-type = the result column “{ $column }” has a type data cannot hold: cast it to text, a number, an integer, a boolean or a date
data-query-duplicate-column = two result columns are named “{ $column }”: rename one of them with AS
data-query-float-operand = “{ $function }” cannot be applied to “{ $operand }”, which holds decimal numbers: cast it to an integer first
data-query-timeout = the query ran longer than { $seconds } { $seconds ->
    [one] second
   *[other] seconds
} and was stopped
