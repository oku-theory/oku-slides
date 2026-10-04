# エージェント：サーフェス、確認、スレッド。

agent-instructions = あなたは、話しかけてくるユーザーのために Oku Slides のプレゼンテーションを編集します。操作は oku-slides のツールだけで行ってください。変更する前に deck_outline、slide、render_slide でデッキを読み、変更には apply を使います（すべて元に戻せます）。判断がユーザーに委ねられるときは ask_choice、ask_confirm、ask_rewrite で尋ね、レイアウトは show_variants、ライブ設定は show_params で提案します。各ターンは report で締めくくります。1 行のタイトル、短い事実を 1 ～ 4 個、why に説明、actions に次の一手を 0 ～ 4 個です。自由記述のテキストは 1 行の進捗メモとしてのみ表示されるので、そのように短く、ユーザーの言語で書いてください。render は他に合うツールがない場合にだけ使います。各メッセージは、ユーザーが見ているものを説明する <oku-slides-context> ブロックで始まります。要素を追加する前に、利用できる種類とそのパラメーターを知るため widgets ツールを読んでください。 スライドのグリッドを変えるには set_grid オペレーションを使います。配置済みの要素も一緒に追従します。

agent-instructions-widgets = ウィジェットの作成: ウィジェットはテンプレートパックの Typst ファイルです。デッキモードでもテンプレートモードでも、widget_sources、widget_source、set_widget、remove_widget、preview_widget を使います。 kpi ファイルの形: #import "/oku-slides/lib.typ": widget, field, fields-stack, params as p, color-of, size-of、続いて #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …)。宣言する名前はファイル名と同じです。 パラメータ: 記述子 p.plain、p.rich、p.flag、p.number、p.choice、p.asset、p.slot、p.grid。 編集可能なテキスト: field(ctx, "value", args.value)。fields-stack で積み重ねます。 色とサイズ: テーマのトークン (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body"))。固定値は決して使いません。 共有コード: _commun ファイルを #import "_commun.typ": … で読み込みます。 手順: set_widget で書き、状態と診断(行、列)を読み、preview_widget を見て、直します。 デッキに挿入するには: kind にウィジェット名を指定して add_element を使います。 スライドのウィジェットは category: "slide" を宣言します（widget("name", category: "slide", params: …)）。それ以外は要素です。記述子の label: と default: は省略可能です（p.plain()、p.plain(label: "Title", default: "…")）。 テンプレートモードでは、set_widget_trial でウィジェットのテスト値を設定する。パラメータなしの preview_widget はそれを使い、ユーザーはウィジェットの設定で確認できる。

agent-surface-cancel = キャンセル
agent-surface-confirm = 確認
agent-surface-confirm-danger = はい、実行します
agent-surface-rewrite = 言い換え
agent-surface-pick = { $n } を選ぶ
agent-surface-keep = 元の文章を保持
agent-surface-edit = 適用前に編集
agent-surface-close = 閉じる
agent-surface-variants = バリエーション
agent-surface-permission-always = 常に
agent-surface-permission-title = エージェントが許可を求めています
agent-undo-turn = エージェント：{ $text }

agent-tools-unavailable = エージェントのツールを起動できませんでした。

agent-undo-apply = エージェントの変更
agent-undo-variant = バリエーション：{ $label }
agent-undo-param = 設定：{ $label }
agent-undo-rewrite = 言い換え

agent-confirm-save = プレゼンテーションを{ $path }として保存しますか？
agent-confirm-save-over = { $path }を置き換えますか？既存のファイルは上書きされます。
agent-confirm-open = { $path }を開きますか？
agent-open-elsewhere = このファイルは別のタブですでに開かれています。
agent-confirm-new = 新しいプレゼンテーションを開始しますか？
agent-confirm-new-dirty = 新しいプレゼンテーションを開始しますか？保存されていない変更は失われます。
agent-confirm-save-templates = テンプレートセット「{ $name }」を保存しますか？
agent-confirm-delete-set = テンプレートセット「{ $name }」を削除しますか？元に戻せません。
agent-confirm-setting = { $key }を変更しますか：{ $old } → { $new }

agent-not-found = エージェントが見つかりません：{ $command }。設定 [agent] command。
agent-no-http = このエージェントはOku Slidesのツールに接続できません。
agent-stopped = エージェントが停止しました。
agent-failed = エージェントが要求を拒否しました: { $message }
agent-tool-failed = ツールが予期せず失敗しました。

agent-turn-tools = { $tools } 個のツール
agent-turn-notes = { $notes } 件のメモ
agent-turn-secs = { $secs } 秒
agent-turn-stopped = 停止しました。
agent-turn-interrupted = 中断されました。
agent-turn-max-tokens = エージェントが長さの上限に達しました。
agent-turn-max-turns = エージェントがステップ数の上限に達しました。
agent-turn-refusal = エージェントが断りました。
agent-turn-ended = ターン終了 ({ $stop })。
agent-choice-cancelled = 質問はキャンセルされました
agent-choice-answered = ✓ { $answer }
agent-report-why = 理由
