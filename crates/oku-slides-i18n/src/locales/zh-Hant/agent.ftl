# AI 代理：介面、確認、對話串。

agent-surface-cancel = 取消
agent-surface-confirm = 確認
agent-surface-confirm-danger = 是，執行
agent-surface-heard = 聽到：「{ $text }」
agent-surface-say-or-click = 請說出回答，或點擊按鈕。
agent-surface-said-too-early = 說得太早：請等視窗出現後再說一次。
agent-surface-click-to-confirm = 請點擊按鈕確認。
agent-surface-verb-replace = 取代
agent-surface-verb-merge = 合併
agent-surface-verb-discard = 捨棄
agent-surface-verb-delete = 刪除
agent-surface-verb-use-api = 使用 api
agent-surface-verb-install = 安裝
agent-surface-verb-quit = 退出應用程式
agent-surface-verb-close = 關閉
agent-surface-verb-import = 匯入
agent-surface-rewrite = 改寫
agent-surface-pick = 選擇{ $n }
agent-surface-keep = 保留原文
agent-surface-edit = 套用前編輯
agent-surface-close = 關閉
agent-surface-themes = 主題
agent-surface-variants = 變體
agent-surface-permission-always = 永久
agent-surface-permission-title = AI 代理請求一項權限
agent-surface-permission-guides = 本輪包含收到的 { $names } 指南。
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
agent-confirm-copy-pack = 要儲存套件「{ $name }」的副本嗎？套件本身保持不變。
agent-confirm-copy-edited-theme = 要儲存主題「{ $name }」的副本嗎？主題本身保持不變。
agent-confirm-delete-pack = 要刪除套件「{ $name }」嗎？此操作無法復原。
agent-confirm-edit-theme = 要儲存主題「{ $name }」的新版本嗎？
agent-confirm-copy-theme = 要建立內建主題「{ $name }」的變體嗎？內建主題保持不變。
agent-confirm-setting = 要變更{ $key }：{ $old } → { $new }嗎？
agent-confirm-move = 將簡報移到 { $path }？舊檔案將被刪除。
agent-confirm-merge = 將 { $path } 合併到此簡報？此操作無法復原。
agent-confirm-save-copy = 將簡報的副本另存為 { $path }？
agent-confirm-dictation-api = 要將您的聽寫內容傳送到 { $host }（{ $url }），模型 { $model }，並使用 { $var } 中的金鑰嗎？
agent-confirm-dictation-download = 要使用 { $name } 並立即下載嗎（{ $size }）？
agent-confirm-dictation-language = 要使用 Apple 聽寫並立即下載其語言嗎？
agent-confirm-agent = 要在沒有回覆執行時將應用程式代理切換為 { $name }（{ $command }）嗎？
agent-confirm-agent-builtin = 要在沒有回覆執行時將應用程式代理切換為 { $name } 嗎？
agent-confirm-agent-model = 要下載本機代理模型嗎（{ $size }）？

agent-not-found = 找不到 AI 代理：{ $command } 不在該路徑，也不在應用程式的 PATH 中。請在 [agent.presets] 中填寫完整路徑。
agent-spawn-failed = 無法啟動 AI 代理：{ $command }。設定 [agent.presets]。
agent-no-http = 此 AI 代理無法連線到 Oku Slides 的工具。
agent-local-no-program = 本機 AI 代理只能在 Oku Slides 應用程式內執行。
agent-local-needs-download = 本機代理程式需要先下載其模型({ $gigabytes }GB)。
agent-local-intel-mac = 本機代理程式需要搭載 Apple 晶片的 Mac。
agent-local-unsupported-os = 本機代理程式目前尚不支援此系統。
agent-local-apple-off = 本機代理程式需要 Apple Intelligence：請在「系統設定」中開啟。
agent-local-apple-not-ready = Apple 的模型仍在準備中：請幾分鐘後再試一次本機代理程式。
agent-local-apple-unsupported = Apple 的模型需要支援 Apple Intelligence 且搭載 macOS 26 或以上版本的 Mac。
agent-model-download = 下載模型
agent-model-downloading = 正在下載模型… { $percent }%
agent-model-cancel = 取消
agent-model-download-failed = 無法下載模型。
agent-model-download-integrity = 下載的模型已損壞，已被刪除。
agent-model-download-incomplete = 下載不完整：已收到 { $got }位元組，共 { $expected }位元組。
agent-model-download-cancelled = 已取消下載。
agent-fallback-notice = 本機 AI 代理 — { $reason }
agent-fallback-download = { $reason } 下載其模型({ $model }，{ $gigabytes }GB)後，本機 AI 代理即可接手。現在下載嗎？
agent-fallback-unusable = 沒有可執行的 AI 代理。主要 AI 代理：{ $main }；本機 AI 代理：{ $local }
local-agent-no-route = 本機 AI 代理不了解要做什麼。請換個說法再試一次。
local-agent-tool-limit = 本機 AI 代理執行了{ $n }個步驟後停止，未能完成。
local-agent-invalid-call = 本機 AI 代理無法正確使用工具。
local-agent-engine-failed = 本機 AI 代理的模型發生錯誤。
local-agent-retry = 請重試，或在 AI 代理面板中選擇其他 AI 代理。
agent-stopped = AI 代理已停止。
agent-failed = AI 代理拒絕了請求：{ $message }
agent-error-work-dir = 無法為此分頁準備 AI 代理的專用工作資料夾。
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
agent-trace = { $name }：{ $count ->
   *[other] { $count }處變更
}
agent-trace-data = { $name }：資料已變更
agent-trace-removed = { $name }：已不存在
agent-trace-more = { $count ->
   *[other] 及其他{ $count }項
}
agent-trace-title = 已變更的元素

agent-error-file = 無法讀取或寫入該檔案。
agent-error-unexpected-action = 非預期的動作：{ $action }。
agent-error-unknown-surface = 此卡片已不再開啟。
agent-error-expired = 過去回合的動作已過期。
agent-error-incomplete = 此卡片的動作不完整。
agent-error-no-field = 此設定已不存在。
agent-error-no-variant = 此變體已不存在。
agent-error-arguments = 工具的參數無效（{ $detail }）：請依照其結構描述檢查。
agent-error-unknown-tool = 未知的工具：{ $name }。
agent-error-pack-id = 這不是套件的 id：請呼叫 packs 列出套件及其 id。
agent-error-asset-unknown = 此圖片既不在簡報中，也不在已貼上的圖片中：請使用已貼上圖片的 ID。
agent-error-denied = 使用者已拒絕。
agent-error-no-turn = 目前沒有進行中的回合：{ $tool } 只會在回合中顯示。
agent-error-tab-no-turn = 只在回答使用者時開啟、顯示或移動分頁。
agent-error-tab-limit = 你在本輪已經開啟、顯示或移動過一個分頁；請在那裡完成，或在處理另一個之前詢問使用者。
agent-error-continue-in-other-tab = 此請求會在你已開啟或顯示的分頁中繼續：請讓使用者在那裡繼續；此分頁暫時不會修改任何小工具。
agent-error-pack-open-in-tab = 此套件正在另一個分頁中編輯：請讓使用者先在那裡完成。
agent-pack-opened-in-tab = { $shown ->
   [yes] 該套件已在分頁 { $tab } 中開啟，現在正在顯示。
  *[no] 該套件已在分頁 { $tab } 中於背景開啟：使用者的視窗正顯示另一個分頁。
} { $draft ->
   [set] 該請求已放入該分頁的草稿中：請讓使用者在那裡檢查後再傳送。
   [kept] 該分頁的草稿中已有內容，保持不變。請讓使用者在那裡繼續：該分頁的代理不知道此請求。
   [too_long] 該請求過長，無法放入該分頁的草稿。請讓使用者在那裡繼續：該分頁的代理不知道此請求。
   [unavailable] 無法將該請求放入該分頁的草稿。請讓使用者在那裡繼續：該分頁的代理不知道此請求。
  *[none] 請讓使用者在那裡繼續：該分頁的代理不知道此請求。
} 此分頁保持不變。
agent-theme-opened-in-tab = { $shown ->
   [yes] 該主題已在分頁 { $tab } 中開啟，現在正在顯示。
  *[no] 該主題已在分頁 { $tab } 中於背景開啟：使用者的視窗正顯示另一個分頁。
} { $draft ->
   [set] 該請求已放入該分頁的草稿中：請讓使用者在那裡檢查後再傳送。
   [kept] 該分頁的草稿中已有內容，保持不變。請讓使用者在那裡繼續：該分頁的代理不知道此請求。
   [too_long] 該請求過長，無法放入該分頁的草稿。請讓使用者在那裡繼續：該分頁的代理不知道此請求。
   [unavailable] 無法將該請求放入該分頁的草稿。請讓使用者在那裡繼續：該分頁的代理不知道此請求。
  *[none] 請讓使用者在那裡繼續：該分頁的代理不知道此請求。
} 此分頁保持不變。
agent-error-turn-over = 此呼叫所屬的回合已結束：未顯示任何內容。
agent-error-no-box-turn = 沒有可關聯此方塊的回合：未顯示任何內容。
agent-error-no-file = 簡報還沒有檔案：請提供絕對路徑。
agent-error-path-relative = 路徑必須是絕對路徑。
agent-error-path-parent = 路徑不能包含「..」。
agent-error-setting-locked = 代理無法變更設定 { $key }。
agent-error-setting = 無法變更設定 { $key }：請檢查其值。
agent-error-history-index = 瀏覽歷史記錄需要版本的序號。
agent-error-no-pack = 此簡報未固定任何套件：請提供 pack（參見 packs）。
agent-error-history-not-opened = 分頁未能及時開啟歷史記錄：請重新請求。
agent-error-locale = 未知的語言代碼。請提供內建語言代碼或 system。
agent-error-empty-interface = 沒有要變更的內容：請提供 scheme、locale、guides、folded、pile、notes 或 code。
agent-error-layout-range = { $field } 必須介於 { $min } 到 { $max } px 之間。
agent-error-layout-min = { $field } 至少為 { $min } px。
agent-error-empty-dictation = 沒有要變更的內容：請至少提供一項聽寫設定。
agent-error-empty-speech = 沒有要變更的內容：請至少提供一項朗讀設定。
agent-error-not-shown = 此分頁不是其視窗中顯示的分頁：請在使用者顯示它後再請求。
agent-error-not-presenting = 這裡沒有在簡報這份投影片：請先用 start 動作開始簡報。
agent-error-present-slide = 要顯示投影片，需要提供這份投影片中某張投影片的 id（見 deck_outline）。
agent-error-undo-open-turn = undo_turn 不會復原正在進行的輪次：本輪自己的變更請使用 undo。
agent-error-app-unavailable = 只有 Oku Slides 應用程式能做到這一點；這裡無法使用。
agent-error-app-closing = Oku Slides 已在關閉。
agent-error-app-no-offer = 這裡沒有可以延後或安裝的更新：請先檢查更新。
agent-error-app-failed = Oku Slides 更新失敗：請檢查網路連線，或使用更新橫幅。
agent-error-after-turn-taken = 這次回覆已經會以結束應用程式、安裝更新或關閉它所在的分頁收尾。
agent-error-no-pack-update = 套件面板沒有為此套件提供這樣的更新：請參閱 show_packs。
agent-confirm-update-install = 要安裝 Oku Slides { $version } 嗎？這次回覆結束後應用程式會重新啟動。
agent-after-turn-quit-unsaved = Oku Slides 沒有結束：自你提出要求以來，有分頁出現了未儲存的變更。
agent-error-workspace-unavailable = 只有 Oku Slides 應用程式能變更分頁和視窗；此處無法使用。
agent-error-workspace-unknown-tab = 此視窗沒有該分頁或視窗。
agent-error-workspace-unknown-window = 沒有該視窗。
agent-error-workspace-index = 該位置超出了分頁列的末端。
agent-error-workspace-closing = Oku Slides 正在關閉：分頁和視窗不再變化。
agent-confirm-close-tab = 要關閉分頁「{ $tab }」嗎？
agent-confirm-discard-tab = 要關閉分頁「{ $tab }」嗎？它未儲存的變更將遺失。
agent-confirm-close-window = 要關閉此視窗及其分頁嗎？
agent-confirm-discard-window = 要關閉此視窗嗎？{ $tabs } 中未儲存的變更將遺失。
agent-error-window-last = 這是唯一的視窗：請改為關閉它的分頁，或結束 Oku Slides。
agent-error-tab-index = 請說明要把分頁放在哪裡：重新排序需要一個位置。
agent-error-tab-unused = 只有重新排序和移動接受位置，只有移動接受視窗。
agent-after-turn-close-unsaved = 分頁沒有關閉：自你提出要求以來它已有變更。
agent-after-turn-close-not-shown = 分頁沒有關閉：回覆結束時，它的視窗顯示的是另一個分頁。
agent-confirm-quit = 結束 Oku Slides？這次回覆結束後應用程式就會結束。
agent-error-quit-unsaved = 有未儲存內容的分頁：{ $count }。Oku Slides 沒有結束：請使用者先儲存或關閉它們。
agent-error-tune-unavailable = 微調無法開始：代理程式無法使用。
agent-error-tune-not-started = 微調沒有開始：該分頁的代理程式暫時無法使用；該分頁會說明原因。
agent-confirm-import = 要將 { $path } 匯入到新分頁嗎？
agent-import-opened-in-tab = { $shown ->
   [yes] 簡報已在分頁 { $tab } 中開啟，現在正在顯示。
  *[no] 簡報已在分頁 { $tab } 中於背景開啟：使用者的視窗顯示的是另一個分頁。
} { $tune ->
   [started] 它的微調在那裡開始，在該分頁自己的對話中。
   [unavailable] 它的微調無法開始：請告訴使用者在該分頁中提出。
  *[no] 請告訴使用者在那裡繼續。
} 此分頁保持原樣。
agent-error-import-hifi-target = 高保真匯入不接受套件或主題：請移除它們，或以 retrofit 模式匯入。
agent-error-import-extension = 只能匯入 PowerPoint .pptx 檔案。
agent-error-a2ui-unknown = { $name } 不是 render 接受的元件。元件：{ $known }。
agent-error-a2ui-elsewhere = render 不顯示 { $name }：請使用 { $tool }。
agent-error-a2ui-host-only = { $name } 保留給應用程式：render 無法顯示它。
agent-error-themes-deck-mode = 主題選擇器只在編輯投影片時開啟。
agent-error-packs-deck-mode = 簡報的套件面板只在編輯投影片時開啟。
agent-error-usage-no-deck = 此分頁的用量不屬於任何簡報：只有從簡報開啟的套件分頁或主題分頁（無論由使用者還是 AI 代理開啟），或簡報自己的套件，才計入該簡報。
agent-error-rewrite-text = 改寫傳回時沒有文字。
agent-error-paragraphs = 改寫的段落無效。
agent-error-draft-too-long = 草稿太長。
agent-error-report-elsewhere = 此視窗未顯示該分頁。
agent-error-ui-too-large = 頁面內容脈絡過大。
agent-error-seq-exhausted = 頁面報告編號超出範圍。
agent-error-draft-origin = 草稿被拒絕：未知的檢視。
agent-error-draft-no-agent = 此分頁的代理尚未啟動：草稿被拒絕。
agent-error-too-many-images = 草稿已包含所有圖片。
agent-error-image-not-base64 = 貼上的圖片沒有完整送達。
agent-error-image-gone = 貼上的圖片已無法使用：請重新貼上。
agent-error-unknown-agent = 此代理已不在設定中。
agent-error-agent-locked = 代理的選擇由環境指定。
agent-error-agent-not-saved = 無法儲存代理的選擇。
agent-error-unknown-model = 此代理不提供該模型。
agent-error-model-not-saved = 無法儲存模型的選擇。
voice-error-no-dictation = 此處無法使用聽寫
voice-error-busy = 聽寫正在進行：請在閒置後重試
voice-error-unknown-model = 未知的聽寫模型
voice-error-unavailable = 此模型不支援所選語言
voice-error-installed = 此模型已安裝
voice-error-not-downloadable = 此模型無需下載
voice-error-nothing-to-cancel = 沒有正在進行的下載
voice-error-bad-silence = 靜音時長須為 0.5 至 10秒
voice-error-bad-language = 未知的聽寫語言
voice-error-bad-url = API 位址必須以 https:// 開頭（http:// 僅限本機），不得包含使用者名稱、參數或特殊字元
voice-error-bad-key-env = 金鑰變數必須是類似 OPENAI_API_KEY 的名稱：大寫字母、數字和 _
voice-error-bad-api-model = API 模型名稱只能使用字母、數字和 . _ : / -；Azure 部署名稱只能使用字母、數字和 . _ -
voice-error-bad-api-version = Azure API 版本只能使用字母、數字、. 和 -，最多32個字元，例如 2025-03-01-preview
voice-error-bad-api-headers = 標頭寫作 Name: value; Name: $VARIABLE，最多8個。Authorization, api-key, Host, Content-Type, Content-Length, Transfer-Encoding, Connection 和重複項會被拒絕
voice-error-api-reserved = 只有使用者能設定 API 的位址、金鑰變數、Azure 版本和標頭：請用 show_dictation 開啟設定框並請使用者設定
voice-error-hands-free-reserved = 只有使用者才能開啟免持聆聽：請用 show_dictation 開啟設定框並請使用者操作
voice-error-changed = 提問之後設定已變更：請重新詢問
voice-error-nothing-to-download = 此代理沒有需要下載的內容
voice-error-bad-rate = 語速必須介於 0.5 到 2 之間
voice-error-unknown-voice = 介面語言的未知語音
voice-error-speech-unavailable = 這部電腦無法使用朗讀
agent-tune-prompt = 微調匯入的簡報
agent-settings-unknown-selected = 設定 agent.selected = { $selected } 未指向 [agent.presets] 中的任何 AI 代理（已知：{ $known }）
agent-settings-preset-launch = AI 代理 [agent.presets.{ $id }] 需要且僅需要 command 或 builtin 其中之一
