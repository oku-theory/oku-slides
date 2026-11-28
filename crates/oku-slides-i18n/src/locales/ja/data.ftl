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
data-source-mismatch = データのソースが種類と一致しません。手動データは表を、クエリデータはクエリを持ちます
data-inline-query = クエリデータはインラインにできません。インラインデータは常に表から作られます
data-query-parse = クエリを読み取れません：{ $line }行目、{ $column }列目を確認してください
data-query-construct = クエリが「{ $construct }」を使っていますが、データクエリでは使えません
data-query-too-long = クエリが{ $max }バイトを超えています
data-query-too-deep = クエリの入れ子が深すぎます
data-query-rewrite = クエリをエンジン用に準備できませんでした。これは Oku Slides の不具合です
data-query-unknown-function = クエリが「{ $function }」を呼び出していますが、データクエリでは使えません
data-query-run = クエリが失敗しました: { $reason }
data-query-input-failed = 入力「{ $input }」が失敗しました
data-query-too-large = { $what ->
    [rows] クエリ結果が行数の上限（{ $max }行）を超えています。結果は切り詰められません
    [columns] クエリ結果が列数の上限（{ $max }列）を超えています。結果は切り詰められません
   *[text] クエリ結果のテキストのセルが上限（{ $max }バイト）を超えています。結果は切り詰められません
}
data-query-column-type = 結果の列「{ $column }」の型はデータに保持できません。テキスト、数値、整数、真偽値、日付のいずれかに変換してください
data-query-duplicate-column = 結果の列のうち2つが「{ $column }」という名前です。AS でどちらかの名前を変えてください
data-query-float-operand = 「{ $operand }」には小数が入っているため、「{ $function }」を適用できません。先に整数に変換してください
data-query-timeout = クエリが{ $seconds }秒を超えたため停止しました
