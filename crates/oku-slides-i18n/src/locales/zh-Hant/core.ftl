op-slide-not-found = 找不到投影片：{ $subject }
op-element-not-found = 找不到元素：{ $subject }
op-duplicate-id = 識別碼已被使用：{ $subject }
op-index-out-of-range = 位置超出範圍
op-invalid-value = { $subject } 的值無效
op-invalid-value-because = { $subject } 的值無效：{ $reason }
op-value-no-value = 此參數沒有值
op-value-no-dataset = 圖表沒有資料集
op-value-grid-tracks = 格線的欄數和列數為 1 到 24
op-value-bad-item-id = 項目 ID 是不含「/」的非空白文字
op-value-missing-item-id = 每個項目都需要一個 ID
op-value-too-deep = 元素巢狀過深
op-value-empty-path = 路徑為空
op-value-not-text = 需要文字或 null
op-value-empty-frame = 寬度和高度必須大於 0
op-value-blocks-off-grid = 流式插槽由投影片排列，格線會將其隱藏；請先將每個元素移到空閒插槽
op-value-outside-grid = 儲存格超出格線
op-value-not-tokens = 需要物件或 null
op-value-not-a-slot = 此參數不是插槽
op-value-out-of-bounds = 數值超出允許範圍
op-value-unreadable = 值的格式不符合預期
op-value-valign = 應為 top、middle 或 bottom
op-value-path = 路徑由 M、L、C、Q、Z 指令及其數字組成，以 M 開頭
op-unknown-widget = 未知的元件：{ $subject }
op-unknown-namespace = 此簡報沒有元件套件 { $pack }。
op-pack-required = 元件套件 { $pack } 是 { $by } 所需的；請先移除後者。
op-pack-in-use = 元件套件 { $pack } 仍被 { $count }個項目使用。
op-pack-requirement-missing = 元件套件 { $pack } 需要 { $required }，但此簡報中沒有。
op-unknown-param = 未知的參數：{ $subject }
op-param-type-because = { $subject } 的值無效：{ $reason }
op-param-shape-text = { $nullable ->
    [yes] 需要文字或 null
   *[no] 需要文字
}
op-param-shape-choice = { $nullable ->
    [yes] 需要其值之一或 null
   *[no] 需要其值之一
}
op-param-shape-paragraphs = { $nullable ->
    [yes] 需要段落清單或 null
   *[no] 需要段落清單
}
op-param-shape-elements = { $nullable ->
    [yes] 需要元素清單或 null
   *[no] 需要元素清單
}
op-param-shape-flag = { $nullable ->
    [yes] 需要布林值或 null
   *[no] 需要布林值
}
op-param-shape-number = { $nullable ->
    [yes] 需要數字或 null
   *[no] 需要數字
}
op-param-shape-bounded-number = { $nullable ->
    [yes] 需要範圍內的數字或 null
   *[no] 需要範圍內的數字
}
op-param-shape-grid = { $nullable ->
    [yes] 需要格線或 null
   *[no] 需要格線
}
op-param-shape-chart-option = { $nullable ->
    [yes] 需要圖表選項或 null
   *[no] 需要圖表選項
}
op-param-shape-table-option = { $nullable ->
    [yes] 需要表格選項或 null
   *[no] 需要表格選項
}
op-param-shape-chart-or-table-option = { $nullable ->
    [yes] 需要圖表或表格選項或 null
   *[no] 需要圖表或表格選項
}
op-param-shape-items = { $nullable ->
    [yes] 需要項目清單或 null
   *[no] 需要項目清單
}
op-param-shape-list = { $nullable ->
    [yes] 需要清單或 null
   *[no] 需要清單
}
op-param-shape-add-element-only = { $nullable ->
    [yes] 元素透過 add_element 新增，或 null
   *[no] 元素透過 add_element 新增
}
op-param-shape-set-param-only = { $nullable ->
    [yes] 值透過 set_param 設定，或 null
   *[no] 值透過 set_param 設定
}
op-param-shape-path-option = { $nullable ->
    [yes] 應為形狀路徑（帶鍵 d 的物件）或 null
   *[no] 應為形狀路徑（帶鍵 d 的物件）
}
op-item-not-found = 沒有項目 { $subject }
op-list-full = 清單 { $subject } 已滿（最多 { $max }個項目）
op-list-minimum = 清單 { $subject } 至少需要 { $min }個項目
op-list-cell-too-long = 清單 { $subject } 的儲存格過長（欄 { $column }）
op-list-id-too-long = 清單 { $subject } 的項目 ID 過長
op-list-too-many-items = 清單 { $subject } 的項目過多（最多 { $max }個）
op-slot-rejects = 此處不接受 { $subject }
op-wrong-category = 此處不能使用 { $subject }
op-choice-out-of-range = { $subject } 不允許此值
transaction-refused = 操作 { $index } 遭拒：{ $reason }

sources-unreadable = 無法讀取原始檔
sources-not-sources = 這不是 Oku Slides 原始檔封存
sources-unsupported-version = 這些原始檔由較新版本的 Oku Slides 寫入
sources-bad-name = 原始檔中的檔名遭拒
sources-missing-deck = 原始檔中缺少簡報
sources-too-large = 此檔案中的原始檔過大，無法開啟
sources-too-deep = 此簡報的元素巢狀過深

issue-empty-slide-id = 投影片識別碼為空
issue-duplicate-slide-id = 投影片識別碼重複
issue-duplicate-element-id = 元素識別碼重複
issue-empty-element-id = 識別碼為空
issue-cell-outside-grid = 儲存格超出格線
issue-empty-frame = 框架尺寸為零
issue-free-without-frame = 自由元素缺少框架
issue-unknown-widget = 未知元件：{ $kind }
issue-wrong-category = { $kind } 不能在此處使用
issue-unknown-param = 未知參數：{ $param }
issue-param-type = { $param } 的值無效
issue-choice-out-of-range = { $param } 不允許此值
issue-slot-rejects = 此處不接受 { $kind }
issue-placed-in-flow = 在排版清單中以框架放置的元素
issue-unknown-bind = { $param } 連結到未知的簡報設定
issue-slot-too-deep = 元素巢狀過深
issue-invalid-style = 無效的樣式值：{ $field }（{ $reason ->
        [valign] { op-value-valign }
        [out-of-bounds] { op-value-out-of-bounds }
       *[other] { op-value-unreadable }
    }）
issue-missing-asset = 缺少資源：{ $asset }
issue-pack-version-mismatch = 已捨棄遭竄改的範本套件版本：{ $version }
issue-pack-version-missing = 原始檔中缺少範本套件版本：{ $version }
issue-pack-pin-mismatch = 內嵌的元件套件 { $pack } 與簡報所期望的不符。
issue-pack-requirement-missing = 元件套件 { $pack } 需要 { $required }，但此簡報中沒有。
issue-theme-version-mismatch = 已捨棄遭竄改的主題版本：{ $version }
issue-theme-version-missing = 原始檔中缺少主題版本：{ $version }
issue-unsupported-link = 不支援的連結：{ $link }
issue-highlight-zero = 醒目提示顏色從 1 開始編號
issue-unreadable-element = 無法讀取的元素，已移出
issue-slot-repaired = 元素沒有已知的插槽，已放入 { $slot }
issue-unreadable-item = 無法讀取的清單項目，已移出
issue-unreadable-row = 無法讀取的表格列，已移出
issue-data-truncated = 資料過大，讀取時已截斷
issue-unreadable-slide = 無法讀取的投影片，已移出
issue-unreadable-field = 無法讀取的簡報欄位 { $field }，已重設
op-invalid-chart = 圖表 { $subject }：{ $reason }
op-invalid-table = 表格 { $subject }：{ $reason }

replica-corrupted = 此檔案中的簡報歷程已損毀
replica-too-large = 此檔案中的簡報歷程過大，無法開啟
replica-write = 無法更新簡報歷程
replica-unrelated = 這兩份簡報沒有共同的來源
replica-unsupported-version = 此簡報使用的格式無法由此版本的 Oku Slides 讀取
replica-too-deep = 無法儲存此簡報：已刪除的元素在其歷史記錄中巢狀過深。請復原最近的移動操作，然後重新儲存。

widget-unreadable = 無法讀取小工具宣告
widget-fault-empty-name = 小工具 { $widget }：某個小工具或參數沒有名稱
widget-fault-duplicate-widget = 小工具 { $widget } 被宣告了兩次
widget-fault-duplicate-param = 小工具 { $widget } 兩次宣告了同一個參數
widget-fault-empty-choice = 小工具 { $widget }：某個選項沒有任何值
widget-fault-default-not-in-choice = 小工具 { $widget }：選項的預設值不在其取值範圍內
widget-fault-default-wrong-type = 小工具 { $widget }：預設值與其參數類型不符
widget-fault-bind-not-plain = 小工具 { $widget }：只有純文字參數才能綁定到簡報
widget-fault-unknown-slot-kind = 小工具 { $widget }：插槽接受了未知的元素小工具
widget-fault-empty-list = 小工具 { $widget }：清單至少需要一欄
widget-fault-column-type = 小工具 { $widget }：清單的欄可以是 plain、rich、number、flag、choice 或 asset（或巢狀一層的 p.list）；若每項需要自由內容，請使用帶子小工具的 p.slot
widget-fault-reserved-name = 小工具 { $widget }：保留名稱：id、kind，或包含 / 的名稱
widget-fault-list-bounds = 小工具 { $widget }：清單的最大值小於其最小值
widget-fault-list-default-too-large = 小工具 { $widget }：清單的預設值超出清單的大小限制
widget-fault-list-depth = 小工具 { $widget }：巢狀清單不能再包含清單欄；清單只能巢狀一層
widget-fault-list-default-below-min = 小工具 { $widget }：巢狀清單預設值的項目數少於其最小值
issue-empty-item-id = 清單 { $param } 中有項目沒有識別碼
issue-duplicate-item-id = 清單 { $param } 中有重複的項目識別碼
issue-invalid-item-id = 清單 { $param } 有包含斜線的項目識別碼
issue-list-bounds = 清單 { $param } 的項目過少或過多
