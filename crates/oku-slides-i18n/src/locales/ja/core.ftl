op-slide-not-found = スライドが見つかりません：{ $subject }
op-element-not-found = 要素が見つかりません：{ $subject }
op-duplicate-id = 識別子はすでに使用されています：{ $subject }
op-index-out-of-range = 位置が範囲外です
op-invalid-value = { $subject } の値が無効です
op-invalid-value-because = { $subject } の値が無効です：{ $reason }
op-value-no-value = パラメーターに値がありません
op-value-no-dataset = グラフにデータセットがありません
op-value-grid-tracks = グリッドの列と行は 1～24 です
op-value-bad-item-id = 項目 ID は「/」を含まない空でないテキストです
op-value-missing-item-id = 各項目には ID が必要です
op-value-too-deep = 要素の入れ子が深すぎます
op-value-empty-path = パスが空です
op-value-not-text = テキストまたは null が必要です
op-value-empty-frame = 幅と高さは 0 より大きくなければなりません
op-value-blocks-off-grid = フロースロットはスライドが配置するため、グリッドを使うと隠れてしまいます。先に各要素を空いているスロットへ移してください
op-value-outside-grid = セルがグリッドの外にあります
op-value-not-tokens = オブジェクトまたは null が必要です
op-value-not-a-slot = このパラメーターはスロットではありません
op-value-out-of-bounds = 数値が許容範囲外です
op-value-unreadable = 値が想定された形式ではありません
op-unknown-widget = 不明なウィジェット：{ $subject }
op-unknown-param = 不明なパラメーター：{ $subject }
op-param-type-because = { $subject } の値が無効です：{ $reason }
op-param-shape-text = { $nullable ->
    [yes] テキストまたは null が必要です
   *[no] テキストが必要です
}
op-param-shape-choice = { $nullable ->
    [yes] その値のいずれかまたは null が必要です
   *[no] その値のいずれかが必要です
}
op-param-shape-paragraphs = { $nullable ->
    [yes] 段落のリストまたは null が必要です
   *[no] 段落のリストが必要です
}
op-param-shape-elements = { $nullable ->
    [yes] 要素のリストまたは null が必要です
   *[no] 要素のリストが必要です
}
op-param-shape-flag = { $nullable ->
    [yes] 真偽値または null が必要です
   *[no] 真偽値が必要です
}
op-param-shape-number = { $nullable ->
    [yes] 数値または null が必要です
   *[no] 数値が必要です
}
op-param-shape-bounded-number = { $nullable ->
    [yes] 範囲内の数値または null が必要です
   *[no] 範囲内の数値が必要です
}
op-param-shape-grid = { $nullable ->
    [yes] グリッドまたは null が必要です
   *[no] グリッドが必要です
}
op-param-shape-chart-option = { $nullable ->
    [yes] グラフのオプションまたは null が必要です
   *[no] グラフのオプションが必要です
}
op-param-shape-table-option = { $nullable ->
    [yes] 表のオプションまたは null が必要です
   *[no] 表のオプションが必要です
}
op-param-shape-chart-or-table-option = { $nullable ->
    [yes] グラフまたは表のオプションまたは null が必要です
   *[no] グラフまたは表のオプションが必要です
}
op-param-shape-items = { $nullable ->
    [yes] 項目のリストまたは null が必要です
   *[no] 項目のリストが必要です
}
op-param-shape-list = { $nullable ->
    [yes] リストまたは null が必要です
   *[no] リストが必要です
}
op-param-shape-add-element-only = { $nullable ->
    [yes] 要素は add_element で追加します、または null
   *[no] 要素は add_element で追加します
}
op-param-shape-set-param-only = { $nullable ->
    [yes] 値は set_param で設定します、または null
   *[no] 値は set_param で設定します
}
op-item-not-found = 項目 { $subject } がありません
op-list-full = リスト { $subject } がいっぱいです（最大 { $max }項目）
op-list-minimum = リスト { $subject } には少なくとも { $min }項目が必要です
op-list-cell-too-long = リスト { $subject } のセルが長すぎます（列 { $column }）
op-list-id-too-long = リスト { $subject } の項目 ID が長すぎます
op-list-too-many-items = リスト { $subject } の項目が多すぎます（最大 { $max }）
op-slot-rejects = { $subject } はここでは使用できません
op-wrong-category = { $subject } はここでは使えません
op-choice-out-of-range = { $subject } に許可されていない値です
transaction-refused = 操作 { $index } が拒否されました：{ $reason }

sources-unreadable = ソースを読み込めません
sources-not-sources = Oku Slides のソースアーカイブではありません
sources-unsupported-version = このソースは新しいバージョンの Oku Slides で書き込まれています
sources-bad-name = ソース内のファイル名が拒否されました
sources-missing-deck = ソースにプレゼンテーションがありません
sources-too-large = このファイルのソースは大きすぎて開けません
sources-too-deep = このデッキは要素の入れ子が深すぎます

issue-empty-slide-id = スライドの識別子が空です
issue-duplicate-slide-id = スライドの識別子が重複しています
issue-duplicate-element-id = 要素の識別子が重複しています
issue-empty-element-id = 識別子が空です
issue-cell-outside-grid = セルがグリッドの外にあります
issue-empty-frame = フレームのサイズがゼロです
issue-free-without-frame = 自由配置の要素にフレームがありません
issue-unknown-widget = 不明なウィジェット: { $kind }
issue-wrong-category = { $kind } はここでは使えません
issue-unknown-param = 不明なパラメーター: { $param }
issue-param-type = { $param } の値が無効です
issue-choice-out-of-range = { $param } に許可されていない値です
issue-slot-rejects = { $kind } はここでは受け付けられません
issue-placed-in-flow = レイアウトされたリストに枠付きで配置された要素
issue-unknown-bind = { $param } は不明なデッキ設定に紐付いています
issue-slot-too-deep = 要素の入れ子が深すぎます
issue-missing-asset = アセットがありません：{ $asset }
issue-pack-version-mismatch = 改変されたパックのバージョンを破棄しました：{ $version }
issue-pack-version-missing = パックのバージョンがソースにありません：{ $version }
issue-theme-version-mismatch = 改変されたテーマのバージョンを破棄しました：{ $version }
issue-theme-version-missing = テーマのバージョンがソースにありません：{ $version }
issue-unsupported-link = サポートされていないリンク：{ $link }
issue-highlight-zero = ハイライトの色番号は 1 から始まります
issue-unreadable-element = 読み取れない要素を除外しました
issue-slot-repaired = 既知のスロットがない要素を { $slot } に配置しました
issue-unreadable-item = 読み取れないリスト項目を除外しました
issue-unreadable-row = 読み取れない表の行を除外しました
issue-data-truncated = データが大きすぎるため、読み込み時に切り詰めました
issue-unreadable-slide = 読み取れないスライドを除外しました
issue-unreadable-field = 読み取れないデッキ項目 { $field } をリセットしました
op-invalid-chart = グラフ { $subject }：{ $reason }
op-invalid-table = テーブル { $subject }：{ $reason }

replica-corrupted = このファイル内のプレゼンテーション履歴が破損しています
replica-too-large = このファイル内のプレゼンテーション履歴は大きすぎて開けません
replica-write = プレゼンテーション履歴を更新できませんでした
replica-unrelated = この2つのプレゼンテーションには共通の起源がありません
replica-unsupported-version = このプレゼンテーションは、このバージョンの Oku Slides では読み込めない形式を使用しています

widget-unreadable = ウィジェットの宣言を読み取れませんでした
widget-fault-empty-name = ウィジェット { $widget }：ウィジェットまたはパラメーターに名前がありません
widget-fault-duplicate-widget = ウィジェット { $widget } が二重に宣言されています
widget-fault-duplicate-param = ウィジェット { $widget } が同じパラメーターを二重に宣言しています
widget-fault-empty-choice = ウィジェット { $widget }：選択肢に値がありません
widget-fault-default-not-in-choice = ウィジェット { $widget }：選択肢の既定値がその値に含まれていません
widget-fault-default-wrong-type = ウィジェット { $widget }：既定値がパラメーターの型と一致していません
widget-fault-bind-not-plain = ウィジェット { $widget }：プレーンテキストのパラメーターのみプレゼンテーションに紐付けできます
widget-fault-unknown-slot-kind = ウィジェット { $widget }：スロットが未知の要素ウィジェットを受け入れています
widget-fault-empty-list = ウィジェット { $widget }：リストには少なくとも 1つの列が必要です
widget-fault-column-type = ウィジェット { $widget }：リストの列は plain、rich、number、flag、choice、asset のいずれかです。項目ごとに自由な内容が必要な場合は、子ウィジェットを持つ p.slot を使ってください
widget-fault-reserved-name = ウィジェット { $widget }：予約された名前：id、kind、または / を含む名前
widget-fault-list-bounds = ウィジェット { $widget }：リストの最大値が最小値を下回っています
widget-fault-list-default-too-large = ウィジェット { $widget }：リストの既定値がリストのサイズ上限を超えています
widget-incompatible-override = ウィジェット { $widget } は組み込みのものを置き換えますが、パラメーター { $param } を変更または削除しています
widget-incompatible-category = ウィジェット { $widget } は組み込みのものを異なるカテゴリで置き換えています
issue-empty-item-id = リスト { $param } に識別子のない項目があります
issue-duplicate-item-id = リスト { $param } に識別子が重複した項目があります
issue-list-bounds = リスト { $param } の項目数が少なすぎるか多すぎます
