op-slide-not-found = 找不到幻灯片：{ $subject }
op-element-not-found = 找不到元素：{ $subject }
op-duplicate-id = 标识符已被使用：{ $subject }
op-index-out-of-range = 位置超出范围
op-invalid-value = { $subject } 的值无效
op-invalid-value-because = { $subject } 的值无效：{ $reason }
op-value-no-value = 该参数没有值
op-value-no-dataset = 图表没有数据集
op-value-grid-tracks = 网格的列数和行数为 1 到 24
op-value-bad-item-id = 项目 ID 是不含“/”的非空文本
op-value-missing-item-id = 每个项目都需要一个 ID
op-value-too-deep = 元素嵌套过深
op-value-empty-path = 路径为空
op-value-not-text = 需要文本或 null
op-value-empty-frame = 宽度和高度必须大于 0
op-value-blocks-off-grid = 流式插槽由幻灯片排布，网格会将其隐藏；请先将每个元素移到空闲插槽
op-value-outside-grid = 单元格超出网格
op-value-not-tokens = 需要对象或 null
op-value-not-a-slot = 此参数不是插槽
op-value-out-of-bounds = 数值超出允许范围
op-value-unreadable = 值的格式不符合预期
op-value-valign = 应为 top、middle 或 bottom
op-value-path = 路径由 M、L、C、Q、Z 命令及其数字组成，以 M 开头
op-unknown-widget = 未知的组件：{ $subject }
op-unknown-namespace = 此演示文稿没有包 { $pack }。
op-pack-required = 包 { $pack } 是 { $by } 所需的；请先移除后者。
op-pack-in-use = 包 { $pack } 仍被 { $count }个项目使用。
op-pack-requirement-missing = 包 { $pack } 需要 { $required }，但此演示文稿中没有。
op-base-required = 每个演示文稿都保留基础组件包。
op-pack-kind-conflict = 此演示文稿已有另一类型（组件或版式）的包 { $pack }。
op-theme-cycle = 主题循环相互扩展：{ $chain }。
op-theme-closure = 此演示文稿固定的主题并非恰好是其主题及其父主题（{ $namespace }）。
op-unknown-param = 未知的参数：{ $subject }
op-param-type-because = { $subject } 的值无效：{ $reason }
op-param-shape-text = { $nullable ->
    [yes] 需要文本或 null
   *[no] 需要文本
}
op-param-shape-choice = { $nullable ->
    [yes] 需要其值之一或 null
   *[no] 需要其值之一
}
op-param-shape-paragraphs = { $nullable ->
    [yes] 需要段落列表或 null
   *[no] 需要段落列表
}
op-param-shape-elements = { $nullable ->
    [yes] 需要元素列表或 null
   *[no] 需要元素列表
}
op-param-shape-flag = { $nullable ->
    [yes] 需要布尔值或 null
   *[no] 需要布尔值
}
op-param-shape-number = { $nullable ->
    [yes] 需要数字或 null
   *[no] 需要数字
}
op-param-shape-bounded-number = { $nullable ->
    [yes] 需要范围内的数字或 null
   *[no] 需要范围内的数字
}
op-param-shape-grid = { $nullable ->
    [yes] 需要网格或 null
   *[no] 需要网格
}
op-param-shape-chart-option = { $nullable ->
    [yes] 需要图表选项或 null
   *[no] 需要图表选项
}
op-param-shape-table-option = { $nullable ->
    [yes] 需要表格选项或 null
   *[no] 需要表格选项
}
op-param-shape-chart-or-table-option = { $nullable ->
    [yes] 需要图表或表格选项或 null
   *[no] 需要图表或表格选项
}
op-param-shape-items = { $nullable ->
    [yes] 需要项目列表或 null
   *[no] 需要项目列表
}
op-param-shape-list = { $nullable ->
    [yes] 需要列表或 null
   *[no] 需要列表
}
op-param-shape-add-element-only = { $nullable ->
    [yes] 元素通过 add_element 添加，或 null
   *[no] 元素通过 add_element 添加
}
op-param-shape-set-param-only = { $nullable ->
    [yes] 值通过 set_param 设置，或 null
   *[no] 值通过 set_param 设置
}
op-param-shape-path-option = { $nullable ->
    [yes] 应为形状路径（带键 d 的对象）或 null
   *[no] 应为形状路径（带键 d 的对象）
}
op-item-not-found = 没有项目 { $subject }
op-list-full = 列表 { $subject } 已满（最多 { $max }个项目）
op-list-minimum = 列表 { $subject } 至少需要 { $min }个项目
op-list-cell-too-long = 列表 { $subject } 的单元格过长（列 { $column }）
op-list-id-too-long = 列表 { $subject } 的项目 ID 过长
op-list-too-many-items = 列表 { $subject } 的项目过多（最多 { $max }个）
op-slot-rejects = 此处不接受 { $subject }
op-wrong-category = 此处不能使用 { $subject }
op-choice-out-of-range = { $subject } 不允许此值
transaction-refused = 操作 { $index } 被拒绝：{ $reason }

sources-unreadable = 源文件无法读取
sources-not-sources = 这不是 Oku Slides 源文件归档
sources-unsupported-version = 这些源文件由更新版本的 Oku Slides 写入
sources-bad-name = 源文件中的文件名被拒绝
sources-missing-deck = 源文件中缺少演示文稿
sources-too-large = 此文件中的源文件过大，无法打开
sources-too-deep = 此演示文稿的元素嵌套过深

issue-empty-slide-id = 幻灯片标识符为空
issue-duplicate-slide-id = 幻灯片标识符重复
issue-duplicate-element-id = 元素标识符重复
issue-empty-element-id = 标识符为空
issue-cell-outside-grid = 单元格超出网格
issue-empty-frame = 框架尺寸为零
issue-free-without-frame = 自由元素缺少框架
issue-unknown-widget = 未知组件：{ $kind }
issue-wrong-category = { $kind } 不能在此处使用
issue-unknown-param = 未知参数：{ $param }
issue-param-type = { $param } 的值无效
issue-choice-out-of-range = { $param } 不允许此值
issue-slot-rejects = 此处不接受 { $kind }
issue-placed-in-flow = 在排版列表中以框架放置的元素
issue-unknown-bind = { $param } 关联到未知的演示文稿设置
issue-slot-too-deep = 元素嵌套过深
issue-invalid-style = 无效的样式值：{ $field }（{ $reason ->
        [valign] { op-value-valign }
        [out-of-bounds] { op-value-out-of-bounds }
       *[other] { op-value-unreadable }
    }）
issue-missing-asset = 缺少资源：{ $asset }
issue-pack-version-mismatch = 已丢弃被篡改的模板包版本：{ $version }
issue-pack-version-missing = 源文件中缺少模板包版本：{ $version }
issue-pack-pin-mismatch = 内嵌的包 { $pack } 与演示文稿所期望的不符。
issue-pack-kind-mismatch = 内嵌的包 { $pack } 不是演示文稿所期望的类型（组件或版式）。
issue-pack-kind-conflict = 命名空间 { $pack } 同时被固定为组件包和版式包：只能使用其中一个。
issue-pack-requirement-missing = 包 { $pack } 需要 { $required }，但此演示文稿中没有。
issue-theme-version-mismatch = 已丢弃被篡改的主题版本：{ $version }
issue-theme-version-missing = 源文件中缺少主题版本：{ $version }
issue-theme-pin-mismatch = 内嵌的主题 { $namespace } 与演示文稿所期望的不符。
issue-theme-closure = 此演示文稿固定的主题并非恰好是其主题及其父主题（{ $namespace }）。
pack-token-entry-invalid = 包清单中的组件令牌“{ $path }”无效。
issue-unsupported-link = 不支持的链接：{ $link }
issue-highlight-zero = 高亮颜色从 1 开始编号
issue-unreadable-element = 无法读取的元素，已移出
issue-slot-repaired = 元素没有已知的插槽，已放入 { $slot }
issue-unreadable-item = 无法读取的列表项，已移出
issue-unreadable-row = 无法读取的表格行，已移出
issue-data-truncated = 数据过大，读取时已截断
issue-unreadable-slide = 无法读取的幻灯片，已移出
issue-unreadable-field = 无法读取的演示文稿字段 { $field }，已重置
op-invalid-chart = 图表 { $subject }：{ $reason }
op-invalid-table = 表格 { $subject }：{ $reason }

replica-corrupted = 此文件中的演示文稿历史已损坏
replica-too-large = 此文件中的演示文稿历史过大，无法打开
replica-write = 无法更新演示文稿历史
replica-unrelated = 这两个演示文稿没有共同的来源
replica-unsupported-version = 此演示文稿使用的格式无法被此版本的 Oku Slides 读取
replica-too-deep = 无法保存此演示文稿：已删除的元素在其历史记录中嵌套过深。请撤销最近的移动操作，然后重新保存。

widget-unreadable = 无法读取小组件声明
widget-fault-empty-name = 小组件 { $widget }：某个小组件或参数没有名称
widget-fault-duplicate-widget = 小组件 { $widget } 被声明了两次
widget-fault-duplicate-param = 小组件 { $widget } 两次声明了同一个参数
widget-fault-empty-choice = 小组件 { $widget }：某个选项没有任何值
widget-fault-default-not-in-choice = 小组件 { $widget }：选项的默认值不在其取值范围内
widget-fault-default-wrong-type = 小组件 { $widget }：默认值与其参数类型不匹配
widget-fault-bind-not-plain = 小组件 { $widget }：只有纯文本参数才能绑定到演示文稿
widget-fault-unknown-slot-kind = 小组件 { $widget }：插槽接受了未知的元素小组件
widget-fault-empty-list = 小组件 { $widget }：列表至少需要一列
widget-fault-column-type = 小组件 { $widget }：列表的列可以是 plain、rich、number、flag、choice 或 asset（或嵌套一层的 p.list）；若每项需要自由内容，请使用带子小组件的 p.slot
widget-fault-reserved-name = 小组件 { $widget }：保留名称：id、kind，或包含 / 的名称
widget-fault-list-bounds = 小组件 { $widget }：列表的最大值小于其最小值
widget-fault-list-default-too-large = 小组件 { $widget }：列表的默认值超出列表的大小限制
widget-fault-list-depth = 小组件 { $widget }：嵌套列表不能再包含列表列；列表只能嵌套一层
widget-fault-list-default-below-min = 小组件 { $widget }：嵌套列表默认值的项数少于其最小值
widget-fault-slide-sizing = 小组件 { $widget }：这是幻灯片，只有元素小组件才能声明 grow 或 typography
widget-fault-pack-category = 小组件 { $widget }：与所在包的类型不符，布局包只含幻灯片小组件，小组件包只含元素小组件
widget-fault-unqualified = 小组件 { $widget }：没有包命名空间
issue-empty-item-id = 列表 { $param } 中有项目没有标识符
issue-duplicate-item-id = 列表 { $param } 中有重复的项目标识符
issue-invalid-item-id = 列表 { $param } 有包含斜杠的项目标识符
issue-list-bounds = 列表 { $param } 的项目过少或过多
