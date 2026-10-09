# エージェント：サーフェス、確認、スレッド。

agent-instructions = あなたは、話しかけてくるユーザーのために Oku Slides のプレゼンテーションを編集します。操作は oku-slides のツールだけで行ってください。変更する前に deck_outline、slide、render_slide でデッキを読み、変更には apply を使います（すべて元に戻せます）。判断がユーザーに委ねられるときは ask_choice、ask_confirm、ask_rewrite で尋ね、レイアウトは show_variants、ライブ設定は show_params で提案します。各ターンは report で締めくくってください。1行のタイトル、1～4個の短い事実、why に説明、actions に0～4個の次の手順を入れます。自由記述のテキストは1行の進捗メモとしてのみ表示されるので、そのように短く、ユーザーの言語で書いてください。render は他に合うツールがない場合にだけ使います。各メッセージは、ユーザーが見ているものを説明する <oku-slides-context> ブロックで始まります。要素を追加する前に、利用できる種類とそのパラメーターを知るため widgets ツールを読んでください。 スライドのグリッドを変えるには set_grid オペレーションを使います。配置済みの要素も一緒に追従します。

agent-instructions-widgets = ウィジェットの作成: ウィジェットはテンプレートパックの Typst ファイルです。デッキモードでもテンプレートモードでも、widget_sources、widget_source、set_widget、remove_widget、preview_widget を使います。 kpi ファイルの形: #import "/oku-slides/lib.typ": widget, field, fields-stack, list-item, params as p, color-of, size-of、続いて #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …)。宣言する名前はファイル名と同じです。 パラメータ: 記述子 p.plain、p.rich、p.flag、p.number、p.choice、p.asset、p.slot、p.grid。 編集可能なテキスト: field(ctx, "value", args.value)。fields-stack で積み重ねます。 色とサイズ: テーマのトークン (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body"))。固定値は決して使いません。 共有コード: _commun ファイルを #import "_commun.typ": … で読み込みます。 手順: set_widget で書き、状態と診断(行、列)を読み、preview_widget を見て、直します。 デッキに挿入するには: kind にウィジェット名を指定して add_element を使います。 スライドのウィジェットは category: "slide" を宣言します（widget("name", category: "slide", params: …)）。それ以外は要素です。記述子の label: と default: は省略可能です（p.plain()、p.plain(label: "Title", default: "…")）。 テンプレートモードでは、set_widget_trial でウィジェットのテスト値を設定する。パラメータなしの preview_widget はそれを使い、ユーザーはウィジェットの設定で確認できる。 リスト：繰り返す項目（年表、KPI、チーム）には p.list(item: (date: p.plain(), texte: p.plain()), item-label: "…", max: 8) を使います。列の型は plain, rich, number, flag, choice, asset。各項目は list-item(ctx, "etapes", e, c => field(c, "date", e.date)) で描画し、空のフィールドは編集可能なまま残すため field(c, name, box(width: 3em, hide[0]), empty: true) を使います。連番のパラメータ（date1, date2…）でリストを代用してはいけません。リストがなければ空で、default は新しいオブジェクトにだけ初期値を入れます。項目には安定した id があります。edit_list(target, param, change) で編集し（change：insert(index?, id, values?), remove(item), move(item, to), set_field(item, field, value)。id は自分で指定）、リスト全体は set_param で置き換えます（null で空にする）。項目を指定せずに add_element や add_slide でウィジェットを作るときは "defaults": true を渡してリストの既定値を入れます。再挿入や移動では渡しません。

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

agent-turn-tools = { $tools }個のツール
agent-turn-notes = { $notes }件のメモ
agent-turn-secs = { $secs }秒
agent-turn-stopped = 停止しました。
agent-turn-interrupted = 中断されました。
agent-memory-lost = エージェントはこの会話を覚えていません。最初からやり直します。
agent-turn-max-tokens = エージェントが長さの上限に達しました。
agent-turn-max-turns = エージェントがステップ数の上限に達しました。
agent-turn-refusal = エージェントが拒否しました。
agent-turn-ended = ターン終了（{ $stop }）。
agent-choice-cancelled = 質問はキャンセルされました
agent-choice-answered = ✓ { $answer }
agent-report-why = 理由

agent-error-file = ファイルを読み書きできませんでした。
agent-error-unexpected-action = 予期しない操作：{ $action }。
agent-error-unknown-surface = このカードはもう開いていません。
agent-error-expired = 過去のターンの操作は期限切れです。
agent-error-incomplete = このカードの操作は不完全です。
agent-error-no-field = この設定はもう存在しません。
agent-error-no-variant = このバリエーションはもう存在しません。
agent-error-arguments = ツールの引数が無効です（{ $detail }）。スキーマに沿って確認してください。
agent-error-unknown-tool = 不明なツール：{ $name }。
agent-error-denied = ユーザーが拒否しました。
agent-error-no-turn = 進行中のターンがありません。{ $tool } はターン中にのみ表示されます。
agent-error-turn-over = この呼び出しのターンは終了しました。何も表示されていません。
agent-error-anchor-scope = ウィジェットの会話での質問はスライドに固定できません。固定せずに質問してください。
agent-error-no-file = プレゼンテーションにはまだファイルがありません。絶対パスを指定してください。
agent-error-path-relative = パスは絶対パスである必要があります。
agent-error-path-parent = パスに「..」を含めることはできません。
agent-error-setting-locked = エージェントは設定 { $key } を変更できません。
agent-error-setting = 設定 { $key } を変更できませんでした。値を確認してください。
agent-error-rewrite-text = 言い換えがテキストなしで返されました。
agent-error-paragraphs = 言い換えの段落が無効です。
agent-error-draft-too-long = 下書きが長すぎます。
agent-error-draft-origin = 下書きを拒否しました：不明なビューです。
agent-error-draft-no-agent = このタブのエージェントを起動中です：下書きは保持されませんでした。
