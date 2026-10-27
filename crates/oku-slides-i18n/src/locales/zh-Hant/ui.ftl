# 應用程式介面（React）。由 ui/src/i18n.ts 透過「messages」請求載入。
ui-loading = 載入中…

## 文件

ui-brand = Oku Slides
ui-brand-pack = { $kind ->
    [layouts] { $name } · 版面配置套件
   *[other] { $name } · 小工具套件
}
ui-title-rename = 重新命名簡報
ui-title-field = 簡報標題
ui-status-unsaved = 有未儲存的變更
ui-status-saved = 已儲存
ui-status-never-saved = 尚未儲存
ui-caption-pack = { $kind ->
    [layouts] { $count ->
        [one] { $count }個版面配置
       *[other] { $count }個版面配置
    }
   *[other] { $count ->
        [one] { $count }個小工具
       *[other] { $count }個小工具
    }
} · { $status }
ui-caption-slides = { $count ->
    [one] { $count }張投影片
   *[other] { $count }張投影片
} · { $status }
ui-confirm-quit-pack = 要結束而不儲存已修改的套件嗎？
ui-confirm-quit-theme = 要結束而不儲存已修改的主題嗎？
ui-confirm-quit-read-only-theme = 無法在此儲存 { $theme }。要關閉此分頁並遺失其變更嗎？
ui-confirm-update-opener = 要將「{ $deck }」更新為套件「{ $pack }」的此版本嗎？
ui-close-writes-pending = 此分頁仍在儲存：請稍後再次關閉。
ui-confirm-close-writes-pending = 此分頁仍在儲存。仍要關閉並遺失尚未儲存的內容嗎？
ui-confirm-quit-read-only-pack = 無法在此儲存 { $pack }。要關閉此分頁並遺失其變更嗎？若要保留，請讓 AI 代理將其另存為新套件。

## 通知

ui-toast-copied = 副本已儲存
ui-toast-field-gone = 正在編輯的欄位已不存在：輸入的內容已遺失。
ui-toast-edit-replaced = 此文字在你編輯期間被更改：你的版本已取代它。還原可恢復另一個版本。

## 工具列

ui-toolbar-undo = 還原
ui-toolbar-undo-label = 還原：{ $label }
ui-toolbar-redo = 重做
ui-toolbar-redo-label = 重做：{ $label }
ui-divider-pile = 投影片堆寬度
ui-divider-notes = AI 代理對話寬度
ui-divider-code = 程式碼大小
ui-toolbar-theme = 主題
ui-toolbar-finish = 完成
ui-pack-deck-only = 僅儲存在此簡報中
ui-toolbar-open = 開啟
ui-menu = 選單
ui-toolbar-add-label = 新增至投影片
ui-toolbar-packs = 簡報的套件
ui-toolbar-packs-update = 簡報的套件（有可用的更新）
ui-toolbar-thumbs-show = 顯示縮圖
ui-toolbar-thumbs-hide = 隱藏縮圖
ui-toolbar-remove-slide = 刪除投影片
ui-toolbar-remove-hint = 投影片 { $n } · 按 ⌘Z 還原

## 還原標籤

ui-undo-add-widget = 新增「{ $name }」
ui-undo-new-slide = 新投影片：{ $name }
ui-undo-move-slide = 移動投影片
ui-undo-remove-slide = 刪除投影片
ui-undo-remove = 刪除
ui-undo-paste = 貼上
ui-undo-place-on-grid = 放到格線上
ui-undo-move-on-grid = 在格線上移動
ui-undo-resize-on-grid = 在格線上調整大小
ui-undo-change-anchor = 變更錨定
ui-undo-move = 移動
ui-undo-resize = 調整大小
ui-undo-edit-slide = 編輯投影片
ui-undo-rename-pack = 重新命名套件
ui-undo-rename-deck = 重新命名簡報

## 投影片列表

ui-pile-slides = 投影片
ui-pile-layouts = 版面配置
ui-pile-add-slide = 新投影片
ui-pile-drag-hint = 拖曳以移動 · ⌥↑ ⌥↓
ui-slide-alt = 投影片 { $n }

## 舞台

ui-stage-rendering = 渲染中…
ui-stage-widget-pack = 從堆疊中開啟一個小工具，或建立新的。
ui-stage-elements = 投影片元素
ui-stage-fields = 投影片欄位
ui-stage-edit-text = 編輯文字
ui-stage-hint-plain = Enter 或 Tab 前往下一個 · Esc 取消
ui-stage-hint-rich = Tab 前往下一個 · ⌘ Enter 確認 · 「- 」表示項目符號 · ⌘B 粗體，⌘I 斜體，⌘K 連結
ui-field = { $kind ->
    [title] 標題
    [subtitle] 副標題
    [kicker] 眉標
    [text] 文字
    [body] 正文
    [author] 作者
    [date] 日期
    [organization] 組織
   *[other] { $kind }
}
ui-element-kind = { $kind ->
    [text] 文字
    [card] 卡片
    [image] 圖片
    [shape] 形狀
    [chart] 圖表
    [table] 表格
    [stack] 堆疊
   *[other] { $kind }
}
ui-grid-cell = 欄 { $cols } · 列 { $rows }
ui-ghost-zone = 區域

## 投影片備註

ui-notes = 投影片工具
ui-stepper-less = { $label }：減少一個
ui-stepper-more = { $label }：增加一個
ui-list-item = 項目
ui-list-add = 新增{ $item }
ui-list-remove = 刪除{ $item }
ui-list-move = 移動{ $item }
ui-list-full = 清單已滿（{ $max }）
ui-undo-list-move = 移動{ $item }
ui-agent-color-other = 其他顏色
ui-agent-thread = AI 代理
ui-agent-new-conversation = 新對話
ui-agent-undo-turn = 還原本輪
ui-agent-detach = 在獨立視窗中開啟代理
ui-agent-attach = 將代理放回主視窗
ui-agent-window-empty = 此分頁沒有代理
ui-agent-kept-in-draft = 沒有可寫入的位置：文字已保留在草稿中
ui-draft = 草稿
ui-draft-send = 傳送給 AI 代理
ui-draft-send-title = 傳送給 AI 代理 (⌘↩)
ui-draft-discard = 捨棄
ui-pasted-image = { $type } { $width }×{ $height }
ui-draft-images = 已貼上的圖片
ui-draft-remove-image = 移除此圖片
ui-draft-send-needs-text = 請說明要如何處理它
ui-agent-state-idle = AI 代理
ui-agent-state-starting = AI 代理正在啟動…
ui-agent-state-working = AI 代理正在工作…
ui-agent-dock = 進行中
ui-agent-dock-settings = 設定
ui-agent-jump = ↓ 本輪
ui-agent-tool-input = 參數
ui-agent-tool-output = 結果
ui-agent-sheet = AI 代理的問題
ui-agent-sheet-keys = 1–4 選擇 · Enter 確認 · E 修改 · Esc 取消
ui-agent-variants = 變體
ui-agent-drawer-keys = 點擊或 1–4 · Esc 關閉
ui-agent-confirm = AI 代理的確認

## 聽寫

ui-dictation = 聽寫
ui-dictation-start = 聽寫
ui-dictation-start-title = 聽寫（⌘D）
ui-dictation-type = 寫訊息給 AI 代理
ui-dictation-type-title = 寫訊息給 AI 代理（⌘K）
ui-dictation-agent-stop = 停止 AI 代理（⌘⇧D 或 Esc）
ui-dictation-agent-working = AI 代理正在工作…
ui-dictation-agent-working-hint = ⌘⇧D 或 Esc 可停止
ui-dictation-finish = 結束聽寫
ui-dictation-finish-title = 結束（⌘D）
ui-dictation-cancel = 取消
ui-dictation-preparing = 正在準備聽寫…
ui-dictation-heard-nothing = 沒有聽到聲音：請靠近麥克風一點再說
ui-dictation-to-cursor = 插入至游標處
ui-dictation-to-draft = 到草稿
ui-dictation-downloading = 正在下載 { $name } 模型
ui-dictation-download-progress = { $percent }% · { $received } / { $total } · 只需一次
ui-dictation-listening = 聆聽中 · { $time }
ui-dictation-transcribing = 正在轉錄最後一句…
ui-dictation-stop-in = { $duration } 後停止
ui-dictation-speak = 請開始說話，文字會顯示在這裡…
ui-dictation-auto-stop = 靜音 { $duration } 後停止
ui-dictation-to-download = 需下載 { $size }
ui-dictation-settings = 設定
ui-dictation-language-auto = 自動偵測
ui-dictation-model-unavailable = 不支援該語言
ui-dictation-language-auto-resolved = 自動（{ $language }）
ui-dictation-to-download-language = 需下載語言
ui-dictation-api-key-missing = 未設定 { $var }
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } 秒
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } GB
ui-size-mb = { $value } MB

## 範本選擇器

ui-picker-new-slide = 新投影片
ui-picker-set-mine = { $name } · 我的
ui-picker-set-received = { $name } · 收到的
ui-picker-keys = ← → 瀏覽 · Enter 新增 · Esc 關閉
ui-picker-close = 關閉
ui-picker-list = 版面配置
ui-widget-picker-filter = 依名稱、標籤或說明篩選
ui-widget-picker-empty = 沒有符合「{ $query }」的小工具。
ui-widget-picker-list = 小工具
ui-widget-picker-keys = 方向鍵瀏覽 · Enter 新增 · Esc 關閉

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
ui-chart-add-row = 列
ui-chart-add-column = 欄
ui-chart-series = 數列 { $n }

## 表格

ui-table-panel = 表格
ui-table-options = 表格選項
ui-table-header-row = 標題列
ui-table-header-column = 首欄作標題
ui-table-striped = 斑馬紋列
ui-table-alignment = 欄對齊
ui-table-align-of = { $name }的對齊方式
ui-table-align = { $align ->
    [start] 起始
    [center] 置中
    [end] 末尾
   *[auto] 自動
}
ui-table-column = 欄 { $n }
ui-table-no-data = 此表格沒有資料。
ui-show-as = 顯示為
ui-show-as-table = 顯示為表格
ui-show-as-chart = 顯示為圖表

## 檔案

ui-file-open-prompt = 要開啟的檔案路徑（.oku-slides.pdf、.json 或 .pptx）
ui-file-filter-presentations = 簡報
ui-file-filter-pptx = PowerPoint
ui-file-save-prompt = 另存為（完整路徑）

## 分頁

ui-tabs = 已開啟的簡報
ui-tab-new = 新增分頁
ui-tab-close = 關閉分頁
ui-tab-unsaved = 未儲存的變更
ui-tab-pack = 套件
ui-tab-theme = 主題
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
ui-help = 說明
ui-help-caption = 第{ $index }張，共 { $count }張 · 唯讀
ui-slide-previous = 上一張投影片
ui-slide-next = 下一張投影片
ui-present = 簡報
ui-present-slide = 第{ $index }張，共 { $count }張
ui-present-screen = 全螢幕簡報
ui-help-close = 關閉說明
ui-help-search = 搜尋說明
ui-help-search-placeholder = 搜尋…
ui-help-search-empty = 沒有結果
ui-help-search-count = { $count }個結果
ui-widgets-files = 小工具檔案
ui-widgets-common = 共用
ui-widgets-new = 新增小工具
ui-widgets-name = 小工具名稱（小寫字母、數字和連字號）
ui-widgets-name-taken = 此名稱已被使用
ui-widgets-name-letter = 名稱必須以字母開頭
ui-widgets-valid = 有效
ui-widgets-errors = 錯誤
ui-widgets-empty = 此套件還沒有小工具。
ui-widgets-preview = 小工具預覽
ui-widgets-no-preview = 無預覽
ui-widgets-field-bound = 已連結到簡報：此處唯讀
ui-widgets-source = 原始碼
ui-widgets-unsaved = 未儲存
ui-widgets-in-error = 有錯誤的小工具
ui-widgets-none-in-error = 沒有小工具有錯誤
ui-widgets-section = 小工具
ui-widgets-open = 開啟小工具 { $name }
ui-widgets-delete-named = 刪除 { $name }
ui-widgets-confirm-delete = 刪除小工具「{ $name }」?
ui-widgets-rename-hint = 按兩下或按 F2 重新命名
ui-pack-tabs = 套件分頁
ui-pack-layouts-tab = 版面配置
ui-theme-name = 主題名稱
ui-theme-unsaved = 主題變更未儲存
ui-widgets-close-tab = 關閉 { $name }
ui-widgets-removed-by-agent = AI 代理刪除了小工具 { $name }。

## 匯入

ui-import-title = 匯入 PowerPoint 簡報
ui-import-mode = 匯入方式
ui-import-hifi = 忠實副本
ui-import-hifi-note = 保留原稿的外觀，使用為此簡報產生的主題和版面配置。
ui-import-retrofit = 使用我的版面配置
ui-import-retrofit-note = 以資料庫中的版面配置套件和主題重建投影片。
ui-import-layout-packs = 版面配置套件
ui-import-theme-default = 預設主題
ui-import-no-packs = 資料庫中沒有版面配置套件。
ui-import-tune = 由 AI 代理微調
ui-import-tune-note = AI 代理會將每張投影片與原稿比較並修正差異。
ui-import-tune-note-retrofit = AI 代理會將每張投影片與原稿比較，並為版面配置無法呈現的形狀設計小工具。
ui-import-tune-unavailable = 沒有可用的 AI 代理：{ $reason }
ui-import-submit = 匯入
ui-import-busy = 正在匯入…
ui-update-available = Oku Slides { $version } 現已推出。
ui-update-install = 安裝並重新啟動
ui-update-see-release = 查看發行版
ui-update-later = 稍後
ui-update-installing = 正在下載更新… { $percent }%
ui-update-downloading = 正在下載更新…
ui-update-up-to-date = Oku Slides 已是最新版本。
ui-update-check = 檢查更新…

ui-shortcut-theme = 主題, theme, themes
