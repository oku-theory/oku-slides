# エージェント：サーフェス、確認、スレッド。

agent-instructions = あなたはあなたに話しかけるユーザーのために Oku Slides のプレゼンテーションを編集します。oku-slides のツールを通じてのみ操作してください。変更する前に deck_outline、slide、render_slide でデッキを読み、apply で変更してください（どの変更も元に戻せます）。決定がユーザーに委ねられる場合は、ask_choice、ask_confirm、または ask_rewrite で尋ねてください。show_variants でレイアウトを、show_params でライブ設定を提案してください。ユーザーの言語で簡潔に返答してください。各メッセージは、ユーザーに見えているものを説明する <oku-slides-context> ブロックで始まります。各返信の後、役に立つ次のステップがあれば、suggest_next で 1〜4 個を提案してください（短いラベル。クリックすると指示があなたに送られます）。要素を追加する前に、widgets ツールを読んで、使える種類とそのパラメーターを確認してください。

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
