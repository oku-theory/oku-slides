# 应用界面（React）。由 ui/src/i18n.ts 通过 “messages” 请求加载。
ui-loading = 正在加载…

## 文档

ui-brand = Écho
ui-brand-templates = Écho · 模板
ui-title-rename = 重命名演示文稿
ui-title-field = 演示文稿标题
ui-status-unsaved = 有未保存的更改
ui-status-saved = 已保存
ui-status-never-saved = 尚未保存
ui-status-builtin = 内置：保存时将创建副本
ui-caption-templates = { $count ->
    [one] { $count } 个模板
   *[other] { $count } 个模板
} · { $status }
ui-caption-slides = { $count ->
    [one] { $count } 张幻灯片
   *[other] { $count } 张幻灯片
} · { $status }
ui-template-default-name = 模板
ui-template-variant-name = { $name }（变体）
ui-confirm-quit-templates = 不保存已修改的模板就退出吗？

## 提示消息

ui-toast-saved = 演示文稿已保存
ui-toast-templates-saved = 模板已保存：{ $name }
ui-toast-select-zone = 请选择一个区域来放置：“{ $text }”

## 工具栏

ui-toolbar-undo = 撤销
ui-toolbar-undo-label = 撤销：{ $label }
ui-toolbar-redo = 重做
ui-toolbar-redo-label = 重做：{ $label }
ui-toolbar-grid-hide = 隐藏网格
ui-toolbar-grid-show = 显示网格
ui-toolbar-grid-hide-title = 隐藏网格（⌘'）
ui-toolbar-grid-show-title = 显示网格（⌘'）
ui-toolbar-theme = 主题
ui-toolbar-theme-default = 默认
ui-toolbar-save-copy = 保存副本
ui-toolbar-save = 保存
ui-toolbar-finish = 完成
ui-toolbar-templates = 模板
ui-toolbar-templates-title = 创建和编辑幻灯片模板
ui-toolbar-open = 打开
ui-toolbar-add-label = 添加到幻灯片
ui-toolbar-add = 添加
ui-add-kind = { $kind ->
    [card] 卡片
    [shape] 形状
    [chart] 图表
   *[text] 文本
}
ui-toolbar-remove-template = 删除模板
ui-toolbar-remove-slide = 删除幻灯片
ui-toolbar-remove-hint = 幻灯片 { $n } · 按 ⌘Z 撤销

## 撤销标签

ui-undo-add = { $kind ->
    [card] 添加卡片
    [shape] 添加形状
    [chart] 添加图表
   *[text] 添加文本
}
ui-undo-new-template = 新模板：{ $name }
ui-undo-new-slide = 新幻灯片：{ $name }
ui-undo-grid = 网格 { $columns } × { $rows }
ui-undo-templates = 模板：{ $name }
ui-undo-move-template = 移动模板
ui-undo-move-slide = 移动幻灯片
ui-undo-remove-slide = 删除幻灯片
ui-undo-remove = 删除
ui-undo-place-on-grid = 放到网格上
ui-undo-move-on-grid = 在网格上移动
ui-undo-resize-on-grid = 在网格上调整大小
ui-undo-change-anchor = 更改锚定
ui-undo-move = 移动
ui-undo-resize = 调整大小
ui-undo-edit-text = 编辑文本
ui-undo-dictation = 听写
ui-undo-rename-set = 重命名模板集
ui-undo-rename-deck = 重命名演示文稿
ui-undo-theme = 更改主题
ui-undo-rename-template = 重命名模板
ui-undo-describe-template = 描述模板
ui-undo-chart-kind = 更改图表类型
ui-undo-chart-data = 编辑数据

## 新元素

ui-new-text = 新文本
ui-new-card-title = 卡片标题
ui-new-card-detail = 详细信息

## 幻灯片列表

ui-pile-slides = 幻灯片
ui-pile-templates = 模板
ui-pile-add-slide = + 新幻灯片
ui-pile-add-template = + 新模板
ui-pile-drag-hint = 拖动以移动 · ⌥↑ ⌥↓
ui-slide-alt = 幻灯片 { $n }

## 舞台

ui-stage-rendering = 正在渲染…
ui-stage-elements = 幻灯片元素
ui-stage-edit-text = 编辑文本
ui-stage-hint-field = Enter 确认 · Esc 取消
ui-stage-hint-block = ⌘ Enter 确认 · 每段一行，“- ” 表示项目符号
ui-field = { $kind ->
    [title] 标题
    [subtitle] 副标题
    [kicker] 眉题
    [text] 文本
   *[other] { $kind }
}
ui-element-kind = { $kind ->
    [text] 文本
    [card] 卡片
    [image] 图片
    [shape] 形状
    [group] 组合
   *[other] { $kind }
}
ui-grid-cell = 列 { $cols } · 行 { $rows }

## 幻灯片备注

ui-notes = 幻灯片工具
ui-note-template = 此模板
ui-note-template-name = 名称
ui-note-template-description = 描述
ui-note-grid-label = 模板网格
ui-note-grid = 网格
ui-note-grid-columns = 列
ui-note-grid-rows = 行
ui-stepper-less = { $label }：减少一个
ui-stepper-more = { $label }：增加一个
ui-note-tip = 提示
ui-note-tip-text = ⌘D 听写。⌘' 显示网格。双击编辑文本。

## 听写

ui-dictation = 听写
ui-dictation-start = 听写
ui-dictation-start-title = 听写（⌘D）
ui-dictation-finish = 结束听写
ui-dictation-finish-title = 结束（⌘D）
ui-dictation-cancel = 取消
ui-dictation-preparing = 正在准备听写…
ui-dictation-heard-nothing = 没有听到声音：请靠近麦克风一些再说
ui-dictation-to-cursor = 插入到光标处
ui-dictation-to-field = 替换{ $field }
ui-dictation-to-block = 插入到所选区块之后
ui-dictation-new-block = 新文本块
ui-dictation-select-zone = 选择一个区域
ui-dictation-downloading = 正在下载 { $name } 模型
ui-dictation-download-progress = { $percent }% · { $received } / { $total } · 仅需一次
ui-dictation-listening = 正在聆听 · { $time }
ui-dictation-transcribing = 正在转写最后一句…
ui-dictation-stop-in = { $duration } 后停止
ui-dictation-speak = 请说话，文字将显示在这里…
ui-dictation-auto-stop = 静音 { $duration } 后停止
ui-dictation-to-download = 需下载 { $size }
ui-dictation-settings = 听写设置
ui-dictation-model-title = 识别模型
ui-dictation-model-intro = Whisper，在本机运行（CTranslate2）。每个模型都支持多种语言。
ui-dictation-note-turbo = 推荐，快速且准确
ui-dictation-note-large = 最准确，但较慢
ui-dictation-note-base = 轻量，适合配置较低的电脑
ui-dictation-installed = 已安装
ui-dictation-not-installed = 待下载
ui-dictation-language = 口述语言
ui-dictation-language-auto = 自动检测
ui-dictation-auto-stop-label = 静音多久后自动停止
ui-dictation-silence = 静音时长
ui-dictation-phrases-note = 只转写有语音的片段，在你说话时逐句进行。
ui-dictation-download-now = 立即下载（{ $size }）
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } 秒
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } GB
ui-size-mb = { $value } MB

## 模板选择器

ui-picker-new-slide = 新幻灯片
ui-picker-new-template = 新模板，基于…
ui-picker-set = 模板
ui-picker-set-mine = { $name } · 我的
ui-picker-keys = ← → 浏览 · Enter 添加 · Esc 关闭
ui-picker-close = 关闭
ui-picker-previous = 上一个模板
ui-picker-next = 下一个模板
ui-picker-list = 幻灯片模板

## 图表

ui-chart-panel = 图表
ui-chart-kinds = 图表类型
ui-chart-kind = { $kind ->
    [line] 折线图
    [pie] 饼图
    [scatter] 散点图
    [radar] 雷达图
   *[bar] 柱形图
}
ui-chart-no-data = 此图表没有数据表。
ui-chart-column-name = 第 { $col } 列的名称
ui-chart-remove-column = 删除列
ui-chart-cell = 第 { $row } 行，第 { $col } 列
ui-chart-remove-row = 删除行
ui-chart-add-row = + 行
ui-chart-add-column = + 列
ui-chart-series = 系列 { $n }

## 文件

ui-file-open-prompt = 要打开的文件路径（.echo.pdf 或 .json）
ui-file-save-prompt = 另存为（完整路径）
