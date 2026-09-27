# 代理：介面、確認、對話串。

agent-instructions = 你為與你對話的使用者編輯一份 Oku Slides 簡報。只透過 oku-slides 工具行動。修改前，先用 deck_outline、slide 和 render_slide 讀取投影片組；用 apply 進行修改（每次修改都可以復原）。當決定權在使用者時，用 ask_choice、ask_confirm 或 ask_rewrite 詢問；用 show_variants 提供版面配置，用 show_params 提供即時設定。用使用者的語言簡短回覆。每則訊息都以描述使用者所見內容的 <oku-slides-context> 區塊開頭。

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
agent-surface-permission-title = 智慧代理請求一項權限
agent-undo-turn = 代理：{ $text }

agent-tools-unavailable = 代理的工具未能啟動。

agent-undo-apply = 代理變更
agent-undo-variant = 變體：{ $label }
agent-undo-param = 設定：{ $label }
agent-undo-rewrite = 改寫

agent-confirm-save = 要將簡報另存為{ $path }嗎？
agent-confirm-save-over = 要取代{ $path }嗎？現有檔案將被覆寫。
agent-confirm-open = 要開啟{ $path }嗎？
agent-confirm-open-dirty = 要開啟{ $path }嗎？未儲存的變更將遺失。
agent-open-elsewhere = 此檔案已在另一個分頁中開啟。
agent-confirm-new = 要開始新的簡報嗎？
agent-confirm-new-dirty = 要開始新的簡報嗎？未儲存的變更將遺失。
agent-confirm-save-templates = 要儲存範本組「{ $name }」嗎？
agent-confirm-delete-set = 要刪除範本組「{ $name }」嗎？此操作無法復原。
agent-confirm-setting = 要變更{ $key }：{ $old } → { $new }嗎？

agent-not-found = 找不到代理：{ $command }。設定 [agent] command。
agent-no-http = 此代理無法連線到 Oku Slides 的工具。
agent-stopped = 代理已停止。
agent-failed = 代理拒絕了請求：{ $message }
