# 代理：界面、确认、对话流。

agent-instructions = 你为与你对话的用户编辑一份 Oku Slides 演示文稿。只能通过 oku-slides 工具进行操作。修改前，先用 deck_outline、slide 和 render_slide 读取幻灯片组；用 apply 进行修改（每次修改都可以撤销）。当决定权在用户时，用 ask_choice、ask_confirm 或 ask_rewrite 询问；用 show_variants 提供版式，用 show_params 提供实时设置。用用户的语言简要回复。每条消息都以描述用户所见内容的 <oku-slides-context> 块开头。每次回复后，如有有用的后续步骤，用 suggest_next 提供 1 到 4 个（标签简短；点击后会把指令发给你）。添加元素之前，先读取 widgets 工具，了解可用的类型及其参数。

agent-instructions-widgets = 创建组件:组件是模板包中的一个 Typst 文件;在演示文稿模式和模板模式下都使用 widget_sources、widget_source、set_widget、remove_widget 和 preview_widget。 kpi 文件的形式:#import "/oku-slides/lib.typ": widget, field, fields-stack, params as p, color-of, size-of,然后 #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …);声明的名称就是文件名。 参数:描述符 p.plain、p.rich、p.flag、p.number、p.choice、p.asset、p.slot、p.grid。 可编辑文本:field(ctx, "value", args.value),用 fields-stack 堆叠。 颜色和大小:主题令牌 (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")),绝不写死数值。 共享代码:_commun 文件,用 #import "_commun.typ": … 导入。 循环:用 set_widget 写入,读取状态及其诊断(行、列),查看 preview_widget,修正。 要把它放进演示文稿:使用 add_element,kind 为组件名称。 幻灯片小组件声明 category: "slide"（widget("name", category: "slide", params: …)）；否则为元素。描述符中的 label: 和 default: 可省略（p.plain()、p.plain(label: "标题", default: "…")）。

agent-surface-cancel = 取消
agent-surface-confirm = 确认
agent-surface-confirm-danger = 是，执行
agent-surface-rewrite = 改写
agent-surface-pick = 选择{ $n }
agent-surface-keep = 保留原文
agent-surface-edit = 应用前编辑
agent-surface-close = 关闭
agent-surface-variants = 变体
agent-surface-permission-always = 永久
agent-surface-permission-title = 智能体请求一项权限
agent-undo-turn = 代理：{ $text }

agent-tools-unavailable = 代理的工具未能启动。

agent-undo-apply = 代理更改
agent-undo-variant = 变体：{ $label }
agent-undo-param = 设置：{ $label }
agent-undo-rewrite = 改写

agent-confirm-save = 要将演示文稿另存为{ $path }吗？
agent-confirm-save-over = 要替换{ $path }吗？现有文件将被覆盖。
agent-confirm-open = 要打开{ $path }吗？
agent-open-elsewhere = 此文件已在另一个标签页中打开。
agent-confirm-new = 要新建一个演示文稿吗？
agent-confirm-new-dirty = 要新建一个演示文稿吗？未保存的更改将丢失。
agent-confirm-save-templates = 要保存模板集“{ $name }”吗？
agent-confirm-delete-set = 要删除模板集“{ $name }”吗？此操作无法撤销。
agent-confirm-setting = 要更改{ $key }：{ $old } → { $new }吗？

agent-not-found = 未找到代理：{ $command }。设置 [agent] command。
agent-no-http = 此代理无法连接到 Oku Slides 的工具。
agent-stopped = 代理已停止。
agent-failed = 代理拒绝了请求：{ $message }
agent-tool-failed = 该工具意外失败。
