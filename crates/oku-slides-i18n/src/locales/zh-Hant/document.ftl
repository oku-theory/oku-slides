pdf-unreadable = 無法讀取 PDF
pdf-unsupported-version = 此 PDF 由較新版本的 Oku Slides 寫入
pdf-corrupted = 此 PDF 中的原始檔已損毀
pdf-write = 無法寫入 PDF

render-compile = 無法渲染簡報
render-pdf = PDF 匯出失敗
render-no-page = 此頁面不存在
render-image = 無法生成幻燈片的圖像

render-setup = 渲染器無法啟動
render-missing-image = 缺少圖片
diagnostic-error = 錯誤
diagnostic-warning = 警告
diagnostic = { $level }（{ $place }）：{ $message }
diagnostic-unplaced = { $level }：{ $message }
diagnostic-hint = 提示：{ $hint }
render-chart-failed = 無法繪製圖表

registry-bad-pack-id = 範本套件識別碼無效: { $id }
registry-base-pack-broken = 內建小工具套件已損毀 ({ $file })；請重新安裝 App。
registry-duplicate-pack = 簡報的兩個小工具套件共用 { $pack }。
registry-compile = 無法編譯範本套件的元件
registry-missing = 範本套件未宣告其元件
render-timeout = 算繪耗時超過 { $seconds }秒，已中斷
render-out-of-memory = 算繪所需記憶體超過 { $mib } MiB，已中斷
render-worker-lost = 算繪程序已停止
render-frame = 算繪程序傳來的訊息無法讀取
render-page-too-large = 此投影片過大，無法預覽（超過 { $mib } MiB，通常是因為圖片過大）。仍可匯出為 PDF 或 PNG。
render-stale = 算繪已被取代，請重試
render-pack-disabled = 「{ $pack }」套件的小工具在本次工作階段中已停用
render-protocol = 算繪程序屬於另一個版本的 Oku Slides，請重新啟動應用程式
render-remote = 算繪程序無法完成該請求
registry-schema-remote = 小工具登錄表無效：{ $message }
probe-not-probed = 尚未檢查：分配給此套件小工具的時間已用完，將於下次轉譯時檢查。
probe-too-many-files = 套件中的小工具檔案超過 { $max }個：此檔案未載入。
probe-too-large = 超過 { $max } KiB：未載入。

import-unreadable = 無法讀取此簡報
import-not-a-presentation = 此檔案不是 PowerPoint 簡報
import-protected = 此簡報已加密或為舊版 .ppt 格式：請先另存為 .pptx
import-too-large = 此簡報太大，無法匯入
import-output-too-large = 匯入後的簡報將超過匯入允許的大小
import-loss-list-levels = 清單層級已合併為一層
import-loss-merged-cells = 已分割合併的儲存格
import-loss-percent-stacked = 百分比堆疊圖已顯示為堆疊圖
import-loss-chart-format = 未保留圖表的數字格式 { $format }
import-loss-run-style = { $effect ->
    [underline] 底線已遺失
    [strike] 刪除線已遺失
    [baseline] 上標或下標已遺失
    [caps] 大寫已遺失
    [font] 部分文字的字型已改用文字方塊的字型
    [tracking] 部分文字的字距已改用文字方塊的字距
    [weight] 一般文字段依文字方塊顯示為粗體
    [slant] 正體文字段依文字方塊顯示為斜體
   *[highlight] 醒目提示已遺失
}
import-loss-transform = 旋轉、翻轉或裁剪已遺失
import-loss-color-dropped = 色彩 { $color } 在佈景主題中沒有相近的色彩：已使用佈景主題色彩
import-loss-text-truncated = 文字已縮短以符合限制
import-loss-table-truncated = 表格已縮短以符合限制
import-loss-chart-truncated = 圖表資料已縮短以符合限制
import-loss-opaque = { $reason ->
    [media] 影片或聲音未匯入（無替代影像）
    [ole] 內嵌物件未匯入（無替代影像）
    [math] 方程式未匯入（無替代影像）
    [smartart] SmartArt 未匯入（無替代影像）
   *[unknown] 不支援的物件未匯入（無替代影像）
}
import-loss-smartart-layout = 沒有繪圖的 SmartArt：其文字以卡片排列，圖表版面配置已遺失
import-loss-unsupported-image = { $format } 格式的影像未匯入
import-loss-hidden-slide = 隱藏的投影片已以可見方式匯入
import-loss-effects = 陰影、反射與立體效果已遺失
import-loss-series-dropped = 圓餅圖只顯示一個數列：保留 { $kept }個，捨棄 { $dropped }個
import-loss-repeated-decor = 已省略在 { $slides }張投影片上重複的頁首或頁尾「{ $text }」：佈景主題會繪製自己的頁首頁尾
import-loss-no-slot = 已省略內容：所選範本中沒有可放置它的位置
import-loss-moved-to-notes = 文字已移至演講者備忘稿：封面上沒有可放置它的位置
import-loss-items-dropped = 投影片上的元素過多：省略了{ $dropped }個元素
import-loss-page-fitted = 來源頁面（{ $width } × { $height } pt）不是 16:9：內容已縮放並置中
import-loss-paragraph-style = 段落的對齊方式、行距、段間距或縮排已改為文字方塊的設定
import-loss-shape-style = { $effect ->
    [gradient] 漸層已顯示為單色
    [picture] 填滿圖片已延展至圖案的外框
    [dash] 虛線已顯示為實線
    [arrow] 未繪製線條箭頭
   *[geometry] 未知圖案已顯示為矩形
}
import-loss-layout-dropped = 一個套件中的版面配置過多：「{ $layout }」沒有範本，其投影片使用空白元件，並以元素形式取得其裝飾
import-loss-decor-truncated = 為符合套件檔案大小而截斷了裝飾：省略了{ $omitted }個元素
import-loss-table-style = 表格樣式為近似：其自身的儲存格背景、框線、欄寬、列高或文字大小已遺失
import-loss-overflow = 元素「{ $element }」超出了投影片邊緣
import-loss-empty-data = 沒有資料的表格或圖表：未放置任何內容
import-reference-unavailable = 無視覺評分：{ $cause ->
    [timeout] 參考轉譯耗時過長
    [memory] 參考轉譯佔用記憶體過多
    [pages] 參考轉譯有{ $reference }頁，而簡報有{ $deck }頁
    [missing] 找不到 LibreOffice (soffice)；請在 import.soffice 中設定其路徑
   *[failed] 參考轉譯器無法讀取該簡報
}
import-region-suspect = 已擱置的差異，參考轉譯器的已知錯誤：{ $limit ->
    [autofit] PowerPoint 未儲存縮放比例時，它會縮小自動調整至文字方塊的文字
    [tracking] 它把加寬字元間距的文字畫得更寬
    [chart] 它只能大致繪製圖表
    [default_font] 它用自己的後備字型繪製僅由簡報預設值指定字型的文字
    [no_substitute] 它沒有此字型的替代字型，改用其他字型繪製
    [smart_art_style] 它用自己的樣式繪製 SmartArt 文字
   *[smart_art] 它把 SmartArt 文字畫得太小
}
import-font-substituted = { $family } 已替換為度量相容的 { $by }
import-font-missing = 找不到 { $family }：文字使用 { $by }
import-font-restricted = { $family }：其授權禁止將部分字樣內嵌到簡報；這些樣式的文字使用其其他字樣
import-font-restricted-substituted = { $family } 不能內嵌到簡報（授權）：已替換為度量相容的 { $by }
import-font-restricted-missing = { $family } 不能內嵌到簡報（授權）：文字使用 { $by }
import-font-too-large = { $family }：檔案太大，無法嵌入簡報；文字使用 { $by }
import-font-missing-no-fallback = 找不到 { $family }，也找不到主題的內文字型：文字以引擎的預設字型顯示
import-font-too-large-no-fallback = { $family }：檔案太大，無法嵌入簡報，主題的內文字型也無法使用：文字以引擎的預設字型顯示
import-font-restricted-no-fallback = { $family } 不能內嵌到簡報（授權），主題的內文字型也無法使用：文字以引擎的預設字型顯示
import-warning-effect-ignored = 效果未匯入：{ $detail }
import-warning-approximated = 近似呈現：{ $detail }
import-warning-unreadable-item = 已略過無法讀取的項目：{ $detail }
import-warning-unreadable-part = 無法讀取的部分：{ $detail }
import-warning-missing-part = 缺少部分：{ $detail }
import-warning-unsupported-chart = 不支援的圖表：{ $detail }
import-warning-unsupported-image = 不支援的影像格式：{ $detail }
import-warning-oversized-image = 影像過大，無法匯入（像素數）：{ $detail }
import-warning-external-image = 檔案外連結的影像，未匯入：{ $detail }
import-warning-unsafe-svg = SVG 影像已清除不安全內容，或已捨棄：{ $detail }
import-warning-unknown-table-style = 未知的表格樣式：{ $detail }
import-warning-unsafe-link = 連結遭拒：{ $detail }
import-warning-nesting-too-deep = 巢狀層級過深，已截斷：{ $detail }
import-warning-font-embedded-unreadable = 無法讀取內嵌字型：{ $detail }
import-warning-truncated = 警告過多：其餘警告已省略
