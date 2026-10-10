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
registry-compile = 無法編譯範本套件的元件
registry-missing = 範本套件未宣告其元件
render-timeout = 算繪耗時超過 { $seconds }秒，已中斷
render-out-of-memory = 算繪所需記憶體超過 { $mib } MiB，已中斷
render-worker-lost = 算繪程序已停止
render-frame = 算繪程序傳來的訊息無法讀取
render-page-too-large = 此投影片過大，無法預覽（超過 { $mib } MiB，通常是因為圖片過大）。仍可匯出為 PDF 或 PNG。
render-stale = 算繪已被取代，請重試
render-pack-disabled = 「{ $pack }」套件的小工具在本次工作階段中已停用
registry-schema-remote = 小工具登錄表無效：{ $message }
probe-not-probed = 尚未檢查：分配給此套件小工具的時間已用完，將於下次轉譯時檢查。
probe-too-many-files = 套件中的小工具檔案超過 { $max }個：此檔案未載入。
probe-too-large = 超過 { $max } KiB：未載入。

import-unreadable = 無法讀取此簡報
import-not-a-presentation = 此檔案不是 PowerPoint 簡報
import-protected = 此簡報已加密或為舊版 .ppt 格式：請先另存為 .pptx
import-too-large = 此簡報太大，無法匯入
import-loss-list-levels = 清單層級已合併為一層
import-loss-merged-cells = 已分割合併的儲存格
import-loss-percent-stacked = 百分比堆疊圖已顯示為堆疊圖
import-loss-run-style = { $effect ->
    [underline] 底線已遺失
    [strike] 刪除線已遺失
    [baseline] 上標或下標已遺失
    [caps] 大寫已遺失
   *[highlight] 醒目提示已遺失
}
import-loss-transform = 旋轉、翻轉或裁剪已遺失
import-loss-color-dropped = 色彩 { $color } 在佈景主題中沒有相近的色彩：已使用佈景主題色彩
import-loss-text-truncated = 文字已縮短以符合限制
import-loss-table-truncated = 表格已縮短以符合限制
import-loss-opaque = { $reason ->
    [media] 影片或聲音未匯入（無替代影像）
    [ole] 內嵌物件未匯入（無替代影像）
    [math] 方程式未匯入（無替代影像）
    [smartart] SmartArt 未匯入（無替代影像）
   *[unknown] 不支援的物件未匯入（無替代影像）
}
import-loss-unsupported-image = { $format } 格式的影像未匯入
import-loss-hidden-slide = 隱藏的投影片已以可見方式匯入
import-loss-effects = 陰影、反射與立體效果已遺失
import-loss-series-dropped = 圓餅圖只顯示一個數列：保留 { $kept }個，捨棄 { $dropped }個
import-loss-repeated-decor = 已省略在 { $slides }張投影片上重複的頁首或頁尾「{ $text }」：佈景主題會繪製自己的頁首頁尾
import-loss-no-slot = 已省略內容：所選範本中沒有可放置它的位置
import-warning-effect-ignored = 效果未匯入：{ $detail }
import-warning-approximated = 近似呈現：{ $detail }
import-warning-unreadable-item = 已略過無法讀取的項目：{ $detail }
import-warning-unreadable-part = 無法讀取的部分：{ $detail }
import-warning-missing-part = 缺少部分：{ $detail }
import-warning-unsupported-chart = 不支援的圖表：{ $detail }
import-warning-unsupported-image = 不支援的影像格式：{ $detail }
import-warning-unknown-table-style = 未知的表格樣式：{ $detail }
import-warning-unsafe-link = 連結遭拒：{ $detail }
import-warning-nesting-too-deep = 巢狀層級過深，已截斷：{ $detail }
import-warning-font-embedded-unreadable = 無法讀取內嵌字型：{ $detail }
import-warning-truncated = 警告過多：其餘警告已省略
