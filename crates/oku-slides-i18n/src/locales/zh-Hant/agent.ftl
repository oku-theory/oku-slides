# AI 代理：介面、確認、對話串。

agent-surface-cancel = 取消
agent-surface-confirm = 確認
agent-surface-confirm-danger = 是，執行
agent-surface-rewrite = 改寫
agent-surface-pick = 選擇{ $n }
agent-surface-keep = 保留原文
agent-surface-edit = 套用前編輯
agent-surface-close = 關閉
agent-surface-variants = 變體
agent-surface-permission-always = 永久
agent-surface-permission-title = AI 代理請求一項權限
agent-undo-turn = AI 代理：{ $text }

agent-tools-unavailable = AI 代理的工具未能啟動。

agent-undo-apply = AI 代理變更
agent-undo-variant = 變體：{ $label }
agent-undo-param = 設定：{ $label }
agent-undo-rewrite = 改寫

agent-confirm-save = 要將簡報另存為{ $path }嗎？
agent-confirm-save-over = 要取代{ $path }嗎？現有檔案將被覆寫。
agent-confirm-open = 要開啟{ $path }嗎？
agent-open-elsewhere = 此檔案已在另一個分頁中開啟。
agent-confirm-new = 要開始新的簡報嗎？
agent-confirm-new-dirty = 要開始新的簡報嗎？未儲存的變更將遺失。
agent-confirm-save-templates = 要儲存範本組「{ $name }」嗎？
agent-confirm-delete-set = 要刪除範本組「{ $name }」嗎？此操作無法復原。
agent-confirm-setting = 要變更{ $key }：{ $old } → { $new }嗎？

agent-not-found = 找不到 AI 代理：{ $command }。設定 [agent] command。
agent-no-http = 此 AI 代理無法連線到 Oku Slides 的工具。
agent-stopped = AI 代理已停止。
agent-failed = AI 代理拒絕了請求：{ $message }
agent-tool-failed = 此工具意外失敗。

agent-turn-tools = { $tools }個工具
agent-turn-notes = { $notes }則備註
agent-turn-secs = { $secs }秒
agent-turn-stopped = 已停止。
agent-turn-interrupted = 已中斷。
agent-memory-lost = 代理已不記得這段對話：將從頭開始。
agent-turn-max-tokens = AI 代理已達到長度上限。
agent-turn-max-turns = AI 代理已達到步數上限。
agent-turn-refusal = AI 代理已拒絕。
agent-turn-ended = 本輪結束（{ $stop }）。
agent-choice-cancelled = 問題已取消
agent-choice-answered = ✓ { $answer }
agent-report-why = 原因

agent-error-file = 無法讀取或寫入該檔案。
agent-error-unexpected-action = 非預期的動作：{ $action }。
agent-error-unknown-surface = 此卡片已不再開啟。
agent-error-expired = 過去回合的動作已過期。
agent-error-incomplete = 此卡片的動作不完整。
agent-error-no-field = 此設定已不存在。
agent-error-no-variant = 此變體已不存在。
agent-error-arguments = 工具的參數無效（{ $detail }）：請依照其結構描述檢查。
agent-error-unknown-tool = 未知的工具：{ $name }。
agent-error-denied = 使用者已拒絕。
agent-error-no-turn = 目前沒有進行中的回合：{ $tool } 只會在回合中顯示。
agent-error-turn-over = 此呼叫所屬的回合已結束：未顯示任何內容。
agent-error-anchor-scope = 小工具對話中的問題不能錨定到投影片：請不帶錨點提問。
agent-error-no-file = 簡報還沒有檔案：請提供絕對路徑。
agent-error-path-relative = 路徑必須是絕對路徑。
agent-error-path-parent = 路徑不能包含「..」。
agent-error-setting-locked = 代理無法變更設定 { $key }。
agent-error-setting = 無法變更設定 { $key }：請檢查其值。
agent-error-rewrite-text = 改寫傳回時沒有文字。
agent-error-paragraphs = 改寫的段落無效。
agent-error-draft-too-long = 草稿太長。
agent-error-draft-origin = 草稿被拒絕：未知的檢視。
agent-error-draft-no-agent = 此分頁的代理尚未啟動：草稿被拒絕。
