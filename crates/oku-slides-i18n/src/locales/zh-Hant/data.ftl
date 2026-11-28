data-cell-type = 有一個值不適合欄「{ $column }」（該欄包含{ $type ->
    [number] 數字
    [integer] 整數
    [bool] 真或假
    [date] 日期，YYYY-MM-DD
   *[text] 最多16KB的文字
}）
data-column-type = 欄「{ $column }」無法無損地改為{ $type ->
    [number] 數字欄
    [integer] 整數欄
    [bool] 真假欄
    [date] 日期欄
   *[text] 文字欄
}，請查看以下列：{ $rows }{ $more ->
    [0] {""}
   *[other] {""}，另有{ $more }列
}
data-no-row = 列 { $row } 不存在
data-no-column = 欄 { $column } 不存在
data-id-taken = ID { $id } 已在此表格中使用
data-id-invalid = ID「{ $id }」無效：請使用1到64個字母、數字、- 或 _
data-id-missing = 新的列、欄或表格沒有 ID
data-too-large = { $what ->
    [rows] 資料將超過列數上限（{ $max }列）
    [columns] 資料將超過欄數上限（{ $max }欄）
    [text] 文字儲存格將超過上限（{ $max }位元組）
   *[overlay] 資料將超過已編輯儲存格上限（{ $max }個）
}
data-invalid-name = 「{ $name }」不是有效的資料名稱：請使用小寫字母、數字和 _，且不以數字開頭
data-non-finite = 有{ $count }個數字不是有限值（NaN 或無限大）
data-columns-invalid = 欄清單為空或重複指定了某一欄
data-row-width = 某列的儲存格多於表格的{ $max }欄
data-needs-base = 此資料將列儲存在基礎檔案中，此編輯暫時無法讀取
data-base-row-cells = 具有基礎檔案的表格只提供列的 ID，不提供儲存格
data-row-cut = 有{ $count }列的儲存格多於表頭，多餘的儲存格已刪除
data-base-invalid = 資料的基礎檔案已損毀或與其表格不符
data-source-mismatch = 資料的來源與其類型不符：手動資料包含表格，查詢資料包含查詢
data-inline-query = 查詢資料不能是內嵌資料：內嵌資料總是來自表格
data-query-parse = 無法讀取查詢：請檢查第 { $line } 行第 { $column } 欄
data-query-construct = 查詢使用了「{ $construct }」，資料查詢不支援它
data-query-too-long = 查詢超過 { $max }位元組
data-query-too-deep = 查詢巢狀過深
data-query-rewrite = 無法為引擎準備查詢；這是 Oku Slides 的缺陷
data-query-unknown-function = 查詢呼叫了「{ $function }」，資料查詢不支援它
data-query-run = 查詢失敗：{ $reason }
data-query-input-failed = 其輸入「{ $input }」失敗
data-query-too-large = { $what ->
    [rows] 查詢結果超過列數上限（{ $max }列）；結果絕不截斷
    [columns] 查詢結果超過欄數上限（{ $max }欄）；結果絕不截斷
   *[text] 查詢結果的文字儲存格超過上限（{ $max }位元組）；結果絕不截斷
}
data-query-column-type = 結果欄「{ $column }」的類型無法存入資料：請將其轉換為文字、數字、整數、布林值或日期
data-query-duplicate-column = 有兩個結果欄都名為「{ $column }」：請用 AS 重新命名其中一個
data-query-float-operand = 「{ $operand }」含有小數，無法對其使用「{ $function }」：請先將其轉換為整數
data-query-timeout = 查詢執行超過 { $seconds }秒，已被停止
