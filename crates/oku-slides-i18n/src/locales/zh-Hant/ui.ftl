# 應用程式介面（React）。由 ui/src/i18n.ts 透過「messages」請求載入。
ui-loading = 載入中…

## 文件

ui-brand = Oku Slides
ui-brand-templates = Oku Slides · 範本
ui-title-rename = 重新命名簡報
ui-title-field = 簡報標題
ui-status-unsaved = 有未儲存的變更
ui-status-saved = 已儲存
ui-status-never-saved = 尚未儲存
ui-status-builtin = 內建：儲存時會建立副本
ui-caption-templates = { $count ->
    [one] { $count } 個範本
   *[other] { $count } 個範本
} · { $status }
ui-caption-slides = { $count ->
    [one] { $count } 張投影片
   *[other] { $count } 張投影片
} · { $status }
ui-template-default-name = 範本
ui-template-variant-name = { $name }（變體）
ui-confirm-quit-templates = 要結束而不儲存已修改的範本嗎？

## 通知

ui-toast-templates-saved = 範本已儲存：{ $name }
ui-toast-moved = 簡報已移動
ui-toast-copied = 副本已儲存
ui-toast-select-zone = 請選取要放置的區域：「{ $text }」

## 工具列

ui-toolbar-undo = 還原
ui-toolbar-undo-label = 還原：{ $label }
ui-toolbar-redo = 重做
ui-toolbar-redo-label = 重做：{ $label }
ui-toolbar-grid-hide = 隱藏格線
ui-toolbar-grid-show = 顯示格線
ui-toolbar-theme = 主題
ui-toolbar-theme-default = 預設
ui-toolbar-save-copy = 儲存副本
ui-toolbar-finish = 完成
ui-toolbar-templates = 範本
ui-toolbar-open = 開啟
ui-merge = 與 PDF 合併…
ui-menu = 選單
ui-menu-move = 移動…
ui-language = 語言
ui-language-system = 系統語言
ui-toolbar-add-label = 新增至投影片
ui-add-kind = { $kind ->
    [card] 卡片
    [shape] 形狀
    [chart] 圖表
   *[text] 文字
}
ui-toolbar-remove-template = 刪除範本
ui-toolbar-remove-slide = 刪除投影片
ui-toolbar-remove-hint = 投影片 { $n } · 按 ⌘Z 還原

## 還原標籤

ui-undo-add = { $kind ->
    [card] 新增卡片
    [shape] 新增形狀
    [chart] 新增圖表
   *[text] 新增文字
}
ui-undo-new-template = 新範本：{ $name }
ui-undo-new-slide = 新投影片：{ $name }
ui-undo-grid = 格線 { $columns } × { $rows }
ui-undo-templates = 範本：{ $name }
ui-undo-move-template = 移動範本
ui-undo-move-slide = 移動投影片
ui-undo-remove-slide = 刪除投影片
ui-undo-remove = 刪除
ui-undo-cut = 剪下
ui-undo-paste = 貼上
ui-undo-place-on-grid = 放到格線上
ui-undo-move-on-grid = 在格線上移動
ui-undo-resize-on-grid = 在格線上調整大小
ui-undo-change-anchor = 變更錨定
ui-undo-move = 移動
ui-undo-resize = 調整大小
ui-undo-edit-text = 編輯文字
ui-undo-dictation = 聽寫
ui-undo-rename-set = 重新命名範本集
ui-undo-rename-deck = 重新命名簡報
ui-undo-theme = 變更主題
ui-undo-rename-template = 重新命名範本
ui-undo-describe-template = 描述範本
ui-undo-chart-kind = 變更圖表類型
ui-undo-chart-data = 編輯資料
ui-undo-chart-color = 為系列設定顏色
ui-undo-chart-stack = { $stacked ->
    [yes] 堆疊系列
   *[no] 取消堆疊系列
}

## 新元素

ui-new-text = 新文字
ui-new-card-title = 卡片標題
ui-new-card-detail = 詳細資訊

## 投影片列表

ui-pile-slides = 投影片
ui-pile-templates = 範本
ui-pile-add-slide = + 新投影片
ui-pile-add-template = + 新範本
ui-pile-drag-hint = 拖曳以移動 · ⌥↑ ⌥↓
ui-slide-alt = 投影片 { $n }

## 舞台

ui-stage-rendering = 渲染中…
ui-stage-elements = 投影片元素
ui-stage-edit-text = 編輯文字
ui-stage-hint-field = Enter 確認 · Esc 取消
ui-stage-hint-block = ⌘ Enter 確認 · 每段一行，「- 」表示項目符號 · ⌘B 粗體，⌘I 斜體，⌘K 連結
ui-field = { $kind ->
    [title] 標題
    [subtitle] 副標題
    [kicker] 眉標
    [text] 文字
   *[other] { $kind }
}
ui-element-kind = { $kind ->
    [text] 文字
    [card] 卡片
    [image] 圖片
    [shape] 形狀
    [group] 群組
   *[other] { $kind }
}
ui-grid-cell = 欄 { $cols } · 列 { $rows }

## 投影片備註

ui-notes = 投影片工具
ui-stepper-less = { $label }：減少一個
ui-stepper-more = { $label }：增加一個
ui-agent-color-other = 其他顏色
ui-agent-thread = 智慧代理
ui-agent-new-conversation = 新對話
ui-agent-tools = { $count } 個工具
ui-draft = 草稿
ui-draft-insert = 插入
ui-draft-insert-title = 原樣插入 (⌘⇧↩)
ui-draft-send = 傳送給智慧代理
ui-draft-send-title = 傳送給智慧代理 (⌘↩)
ui-draft-discard = 捨棄
ui-agent-state-idle = 智慧代理
ui-agent-state-starting = 智慧代理正在啟動…
ui-agent-state-working = 智慧代理正在工作…
ui-agent-stopped = 已停止。
ui-agent-ended = 本輪結束（{ $stop }）。
ui-agent-stop-max-tokens = 智慧代理已達到長度上限。
ui-agent-stop-max-turns = 智慧代理已達到步數上限。
ui-agent-stop-refusal = 智慧代理拒絕了。
ui-agent-sheet = 智慧代理的問題
ui-agent-sheet-keys = 1–4 選擇 · Enter 確認 · E 修改 · Esc 取消
ui-agent-variants = 變體
ui-agent-drawer-keys = 點擊或 1–4 · Esc 關閉
ui-agent-confirm = 智慧代理的確認

## 聽寫

ui-dictation = 聽寫
ui-dictation-start = 聽寫
ui-dictation-start-title = 聽寫（⌘D）
ui-dictation-agent-stop = 停止智慧代理（⌘⇧D 或 Esc）
ui-dictation-agent-working = 智慧代理正在工作… ⌘⇧D 或 Esc 可停止
ui-dictation-finish = 結束聽寫
ui-dictation-finish-title = 結束（⌘D）
ui-dictation-cancel = 取消
ui-dictation-preparing = 正在準備聽寫…
ui-dictation-heard-nothing = 沒有聽到聲音：請靠近麥克風一點再說
ui-dictation-to-cursor = 插入至游標處
ui-dictation-to-draft = 到草稿
ui-dictation-to-field = 取代{ $field }
ui-dictation-to-block = 插入至所選區塊之後
ui-dictation-new-block = 新文字區塊
ui-dictation-select-zone = 選取一個區域
ui-dictation-downloading = 正在下載 { $name } 模型
ui-dictation-download-progress = { $percent }% · { $received } / { $total } · 只需一次
ui-dictation-listening = 聆聽中 · { $time }
ui-dictation-transcribing = 正在轉錄最後一句…
ui-dictation-stop-in = { $duration } 後停止
ui-dictation-speak = 請開始說話，文字會顯示在這裡…
ui-dictation-auto-stop = 靜音 { $duration } 後停止
ui-dictation-to-download = 需下載 { $size }
ui-dictation-settings = 聽寫設定
ui-dictation-model-title = 辨識模型
ui-dictation-model-intro = Whisper，在這台電腦上執行（CTranslate2）。每個模型都支援多種語言。
ui-dictation-note-turbo = 推薦，快速又準確
ui-dictation-note-large = 最準確，但較慢
ui-dictation-note-base = 輕量，適合規格較低的電腦
ui-dictation-installed = 已安裝
ui-dictation-not-installed = 待下載
ui-dictation-language = 口說語言
ui-dictation-language-auto = 自動偵測
ui-dictation-auto-stop-label = 靜音多久後自動停止
ui-dictation-silence = 靜音時間長度
ui-dictation-phrases-note = 只會轉錄有語音的片段，在你說話時逐句進行。
ui-dictation-download-now = 立即下載（{ $size }）
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } 秒
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } GB
ui-size-mb = { $value } MB

## 範本選擇器

ui-picker-new-slide = 新投影片
ui-picker-new-template = 新範本，根據…
ui-picker-set = 範本
ui-picker-set-mine = { $name } · 我的
ui-picker-keys = ← → 瀏覽 · Enter 新增 · Esc 關閉
ui-picker-close = 關閉
ui-picker-previous = 上一個範本
ui-picker-next = 下一個範本
ui-picker-list = 投影片範本

## 圖表

ui-chart-panel = 圖表
ui-chart-kinds = 圖表類型
ui-chart-kind = { $kind ->
    [line] 折線圖
    [pie] 圓餅圖
    [scatter] 散佈圖
    [radar] 雷達圖
    [waterfall] 瀑布圖
   *[bar] 長條圖
}
ui-chart-no-data = 此圖表沒有資料表。
ui-chart-options = 圖表選項
ui-chart-stack = 堆疊
ui-chart-waterfall-hint = 勾選「合計」即為合計長條。
ui-chart-colors = 系列顏色
ui-chart-color-of = { $name } 的顏色
ui-chart-color-auto = 自動
ui-chart-color-pick = 顏色 { $n }
ui-chart-column-name = 第 { $col } 欄的名稱
ui-chart-remove-column = 刪除欄
ui-chart-cell = 第 { $row } 列，第 { $col } 欄
ui-chart-remove-row = 刪除列
ui-chart-add-row = + 列
ui-chart-add-column = + 欄
ui-chart-series = 數列 { $n }

## 檔案

ui-file-open-prompt = 要開啟的檔案路徑（.oku-slides.pdf 或 .json）
ui-file-save-prompt = 另存為（完整路徑）

## 分頁

ui-tabs = 已開啟的簡報
ui-tab-new = 新增分頁
ui-tab-close = 關閉分頁
ui-tab-unsaved = 未儲存的變更
ui-tab-templates = 範本
ui-tab-move-new-window = 移至新視窗
ui-tab-move-to = 移至視窗 { $n }
ui-tab-close-others = 關閉其他分頁
ui-close-cancel = 取消
ui-disk-replaced-question = 磁碟上的 { $file } 已被另一份簡報取代。
ui-disk-replaced-overwrite = 覆寫
ui-disk-replaced-save-as = 另存新檔…
ui-same-deck-question = { $file } 是分頁「{ $title }」中已開啟簡報的一個版本。
ui-same-deck-merge = 合併到該分頁
ui-same-deck-apart = 單獨開啟
ui-history = 歷程記錄
ui-history-version = { $date } 的版本
ui-history-restore = 還原
ui-history-back = 返回
ui-history-anonymous = 匿名
ui-history-slider = 簡報的版本
ui-history-removed-since = 此後已刪除
ui-history-changed-since = 此後已修改
ui-history-added-since = 此後新增的投影片：{ $count }
