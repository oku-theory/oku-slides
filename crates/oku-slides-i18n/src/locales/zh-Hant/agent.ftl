# 代理：介面、確認、對話串。

agent-instructions = 你為與你對話的使用者編輯一份 Oku Slides 簡報。只透過 oku-slides 工具行事。修改之前，先用 deck_outline、slide 和 render_slide 閱讀投影片組；用 apply 修改（一切都可以復原）。當決定權在使用者時，用 ask_choice、ask_confirm 或 ask_rewrite 詢問；用 show_variants 提供版面，用 show_params 提供即時設定。每一輪都以 report 結束：一行標題、1 到 4 條簡短事實、在 why 中寫你的說明，以及在 actions 中寫 0 到 4 個後續步驟。你的自由文字只會顯示為一行進度說明：請照此簡短書寫，並使用使用者的語言。只有在沒有其他工具適用時才使用 render。每則訊息都以描述使用者所見內容的 <oku-slides-context> 區塊開頭。新增元素之前，先閱讀 widgets 工具，了解可用的種類及其參數。

agent-instructions-widgets = 建立元件:元件是範本包中的一個 Typst 檔案;在簡報模式與範本模式下都使用 widget_sources、widget_source、set_widget、remove_widget 和 preview_widget。 kpi 檔案的形式:#import "/oku-slides/lib.typ": widget, field, fields-stack, params as p, color-of, size-of,然後 #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …);宣告的名稱就是檔名。 參數:描述符 p.plain、p.rich、p.flag、p.number、p.choice、p.asset、p.slot、p.grid。 可編輯文字:field(ctx, "value", args.value),以 fields-stack 堆疊。 顏色與大小:主題權杖 (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")),絕不寫死數值。 共用程式碼:_commun 檔案,以 #import "_commun.typ": … 匯入。 迴圈:用 set_widget 寫入,讀取狀態及其診斷(行、列),查看 preview_widget,修正。 要把它放進簡報:使用 add_element,kind 為元件名稱。 投影片小工具宣告 category: "slide"（widget("name", category: "slide", params: …)）；否則為元素。描述子中的 label: 與 default: 可省略（p.plain()、p.plain(label: "標題", default: "…")）。

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
agent-tool-failed = 此工具意外失敗。

agent-turn-tools = { $tools } 個工具
agent-turn-notes = { $notes } 則備註
agent-turn-secs = { $secs } 秒
agent-turn-stopped = 已停止。
agent-turn-interrupted = 已中斷。
agent-turn-max-tokens = 智慧代理已達到長度上限。
agent-turn-max-turns = 智慧代理已達到步數上限。
agent-turn-refusal = 智慧代理拒絕了。
agent-turn-ended = 本輪結束（{ $stop }）。
agent-choice-cancelled = 問題已取消
agent-choice-answered = ✓ { $answer }
agent-report-why = 原因
