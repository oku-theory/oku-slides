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
data-source-mismatch = 数据的来源与其类型不符：手动数据包含表格，查询数据包含查询
data-inline-query = 查询数据不能是内联数据：内联数据总是来自表格
data-query-parse = 无法读取查询：请检查第 { $line } 行第 { $column } 列
data-query-construct = 查询使用了“{ $construct }”，数据查询不支持它
data-query-too-long = 查询超过 { $max }字节
data-query-too-deep = 查询嵌套过深
data-query-rewrite = 无法为引擎准备查询；这是 Oku Slides 的缺陷
data-query-unknown-function = 查询调用了“{ $function }”，数据查询不支持它
data-query-run = 查询失败：{ $reason }
data-query-input-failed = 其输入“{ $input }”失败
data-query-too-large = { $what ->
    [rows] 查询结果超过行数上限（{ $max }行）；结果从不截断
    [columns] 查询结果超过列数上限（{ $max }列）；结果从不截断
   *[text] 查询结果的文本单元格超过上限（{ $max }字节）；结果从不截断
}
data-query-column-type = 结果列“{ $column }”的类型无法存入数据：请将其转换为文本、数字、整数、布尔值或日期
data-query-duplicate-column = 有两个结果列都名为“{ $column }”：请用 AS 重命名其中一个
data-query-float-operand = “{ $operand }”含有小数，无法对其使用“{ $function }”：请先将其转换为整数
data-query-timeout = 查询运行超过 { $seconds }秒，已被停止
