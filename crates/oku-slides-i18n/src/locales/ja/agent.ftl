# エージェント：サーフェス、確認、スレッド。

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
agent-confirm-edit-theme = テーマ「{ $name }」の新しいバージョンを保存しますか？
agent-confirm-copy-theme = 組み込みテーマ「{ $name }」の編集済みコピーを保存しますか？
agent-confirm-setting = { $key }を変更しますか：{ $old } → { $new }

agent-not-found = エージェントが見つかりません：{ $command }。設定 [agent.presets]。
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
agent-error-draft-no-agent = このタブのエージェントはまだ起動していません：下書きを拒否しました。
agent-error-unknown-agent = このエージェントは設定にもうありません。
agent-error-agent-locked = エージェントの選択は環境によって固定されています。
agent-error-agent-not-saved = エージェントの選択を保存できませんでした。
agent-picker-title = エージェント
agent-picker-busy = エージェントを切り替えると、進行中のターンが中断されます。
agent-picker-locked = 環境によって指定されています (OKU_SLIDES_AGENT__SELECTED)。
