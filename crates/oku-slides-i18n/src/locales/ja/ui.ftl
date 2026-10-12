# アプリのインターフェイス（React）。ui/src/i18n.ts が「messages」リクエストで読み込む。
ui-loading = 読み込み中…

## ドキュメント

ui-brand = Oku Slides
ui-brand-templates = Oku Slides · テンプレート
ui-title-rename = プレゼンテーション名を変更
ui-title-field = プレゼンテーションのタイトル
ui-status-unsaved = 未保存の変更あり
ui-status-saved = 保存済み
ui-status-never-saved = 未保存
ui-caption-templates = { $count ->
    [one] テンプレート { $count }個
   *[other] テンプレート { $count }個
} · { $status }
ui-caption-slides = { $count ->
    [one] スライド { $count }枚
   *[other] スライド { $count }枚
} · { $status }
ui-template-default-name = テンプレート
ui-template-variant-name = { $name }（バリエーション）
ui-confirm-quit-templates = 変更したテンプレートを保存せずに終了しますか？

## トースト

ui-toast-templates-saved = テンプレートを保存しました：{ $name }
ui-toast-moved = プレゼンテーションを移動しました
ui-toast-copied = コピーを保存しました
ui-toast-select-zone = 配置する領域を選択してください：「{ $text }」
ui-toast-field-gone = 編集中のフィールドはもう存在しません。入力内容は失われました。

## ツールバー

ui-toolbar-undo = 取り消す
ui-toolbar-undo-label = 取り消す：{ $label }
ui-toolbar-redo = やり直す
ui-toolbar-redo-label = やり直す：{ $label }
ui-toolbar-grid-hide = グリッドを隠す
ui-toolbar-grid-show = グリッドを表示
ui-toolbar-ghosts-hide = ゴーストを隠す
ui-toolbar-ghosts-show = ゴーストを表示
ui-divider-pile = スライド一覧の幅
ui-divider-notes = エージェントのスレッドの幅
ui-divider-code = コードのサイズ
ui-divider-band = 設定の高さ
ui-toolbar-theme = テーマ
ui-toolbar-theme-embedded = ファイルのテーマ
ui-toolbar-save-copy = コピーを保存
ui-toolbar-finish = 完了
ui-menu-workshop = 工房
ui-toolbar-open = 開く
ui-merge = PDF と統合…
ui-menu = メニュー
ui-menu-move = 移動…
ui-language = 言語
ui-language-system = システムの言語
ui-scheme = 外観
ui-scheme-system = システム
ui-scheme-light = ライト
ui-scheme-dark = ダーク
ui-toolbar-add-label = スライドに追加
ui-add-kind = { $kind ->
    [card] カード
    [shape] 図形
    [chart] グラフ
    [table] 表
   *[text] テキスト
}
ui-add-widget = パックのウィジェット
ui-toolbar-remove-template = テンプレートを削除
ui-toolbar-remove-slide = スライドを削除
ui-toolbar-remove-hint = スライド { $n } · ⌘Z で取り消し

## 取り消しラベル

ui-undo-add = { $kind ->
    [card] カードを追加
    [shape] 図形を追加
    [chart] グラフを追加
    [table] 表を追加
   *[text] テキストを追加
}
ui-undo-add-widget = 「{ $name }」を追加
ui-undo-new-template = 新規テンプレート：{ $name }
ui-undo-new-slide = 新規スライド：{ $name }
ui-undo-templates = テンプレート：{ $name }
ui-undo-move-template = テンプレートを移動
ui-undo-move-slide = スライドを移動
ui-undo-remove-slide = スライドを削除
ui-undo-remove = 削除
ui-undo-paste = ペースト
ui-undo-place-on-grid = グリッドに配置
ui-undo-move-on-grid = グリッド上で移動
ui-undo-resize-on-grid = グリッド上でサイズ変更
ui-undo-change-anchor = アンカーを変更
ui-undo-move = 移動
ui-undo-resize = サイズ変更
ui-undo-edit-slide = スライドを編集
ui-undo-dictation = 音声入力
ui-undo-rename-set = セット名を変更
ui-undo-rename-deck = プレゼンテーション名を変更
ui-undo-chart-kind = グラフの種類を変更
ui-undo-chart-data = データを編集
ui-undo-chart-color = 系列に色を付ける
ui-undo-chart-stack = { $stacked ->
    [yes] 系列を積み上げ
   *[no] 系列の積み上げを解除
}

## 新しい要素

ui-new-text = 新しいテキスト
ui-new-card-title = カードのタイトル
ui-new-card-detail = 詳細

## スライド一覧

ui-pile-slides = スライド
ui-pile-templates = テンプレート
ui-pile-add-slide = + 新規スライド
ui-pile-add-template = + 新規テンプレート
ui-pile-drag-hint = ドラッグで移動 · ⌥↑ ⌥↓
ui-slide-alt = スライド { $n }

## ステージ

ui-stage-rendering = レンダリング中…
ui-stage-elements = スライドの要素
ui-stage-fields = スライドのフィールド
ui-stage-edit-text = テキストを編集
ui-stage-hint-plain = Enter または Tab で次へ · Esc でキャンセル
ui-stage-hint-rich = Tab で次へ · ⌘ Enter で確定 · 「- 」で箇条書き · ⌘B 太字、⌘I 斜体、⌘K リンク
ui-field = { $kind ->
    [title] タイトル
    [subtitle] サブタイトル
    [kicker] キッカー
    [text] テキスト
    [body] 本文
    [author] 著者
    [date] 日付
    [organization] 組織
   *[other] { $kind }
}
ui-element-kind = { $kind ->
    [text] テキスト
    [card] カード
    [image] 画像
    [shape] 図形
    [group] グループ
   *[other] { $kind }
}
ui-grid-cell = 列 { $cols } · 行 { $rows }
ui-ghost-zone = ゾーン

## スライドノート

ui-notes = スライドツール
ui-stepper-less = { $label }：1つ減らす
ui-stepper-more = { $label }：1つ増やす
ui-list-item = 項目
ui-list-add = { $item }を追加
ui-list-remove = { $item }を削除
ui-list-move = { $item }を移動
ui-list-full = リストがいっぱいです（{ $max }）
ui-undo-list-move = { $item }を移動
ui-agent-color-other = その他の色
ui-agent-thread = エージェント
ui-agent-new-conversation = 新しい会話
ui-agent-detach = エージェントを別ウィンドウで開く
ui-agent-attach = エージェントをメインウィンドウに戻す
ui-agent-window-empty = このタブにはエージェントがありません
ui-agent-kept-in-draft = 書き込み先がありません：テキストは下書きに残しました
ui-agent-busy-elsewhere = エージェントは { $place } で応答中です…
ui-agent-scope-slides = スライド
ui-agent-scope-atelier = ワークショップ
ui-draft = 下書き
ui-draft-insert = 挿入
ui-draft-insert-title = そのまま挿入 (⌘⇧↩)
ui-draft-send = エージェントに送信
ui-draft-send-title = エージェントに送信 (⌘↩)
ui-draft-discard = 破棄
ui-agent-state-idle = エージェント
ui-agent-state-starting = エージェントを起動中…
ui-agent-state-working = エージェントが作業中…
ui-agent-dock = 進行中
ui-agent-dock-settings = 設定
ui-agent-jump = ↓ 現在のターン
ui-agent-tool-input = 引数
ui-agent-tool-output = 結果
ui-agent-sheet = エージェントの質問
ui-agent-sheet-keys = 1–4 選択 · Enter 確定 · E 編集 · Esc 取り消し
ui-agent-variants = バリエーション
ui-agent-drawer-keys = クリックまたは 1–4 · Esc で閉じる
ui-agent-confirm = エージェントの確認

## 音声入力

ui-dictation = 音声入力
ui-dictation-start = 音声入力
ui-dictation-start-title = 音声入力（⌘D）
ui-dictation-type = エージェントに書く
ui-dictation-type-title = エージェントに書く（⌘K）
ui-dictation-agent-stop = エージェントを停止（⌘⇧D または Esc）
ui-dictation-agent-working = エージェントが作業中…
ui-dictation-agent-working-hint = ⌘⇧D または Esc で停止
ui-dictation-finish = 音声入力を終了
ui-dictation-finish-title = 終了（⌘D）
ui-dictation-cancel = キャンセル
ui-dictation-preparing = 音声入力を準備中…
ui-dictation-heard-nothing = 何も聞き取れませんでした：もう少しマイクに近づいて話してください
ui-dictation-to-cursor = カーソル位置に挿入
ui-dictation-to-draft = 下書きへ
ui-dictation-to-field = { $field }を置き換え
ui-dictation-to-block = 選択したブロックの後に挿入
ui-dictation-new-block = 新しいテキストブロック
ui-dictation-select-zone = 領域を選択
ui-dictation-downloading = { $name } モデルをダウンロード中
ui-dictation-download-progress = { $percent }% · { $total } 中 { $received } · 初回のみ
ui-dictation-listening = 聞き取り中 · { $time }
ui-dictation-transcribing = 最後の文を文字起こし中…
ui-dictation-stop-in = { $duration } 後に停止
ui-dictation-speak = 話してください。ここにテキストが表示されます…
ui-dictation-auto-stop = { $duration } 無音が続くと停止
ui-dictation-to-download = { $size } のダウンロードが必要
ui-dictation-settings = 設定
ui-dictation-model-title = 認識モデル
ui-dictation-model-intro = Whisper をこのコンピュータ上で実行します（CTranslate2）。どのモデルも複数の言語に対応しています。
ui-dictation-note-turbo = おすすめ。高速で高精度
ui-dictation-note-large = 最も高精度ですが低速
ui-dictation-note-base = 軽量。性能の低いマシン向け
ui-dictation-installed = インストール済み
ui-dictation-not-installed = 未ダウンロード
ui-dictation-language = 話す言語
ui-dictation-language-auto = 自動検出
ui-dictation-note-apple = 最速、システムに内蔵
ui-dictation-model-unavailable = この言語には非対応
ui-dictation-model-intro-apple = Apple（システム内蔵）、またはこのコンピュータ上の Whisper（CTranslate2）。どれも複数の言語に対応しています。
ui-dictation-language-auto-resolved = 自動（{ $language }）
ui-dictation-to-download-language = 言語のダウンロードが必要
ui-dictation-auto-stop-label = 次の時間無音が続くと自動停止
ui-dictation-silence = 無音の長さ
ui-dictation-phrases-note = 話している間、発話部分だけを 1 文ずつ文字起こしします。
ui-dictation-download-now = 今すぐダウンロード（{ $size }）
ui-dictation-download-language = 言語をダウンロード
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } 秒
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } GB
ui-size-mb = { $value } MB

## テンプレートピッカー

ui-picker-new-slide = 新規スライド
ui-picker-new-template = 新規テンプレートのベースを選択…
ui-picker-set = テンプレート
ui-picker-set-mine = { $name } · マイ
ui-picker-keys = ← → で切り替え · Enter で追加 · Esc で閉じる
ui-picker-close = 閉じる
ui-picker-previous = 前のテンプレート
ui-picker-next = 次のテンプレート
ui-picker-list = スライドテンプレート

## グラフ

ui-chart-panel = グラフ
ui-chart-kinds = グラフの種類
ui-chart-kind = { $kind ->
    [line] 折れ線
    [pie] 円
    [scatter] 散布
    [radar] レーダー
    [waterfall] ウォーターフォール
   *[bar] 棒
}
ui-chart-no-data = このグラフにはデータ表がありません。
ui-chart-options = グラフのオプション
ui-chart-stack = 積み上げ
ui-chart-waterfall-hint = 合計バーにするには「合計」にチェックを入れます。
ui-chart-colors = 系列の色
ui-chart-color-of = { $name } の色
ui-chart-color-auto = 自動
ui-chart-color-pick = 色 { $n }
ui-chart-column-name = 列 { $col } の名前
ui-chart-remove-column = 列を削除
ui-chart-cell = 行 { $row }、列 { $col }
ui-chart-remove-row = 行を削除
ui-chart-add-row = 行
ui-chart-add-column = 列
ui-chart-series = 系列 { $n }

## 表

ui-table-panel = 表
ui-table-options = 表のオプション
ui-table-header-row = 見出し行
ui-table-header-column = 最初の列を見出しにする
ui-table-striped = 縞模様の行
ui-table-alignment = 列の配置
ui-table-align-of = { $name }の配置
ui-table-align = { $align ->
    [start] 先頭
    [center] 中央
    [end] 末尾
   *[auto] 自動
}
ui-table-column = 列 { $n }
ui-table-no-data = この表にはデータがありません。
ui-undo-table-option = 表のオプション
ui-undo-table-align = 列を揃える
ui-undo-table-data = 表を編集
ui-show-as = 表示形式
ui-show-as-table = 表として表示
ui-show-as-chart = グラフとして表示
ui-undo-show-as-table = 表として表示
ui-undo-show-as-chart = グラフとして表示

## ファイル

ui-file-open-prompt = 開くファイルのパス（.oku-slides.pdf または .json）
ui-file-save-prompt = 名前を付けて保存（フルパス）

## タブ

ui-tabs = 開いているプレゼンテーション
ui-tab-new = 新しいタブ
ui-tab-close = タブを閉じる
ui-tab-unsaved = 未保存の変更
ui-tab-templates = テンプレート
ui-tab-move-new-window = 新しいウィンドウに移動
ui-tab-move-to = ウィンドウ { $n } に移動
ui-tab-close-others = ほかのタブを閉じる
ui-close-cancel = キャンセル
ui-disk-replaced-question = { $file } はディスク上で別のプレゼンテーションに置き換えられました。
ui-disk-replaced-overwrite = 上書き
ui-disk-replaced-save-as = 名前を付けて保存…
ui-same-deck-question = { $file } は、タブ「{ $title }」で開いているプレゼンテーションの別バージョンです。
ui-same-deck-merge = そのタブに統合
ui-same-deck-apart = 別に開く
ui-history = 履歴
ui-history-version = { $date } のバージョン
ui-history-restore = 復元
ui-history-back = 戻る
ui-history-anonymous = 匿名
ui-history-slider = プレゼンテーションのバージョン
ui-history-removed-since = その後削除
ui-history-changed-since = その後変更
ui-history-added-since = その後追加されたスライド: { $count }
ui-help = ヘルプ
ui-help-caption = スライド { $index } / { $count } · 読み取り専用
ui-slide-previous = 前のスライド
ui-slide-next = 次のスライド
ui-present = プレゼンテーション
ui-present-slide = スライド { $index } / { $count }
ui-present-screen = フルスクリーンで発表
ui-present-window = ウインドウ内で発表
ui-toolbar-params-hide = 設定を隠す
ui-toolbar-params-show = 設定を表示
ui-help-close = ヘルプを閉じる
ui-help-search = ヘルプを検索
ui-help-search-placeholder = 検索…
ui-help-search-empty = 結果なし
ui-help-search-count = { $count }件
ui-pack-notice-close = 閉じる
ui-widgets-files = ウィジェットファイル
ui-widgets-common = 共通
ui-widgets-new = 新規ウィジェット
ui-widgets-name = ウィジェット名（小文字、数字、ハイフン）
ui-widgets-name-taken = この名前はすでに使われています
ui-widgets-name-letter = 名前は英字で始める必要があります
ui-widgets-valid = 有効
ui-widgets-errors = エラー
ui-widgets-empty = このパックにはまだウィジェットがありません。
ui-widgets-preview = ウィジェットのプレビュー
ui-widgets-no-preview = プレビューなし
ui-widgets-field-bound = プレゼンテーションにリンク：ここでは読み取り専用
ui-widgets-source = ソースコード
ui-widgets-unsaved = 未保存
ui-widgets-in-error = エラーのあるウィジェット
ui-widgets-none-in-error = エラーのあるウィジェットはありません
ui-widgets-section = ウィジェット
ui-widgets-open = ウィジェット { $name } を開く
ui-widgets-delete-named = { $name } を削除
ui-widgets-confirm-delete = ウィジェット「{ $name }」を削除しますか?
ui-widgets-rename-hint = ダブルクリックまたは F2 で名前を変更
ui-widgets-tabs = テンプレートのタブ
ui-widgets-templates-tab = テンプレート
ui-theme-tab = テーマ
ui-theme-picker = テーマ
ui-theme-name = テーマ名
ui-theme-unsaved = テーマの変更は未保存
ui-widgets-close-tab = { $name } を閉じる
ui-widgets-removed-by-agent = エージェントがウィジェット { $name } を削除しました。
