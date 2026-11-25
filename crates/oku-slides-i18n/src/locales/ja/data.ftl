data-cell-type = 値が列「{ $column }」に合いません（この列の内容：{ $type ->
    [number] 数値
    [integer] 整数
    [bool] 真または偽
    [date] 日付、YYYY-MM-DD
   *[text] 最大16KBのテキスト
}）
data-column-type = 列「{ $column }」は損失なしに{ $type ->
    [number] 数値の列
    [integer] 整数の列
    [bool] 真偽の列
    [date] 日付の列
   *[text] テキストの列
}にできません。該当する行：{ $rows }{ $more ->
    [0] {""}
   *[other] {""}、ほか{ $more }行
}
data-no-row = 行 { $row } は存在しません
data-no-column = 列 { $column } は存在しません
data-id-taken = ID { $id } はこの表ですでに使われています
data-id-invalid = ID「{ $id }」は無効です：英字、数字、- または _ を1～64文字で使用してください
data-id-missing = 新しい行、列、または表に ID がありません
data-too-large = { $what ->
    [rows] データが行数の上限（{ $max }行）を超えます
    [columns] データが列数の上限（{ $max }列）を超えます
    [text] テキストのセルが上限（{ $max }バイト）を超えます
   *[overlay] データが編集済みセルの上限（{ $max }件）を超えます
}
data-invalid-name = 「{ $name }」は有効なデータ名ではありません：小文字の英字、数字、_ を使い、数字で始めないでください
data-non-finite = { $count }個の数値が有限ではありません（NaN または無限大）
data-columns-invalid = 列のリストが空か、同じ列を2回指定しています
data-row-width = 行のセル数が表の{ $max }列を超えています
data-needs-base = このデータは行をベースファイルに保存しており、この編集ではまだ読み取れません
data-base-row-cells = ベースファイルを持つ表は、行の ID だけを指定し、セルは指定しません
data-row-cut = { $count }行のセル数が見出しより多かったため、余分なセルを削除しました
data-base-invalid = データのベースファイルが破損しているか、表と一致しません
