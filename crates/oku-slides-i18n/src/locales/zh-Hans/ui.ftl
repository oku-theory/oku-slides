# 应用界面（React）。由 ui/src/i18n.ts 通过 “messages” 请求加载。
ui-loading = 正在加载…

## 文档

ui-brand = Oku Slides
ui-brand-pack = { $kind ->
    [layouts] { $name } · 版式包
   *[other] { $name } · 小组件包
}
ui-title-rename = 重命名演示文稿
ui-title-field = 演示文稿标题
ui-status-unsaved = 有未保存的更改
ui-status-saved = 已保存
ui-status-never-saved = 尚未保存
ui-caption-pack = { $kind ->
    [layouts] { $count ->
        [one] { $count }个版式
       *[other] { $count }个版式
    }
   *[other] { $count ->
        [one] { $count }个小组件
       *[other] { $count }个小组件
    }
} · { $status }
ui-caption-slides = { $count ->
    [one] { $count }张幻灯片
   *[other] { $count }张幻灯片
} · { $status }
ui-confirm-quit-pack = 不保存已修改的包就退出吗？

## 提示消息

ui-toast-copied = 副本已保存
ui-toast-field-gone = 正在编辑的字段已不存在：输入的内容已丢失。
ui-toast-edit-replaced = 此文本在你编辑期间被更改：你的版本已替换它。撤销可恢复另一个版本。

## 工具栏

ui-toolbar-undo = 撤销
ui-toolbar-undo-label = 撤销：{ $label }
ui-toolbar-redo = 重做
ui-toolbar-redo-label = 重做：{ $label }
ui-divider-pile = 幻灯片堆宽度
ui-divider-notes = 智能体对话宽度
ui-divider-code = 代码大小
ui-divider-band = 设置区高度
ui-toolbar-theme = 主题
ui-toolbar-theme-embedded = 文件主题
ui-toolbar-finish = 完成
ui-pack-deck-only = 仅保存在此演示文稿中
ui-toolbar-open = 打开
ui-menu = 菜单
ui-toolbar-add-label = 添加到幻灯片
ui-add-kind = { $kind ->
    [card] 卡片
    [shape] 形状
    [chart] 图表
    [table] 表格
   *[text] 文本
}
ui-add-kind-description = { $kind ->
    [card] 边框中的标题和简短文本
    [shape] 彩色形状
    [chart] 由数据表绘制的图表
    [table] 数据表
   *[text] 一段文本
}
ui-toolbar-remove-slide = 删除幻灯片
ui-toolbar-remove-hint = 幻灯片 { $n } · 按 ⌘Z 撤销

## 撤销标签

ui-undo-add = { $kind ->
    [card] 添加卡片
    [shape] 添加形状
    [chart] 添加图表
    [table] 添加表格
   *[text] 添加文本
}
ui-undo-add-widget = 添加“{ $name }”
ui-undo-new-slide = 新幻灯片：{ $name }
ui-undo-move-slide = 移动幻灯片
ui-undo-remove-slide = 删除幻灯片
ui-undo-remove = 删除
ui-undo-paste = 粘贴
ui-undo-place-on-grid = 放到网格上
ui-undo-move-on-grid = 在网格上移动
ui-undo-resize-on-grid = 在网格上调整大小
ui-undo-change-anchor = 更改锚定
ui-undo-move = 移动
ui-undo-resize = 调整大小
ui-undo-edit-slide = 编辑幻灯片
ui-undo-rename-pack = 重命名包
ui-undo-rename-deck = 重命名演示文稿
ui-undo-chart-kind = 更改图表类型
ui-undo-chart-data = 编辑数据
ui-undo-chart-color = 为系列设置颜色
ui-undo-chart-stack = { $stacked ->
    [yes] 堆叠系列
   *[no] 取消堆叠系列
}

## 新元素

ui-new-text = 新文本
ui-new-card-title = 卡片标题
ui-new-card-detail = 详细信息

## 幻灯片列表

ui-pile-slides = 幻灯片
ui-pile-layouts = 版式
ui-pile-add-slide = + 新幻灯片
ui-pile-drag-hint = 拖动以移动 · ⌥↑ ⌥↓
ui-slide-alt = 幻灯片 { $n }

## 舞台

ui-stage-rendering = 正在渲染…
ui-stage-widget-pack = 从堆栈中打开一个小组件，或新建一个。
ui-stage-elements = 幻灯片元素
ui-stage-fields = 幻灯片字段
ui-stage-edit-text = 编辑文本
ui-stage-hint-plain = Enter 或 Tab 转到下一个 · Esc 取消
ui-stage-hint-rich = Tab 转到下一个 · ⌘ Enter 确认 · “- ” 表示项目符号 · ⌘B 粗体，⌘I 斜体，⌘K 链接
ui-field = { $kind ->
    [title] 标题
    [subtitle] 副标题
    [kicker] 眉题
    [text] 文本
    [body] 正文
    [author] 作者
    [date] 日期
    [organization] 组织
   *[other] { $kind }
}
ui-element-kind = { $kind ->
    [text] 文本
    [card] 卡片
    [image] 图片
    [shape] 形状
    [chart] 图表
    [table] 表格
    [stack] 堆叠
   *[other] { $kind }
}
ui-grid-cell = 列 { $cols } · 行 { $rows }
ui-ghost-zone = 区域

## 幻灯片备注

ui-notes = 幻灯片工具
ui-stepper-less = { $label }：减少一个
ui-stepper-more = { $label }：增加一个
ui-list-item = 项目
ui-list-add = 添加{ $item }
ui-list-remove = 删除{ $item }
ui-list-move = 移动{ $item }
ui-list-full = 列表已满（{ $max }）
ui-undo-list-move = 移动{ $item }
ui-agent-color-other = 其他颜色
ui-agent-thread = 智能体
ui-agent-new-conversation = 新对话
ui-agent-undo-turn = 撤销本轮
ui-agent-detach = 在单独的窗口中打开智能体
ui-agent-attach = 将智能体放回主窗口
ui-agent-window-empty = 此标签页没有智能体
ui-agent-kept-in-draft = 没有可写入的位置：文本已保留在草稿中
ui-agent-busy-elsewhere = 智能体正在 { $place } 中回复…
ui-agent-scope-slides = 幻灯片
ui-agent-scope-atelier = 工作坊
ui-draft = 草稿
ui-draft-send = 发送给智能体
ui-draft-send-title = 发送给智能体 (⌘↩)
ui-draft-discard = 丢弃
ui-agent-state-idle = 智能体
ui-agent-state-starting = 智能体正在启动…
ui-agent-state-working = 智能体正在工作…
ui-agent-dock = 进行中
ui-agent-dock-settings = 设置
ui-agent-jump = ↓ 本轮
ui-agent-tool-input = 参数
ui-agent-tool-output = 结果
ui-agent-sheet = 智能体的问题
ui-agent-sheet-keys = 1–4 选择 · Enter 确认 · E 修改 · Esc 取消
ui-agent-variants = 变体
ui-agent-drawer-keys = 点击或 1–4 · Esc 关闭
ui-agent-confirm = 智能体的确认

## 听写

ui-dictation = 听写
ui-dictation-start = 听写
ui-dictation-start-title = 听写（⌘D）
ui-dictation-type = 给智能体写消息
ui-dictation-type-title = 给智能体写消息（⌘K）
ui-dictation-agent-stop = 停止智能体（⌘⇧D 或 Esc）
ui-dictation-agent-working = 智能体正在工作…
ui-dictation-agent-working-hint = ⌘⇧D 或 Esc 可停止
ui-dictation-finish = 结束听写
ui-dictation-finish-title = 结束（⌘D）
ui-dictation-cancel = 取消
ui-dictation-preparing = 正在准备听写…
ui-dictation-heard-nothing = 没有听到声音：请靠近麦克风一些再说
ui-dictation-to-cursor = 插入到光标处
ui-dictation-to-draft = 到草稿
ui-dictation-downloading = 正在下载 { $name } 模型
ui-dictation-download-progress = { $percent }% · { $received } / { $total } · 仅需一次
ui-dictation-listening = 正在聆听 · { $time }
ui-dictation-transcribing = 正在转写最后一句…
ui-dictation-stop-in = { $duration } 后停止
ui-dictation-speak = 请说话，文字将显示在这里…
ui-dictation-auto-stop = 静音 { $duration } 后停止
ui-dictation-to-download = 需下载 { $size }
ui-dictation-settings = 设置
ui-dictation-model-title = 识别模型
ui-dictation-model-intro = Whisper，在本机运行（CTranslate2）。每个模型都支持多种语言。
ui-dictation-note-turbo = 推荐，快速且准确
ui-dictation-note-large = 最准确，但较慢
ui-dictation-note-base = 轻量，适合配置较低的电脑
ui-dictation-installed = 已安装
ui-dictation-not-installed = 待下载
ui-dictation-language = 口述语言
ui-dictation-language-auto = 自动检测
ui-dictation-note-apple = 速度最快，系统内置
ui-dictation-model-unavailable = 不支持该语言
ui-dictation-model-intro-apple = Apple（系统内置）或在本机运行的 Whisper（CTranslate2）。所有模型都支持多种语言。
ui-dictation-language-auto-resolved = 自动（{ $language }）
ui-dictation-to-download-language = 需下载语言
ui-dictation-auto-stop-label = 静音多久后自动停止
ui-dictation-silence = 静音时长
ui-dictation-phrases-note = 只转写有语音的片段，在你说话时逐句进行。
ui-dictation-download-now = 立即下载（{ $size }）
ui-dictation-download-language = 下载语言
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } 秒
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } GB
ui-size-mb = { $value } MB

## 模板选择器

ui-picker-new-slide = 新幻灯片
ui-picker-set-mine = { $name } · 我的
ui-picker-set-received = { $name } · 收到的
ui-picker-keys = ← → 浏览 · Enter 添加 · Esc 关闭
ui-picker-close = 关闭
ui-picker-list = 版式
ui-widget-picker-filter = 按名称、标签或描述筛选
ui-widget-picker-empty = 没有与“{ $query }”匹配的小组件。
ui-widget-picker-list = 小组件
ui-widget-picker-keys = 方向键浏览 · Enter 添加 · Esc 关闭

## 图表

ui-chart-panel = 图表
ui-chart-kinds = 图表类型
ui-chart-kind = { $kind ->
    [line] 折线图
    [pie] 饼图
    [scatter] 散点图
    [radar] 雷达图
    [waterfall] 瀑布图
   *[bar] 柱形图
}
ui-chart-no-data = 此图表没有数据表。
ui-chart-options = 图表选项
ui-chart-stack = 堆叠
ui-chart-waterfall-hint = 勾选「合计」即为合计柱。
ui-chart-colors = 系列颜色
ui-chart-color-of = { $name } 的颜色
ui-chart-color-auto = 自动
ui-chart-color-pick = 颜色 { $n }
ui-chart-column-name = 第 { $col } 列的名称
ui-chart-remove-column = 删除列
ui-chart-cell = 第 { $row } 行，第 { $col } 列
ui-chart-remove-row = 删除行
ui-chart-add-row = 行
ui-chart-add-column = 列
ui-chart-series = 系列 { $n }

## 表格

ui-table-panel = 表格
ui-table-options = 表格选项
ui-table-header-row = 标题行
ui-table-header-column = 首列作标题
ui-table-striped = 斑马纹行
ui-table-alignment = 列对齐
ui-table-align-of = { $name }的对齐方式
ui-table-align = { $align ->
    [start] 起始
    [center] 居中
    [end] 末尾
   *[auto] 自动
}
ui-table-column = 列 { $n }
ui-table-no-data = 此表格没有数据。
ui-undo-table-option = 表格选项
ui-undo-table-align = 对齐该列
ui-undo-table-data = 编辑表格
ui-show-as = 显示为
ui-show-as-table = 显示为表格
ui-show-as-chart = 显示为图表
ui-undo-show-as-table = 显示为表格
ui-undo-show-as-chart = 显示为图表

## 文件

ui-file-open-prompt = 要打开的文件路径（.oku-slides.pdf、.json 或 .pptx）
ui-file-filter-presentations = 演示文稿
ui-file-filter-pptx = PowerPoint
ui-file-save-prompt = 另存为（完整路径）

## 标签页

ui-tabs = 已打开的演示文稿
ui-tab-new = 新建标签页
ui-tab-close = 关闭标签页
ui-tab-unsaved = 未保存的更改
ui-tab-pack = 包
ui-tab-move-new-window = 移到新窗口
ui-tab-move-to = 移到窗口 { $n }
ui-tab-close-others = 关闭其他标签页
ui-close-cancel = 取消
ui-disk-replaced-question = 磁盘上的 { $file } 已被另一个演示文稿替换。
ui-disk-replaced-overwrite = 覆盖
ui-disk-replaced-save-as = 另存为…
ui-same-deck-question = { $file } 是标签页“{ $title }”中已打开演示文稿的一个版本。
ui-same-deck-merge = 合并到该标签页
ui-same-deck-apart = 单独打开
ui-history = 历史记录
ui-history-version = { $date } 的版本
ui-history-restore = 恢复
ui-history-back = 返回
ui-history-anonymous = 匿名
ui-history-slider = 演示文稿的版本
ui-history-removed-since = 此后已删除
ui-history-changed-since = 此后已修改
ui-history-added-since = 此后新增的幻灯片：{ $count }
ui-help = 帮助
ui-help-caption = 第{ $index }张，共 { $count }张 · 只读
ui-slide-previous = 上一张幻灯片
ui-slide-next = 下一张幻灯片
ui-present = 演示
ui-present-slide = 第{ $index }张，共 { $count }张
ui-present-screen = 全屏演示
ui-toolbar-params-hide = 隐藏设置
ui-toolbar-params-show = 显示设置
ui-help-close = 关闭帮助
ui-help-search = 搜索帮助
ui-help-search-placeholder = 搜索…
ui-help-search-empty = 无结果
ui-help-search-count = { $count }个结果
ui-widgets-files = 小组件文件
ui-widgets-common = 共用
ui-widgets-new = 新建小组件
ui-widgets-name = 小组件名称（小写字母、数字和连字符）
ui-widgets-name-taken = 此名称已被使用
ui-widgets-name-letter = 名称必须以字母开头
ui-widgets-valid = 有效
ui-widgets-errors = 错误
ui-widgets-empty = 此包还没有小组件。
ui-widgets-preview = 小组件预览
ui-widgets-no-preview = 无预览
ui-widgets-field-bound = 已关联到演示文稿：此处只读
ui-widgets-source = 源代码
ui-widgets-unsaved = 未保存
ui-widgets-in-error = 有错误的小组件
ui-widgets-none-in-error = 没有小组件有错误
ui-widgets-section = 小组件
ui-widgets-open = 打开小组件 { $name }
ui-widgets-delete-named = 删除 { $name }
ui-widgets-confirm-delete = 删除小组件“{ $name }”?
ui-widgets-rename-hint = 双击或按 F2 重命名
ui-pack-tabs = 包的标签页
ui-pack-layouts-tab = 版式
ui-theme-tab = 主题
ui-theme-picker = 主题
ui-theme-name = 主题名称
ui-theme-unsaved = 主题更改未保存
ui-widgets-close-tab = 关闭 { $name }
ui-widgets-removed-by-agent = 智能体删除了小组件 { $name }。

## 导入

ui-import-title = 导入 PowerPoint 演示文稿
ui-import-mode = 导入方式
ui-import-hifi = 忠实副本
ui-import-hifi-note = 保留原稿的外观，使用为此演示文稿生成的主题和版式。
ui-import-retrofit = 使用我的版式
ui-import-retrofit-note = 用库中的版式包和主题重建幻灯片。
ui-import-layout-packs = 版式包
ui-import-theme-default = 默认主题
ui-import-no-packs = 库中没有版式包。
ui-import-tune = 由智能体微调
ui-import-tune-note = 智能体将每张幻灯片与原稿比较并修正差异。
ui-import-tune-note-retrofit = 智能体将每张幻灯片与原稿比较，并为版式无法呈现的形状设计小组件。
ui-import-tune-unavailable = 没有可用的智能体：{ $reason }
ui-import-submit = 导入
ui-import-busy = 正在导入…
ui-update-available = Oku Slides { $version } 现已推出。
ui-update-install = 安装并重新启动
ui-update-see-release = 查看发行版
ui-update-later = 稍后
ui-update-installing = 正在下载更新… { $percent }%
ui-update-downloading = 正在下载更新…
ui-update-up-to-date = Oku Slides 已是最新版本。

ui-shortcut-theme = 主题, theme, themes
