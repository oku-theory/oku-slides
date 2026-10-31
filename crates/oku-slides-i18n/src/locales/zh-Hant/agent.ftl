# AI 代理：介面、確認、對話串。

agent-surface-cancel = 取消
agent-surface-confirm = 確認
agent-surface-confirm-danger = 是，執行
agent-surface-rewrite = 改寫
agent-surface-pick = 選擇{ $n }
agent-surface-keep = 保留原文
agent-surface-edit = 套用前編輯
agent-surface-close = 關閉
agent-surface-themes = 主題
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

agent-not-found = 找不到 AI 代理：{ $command }。設定 [agent.presets]。
agent-spawn-failed = 無法啟動 AI 代理：{ $command }。設定 [agent.presets]。
agent-no-http = 此 AI 代理無法連線到 Oku Slides 的工具。
agent-local-no-program = 本機 AI 代理只能在 Oku Slides 應用程式內執行。
agent-local-needs-download = 本機代理程式需要先下載其模型({ $gigabytes }GB)。
agent-local-intel-mac = 本機代理程式需要搭載 Apple 晶片的 Mac。
agent-local-unsupported-os = 本機代理程式目前尚不支援此系統。
agent-model-download = 下載模型
agent-model-downloading = 正在下載模型… { $percent }%
agent-model-cancel = 取消
agent-model-download-failed = 無法下載模型。
agent-model-download-integrity = 下載的模型已損壞，已被刪除。
agent-model-download-incomplete = 下載不完整：已收到 { $got }位元組，共 { $expected }位元組。
agent-model-download-cancelled = 已取消下載。
local-agent-no-route = 本機 AI 代理不了解要做什麼。請換個說法再試一次。
local-agent-tool-limit = 本機 AI 代理執行了{ $n }個步驟後停止，未能完成。
local-agent-invalid-call = 本機 AI 代理無法正確使用工具。
local-agent-engine-failed = 本機 AI 代理的模型發生錯誤。
local-agent-retry = 請重試，或在 AI 代理面板中選擇其他 AI 代理。
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
agent-error-pack-id = 這不是套件的 id：請呼叫 packs 列出套件及其 id。
agent-error-asset-unknown = 此圖片既不在簡報中，也不在已貼上的圖片中：請使用已貼上圖片的 ID。
agent-error-denied = 使用者已拒絕。
agent-error-no-turn = 目前沒有進行中的回合：{ $tool } 只會在回合中顯示。
agent-error-tab-no-turn = 只有在回覆使用者時，才可開啟或顯示套件分頁或主題分頁。
agent-error-tab-limit = 本回合已開啟或顯示過一個套件或主題分頁：請在那裡完成，或先詢問使用者再換另一個。
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
agent-error-no-file = 簡報還沒有檔案：請提供絕對路徑。
agent-error-path-relative = 路徑必須是絕對路徑。
agent-error-path-parent = 路徑不能包含「..」。
agent-error-setting-locked = 代理無法變更設定 { $key }。
agent-error-setting = 無法變更設定 { $key }：請檢查其值。
agent-error-history-index = 瀏覽歷史記錄需要版本的序號。
agent-error-no-pack = 此簡報未固定任何套件：請提供 pack（參見 packs）。
agent-error-history-not-opened = 分頁未能及時開啟歷史記錄：請重新請求。
agent-error-present = 這裡沒有可簡報的內容：沒有投影片，或分頁處於套件、歷史記錄或唯讀模式。
agent-error-locale = 未知的語言代碼。請提供內建語言代碼或 system。
agent-error-empty-interface = 沒有要變更的內容：請提供 scheme、locale 或 guides。
agent-error-empty-dictation = 沒有要變更的內容：請至少提供一項聽寫設定。
agent-error-not-shown = 此分頁不是其視窗中顯示的分頁：請在使用者顯示它後再請求。
agent-error-themes-deck-mode = 主題選擇器只在編輯投影片時開啟。
agent-error-packs-deck-mode = 簡報的套件面板只在編輯投影片時開啟。
agent-error-usage-no-deck = 此分頁的用量不屬於任何簡報：只有從簡報開啟的套件分頁或主題分頁（無論由使用者還是 AI 代理開啟），或簡報自己的套件，才計入該簡報。
agent-error-rewrite-text = 改寫傳回時沒有文字。
agent-error-paragraphs = 改寫的段落無效。
agent-error-draft-too-long = 草稿太長。
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
voice-error-changed = 提問之後設定已變更：請重新詢問
voice-error-nothing-to-download = 此代理沒有需要下載的內容
agent-tune-prompt = 微調匯入的簡報
agent-settings-unknown-selected = 設定 agent.selected = { $selected } 未指向 [agent.presets] 中的任何 AI 代理（已知：{ $known }）
agent-settings-preset-launch = AI 代理 [agent.presets.{ $id }] 需要且僅需要 command 或 builtin 其中之一
