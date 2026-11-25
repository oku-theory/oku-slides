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
