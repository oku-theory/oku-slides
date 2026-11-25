data-cell-type = 有一个值不适合列“{ $column }”（该列包含{ $type ->
    [number] 数字
    [integer] 整数
    [bool] 真或假
    [date] 日期，YYYY-MM-DD
   *[text] 最多16KB的文本
}）
data-column-type = 列“{ $column }”无法无损地改为{ $type ->
    [number] 数字列
    [integer] 整数列
    [bool] 真假列
    [date] 日期列
   *[text] 文本列
}，请查看以下行：{ $rows }{ $more ->
    [0] {""}
   *[other] {""}，另有{ $more }行
}
data-no-row = 行 { $row } 不存在
data-no-column = 列 { $column } 不存在
data-id-taken = ID { $id } 已在此表中使用
data-id-invalid = ID“{ $id }”无效：请使用1到64个字母、数字、- 或 _
data-id-missing = 新的行、列或表没有 ID
data-too-large = { $what ->
    [rows] 数据将超过行数上限（{ $max }行）
    [columns] 数据将超过列数上限（{ $max }列）
    [text] 文本单元格将超过上限（{ $max }字节）
   *[overlay] 数据将超过已编辑单元格上限（{ $max }个）
}
data-invalid-name = “{ $name }”不是有效的数据名称：请使用小写字母、数字和 _，且不以数字开头
data-non-finite = 有{ $count }个数字不是有限值（NaN 或无穷大）
data-columns-invalid = 列列表为空或重复指定了某一列
data-row-width = 某行的单元格多于表格的{ $max }列
data-needs-base = 此数据将行保存在基础文件中，此编辑暂时无法读取
data-base-row-cells = 带有基础文件的表格只给出行的 ID，不给出单元格
data-row-cut = 有{ $count }行的单元格多于表头，多余的单元格已删除
data-base-invalid = 数据的基础文件已损坏或与其表格不符
