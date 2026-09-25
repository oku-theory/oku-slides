# アプリのインターフェイス（React）。ui/src/i18n.ts が「messages」リクエストで読み込む。
ui-loading = 読み込み中…

## ドキュメント

ui-brand = Écho
ui-brand-templates = Écho · テンプレート
ui-title-rename = プレゼンテーション名を変更
ui-title-field = プレゼンテーションのタイトル
ui-status-unsaved = 未保存の変更あり
ui-status-saved = 保存済み
ui-status-never-saved = 未保存
ui-status-builtin = 組み込み：保存するとコピーが作成されます
ui-caption-templates = { $count ->
    [one] テンプレート { $count } 個
   *[other] テンプレート { $count } 個
} · { $status }
ui-caption-slides = { $count ->
    [one] スライド { $count } 枚
   *[other] スライド { $count } 枚
} · { $status }
ui-template-default-name = テンプレート
ui-template-variant-name = { $name }（バリエーション）
ui-confirm-quit-templates = 変更したテンプレートを保存せずに終了しますか？

## トースト

ui-toast-saved = プレゼンテーションを保存しました
ui-toast-templates-saved = テンプレートを保存しました：{ $name }
ui-toast-select-zone = 配置する領域を選択してください：「{ $text }」

## ツールバー

ui-toolbar-undo = 取り消す
ui-toolbar-undo-label = 取り消す：{ $label }
ui-toolbar-redo = やり直す
ui-toolbar-redo-label = やり直す：{ $label }
ui-toolbar-grid-hide = グリッドを隠す
ui-toolbar-grid-show = グリッドを表示
ui-toolbar-grid-hide-title = グリッドを隠す（⌘'）
ui-toolbar-grid-show-title = グリッドを表示（⌘'）
ui-toolbar-theme = テーマ
ui-toolbar-theme-default = デフォルト
ui-toolbar-save-copy = コピーを保存
ui-toolbar-save = 保存
ui-toolbar-finish = 完了
ui-toolbar-templates = テンプレート
ui-toolbar-templates-title = スライドテンプレートを作成・編集
ui-toolbar-open = 開く

## 取り消しラベル

ui-undo-add = { $kind ->
    [card] カードを追加
    [shape] 図形を追加
    [chart] グラフを追加
   *[text] テキストを追加
}
ui-undo-new-template = 新規テンプレート：{ $name }
ui-undo-new-slide = 新規スライド：{ $name }
ui-undo-grid = グリッド { $columns } × { $rows }
ui-undo-templates = テンプレート：{ $name }
ui-undo-move-template = テンプレートを移動
ui-undo-move-slide = スライドを移動
ui-undo-remove-slide = スライドを削除
ui-undo-remove = 削除
ui-undo-place-on-grid = グリッドに配置
ui-undo-move-on-grid = グリッド上で移動
ui-undo-resize-on-grid = グリッド上でサイズ変更
ui-undo-change-anchor = アンカーを変更
ui-undo-move = 移動
ui-undo-resize = サイズ変更
ui-undo-edit-text = テキストを編集
ui-undo-dictation = 音声入力
ui-undo-rename-set = セット名を変更
ui-undo-rename-deck = プレゼンテーション名を変更
ui-undo-theme = テーマを変更
ui-undo-rename-template = テンプレート名を変更
ui-undo-describe-template = テンプレートの説明を編集
ui-undo-chart-kind = グラフの種類を変更
ui-undo-chart-data = データを編集

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
ui-stage-edit-text = テキストを編集
ui-stage-hint-field = Enter で確定 · Esc でキャンセル
ui-stage-hint-block = ⌘ Enter で確定 · 1 行に 1 段落、「- 」で箇条書き
ui-field = { $kind ->
    [title] タイトル
    [subtitle] サブタイトル
    [kicker] キッカー
    [text] テキスト
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

## スライドノート

ui-notes = スライドツール
ui-note-template = このテンプレート
ui-note-template-name = 名前
ui-note-template-description = 説明
ui-note-add-label = スライドに追加
ui-note-add = 追加
ui-add-kind = { $kind ->
    [card] カード
    [shape] 図形
    [chart] グラフ
   *[text] テキスト
}
ui-note-grid-label = テンプレートのグリッド
ui-note-grid = グリッド
ui-note-grid-columns = 列
ui-note-grid-rows = 行
ui-stepper-less = { $label }：1 つ減らす
ui-stepper-more = { $label }：1 つ増やす
ui-note-remove-template = テンプレートを削除
ui-note-remove-slide = スライドを削除
ui-note-remove-hint = スライド { $n } · ⌘Z で取り消し
ui-note-tip = ヒント
ui-note-tip-text = ⌘D で音声入力。⌘' でグリッドを表示。ダブルクリックでテキストを編集。

## 音声入力

ui-dictation = 音声入力
ui-dictation-start = 音声入力
ui-dictation-start-title = 音声入力（⌘D）
ui-dictation-finish = 音声入力を終了
ui-dictation-finish-title = 終了（⌘D）
ui-dictation-cancel = キャンセル
ui-dictation-preparing = 音声入力を準備中…
ui-dictation-heard-nothing = 何も聞き取れませんでした：もう少しマイクに近づいて話してください
ui-dictation-to-cursor = カーソル位置に挿入
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
ui-dictation-settings = 音声入力の設定
ui-dictation-model-title = 認識モデル
ui-dictation-model-intro = Whisper をこのコンピュータ上で実行します（CTranslate2）。どのモデルも複数の言語に対応しています。
ui-dictation-note-turbo = おすすめ。高速で高精度
ui-dictation-note-large = 最も高精度ですが低速
ui-dictation-note-base = 軽量。性能の低いマシン向け
ui-dictation-installed = インストール済み
ui-dictation-not-installed = 未ダウンロード
ui-dictation-language = 話す言語
ui-dictation-language-auto = 自動検出
ui-dictation-auto-stop-label = 次の時間無音が続くと自動停止
ui-dictation-silence = 無音の長さ
ui-dictation-phrases-note = 話している間、発話部分だけを 1 文ずつ文字起こしします。
ui-dictation-download-now = 今すぐダウンロード（{ $size }）
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
   *[bar] 棒
}
ui-chart-no-data = このグラフにはデータ表がありません。
ui-chart-column-name = 列 { $col } の名前
ui-chart-remove-column = 列を削除
ui-chart-cell = 行 { $row }、列 { $col }
ui-chart-remove-row = 行を削除
ui-chart-add-row = + 行
ui-chart-add-column = + 列
ui-chart-series = 系列 { $n }

## ファイル

ui-file-open-prompt = 開くファイルのパス（.echo.pdf または .json）
ui-file-save-prompt = 名前を付けて保存（フルパス）
