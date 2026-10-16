pdf-unreadable = PDF 无法读取
pdf-unsupported-version = 此 PDF 由更新版本的 Oku Slides 写入
pdf-corrupted = 此 PDF 中的源文件已损坏
pdf-write = 无法写入 PDF

render-compile = 无法渲染演示文稿
render-pdf = PDF 导出失败
render-no-page = 此页面不存在
render-image = 无法生成幻灯片的图像

render-setup = 渲染器无法启动
render-missing-image = 缺少图片
diagnostic-error = 错误
diagnostic-warning = 警告
diagnostic = { $level }（{ $place }）：{ $message }
diagnostic-unplaced = { $level }：{ $message }
diagnostic-hint = 提示：{ $hint }
render-chart-failed = 无法绘制图表

registry-bad-pack-id = 模板包标识符无效: { $id }
registry-compile = 无法编译模板包的组件
registry-missing = 模板包未声明其组件
render-timeout = 渲染耗时超过 { $seconds }秒，已中断
render-out-of-memory = 渲染所需内存超过 { $mib } MiB，已中断
render-worker-lost = 渲染进程已停止
render-frame = 渲染进程发来的消息无法读取
render-page-too-large = 此幻灯片过大，无法预览（超过 { $mib } MiB，通常是因为图片过大）。仍可导出为 PDF 或 PNG。
render-stale = 渲染已被替换，请重试
render-pack-disabled = “{ $pack }”包的小组件在本次会话中已停用
render-protocol = 渲染进程属于另一个版本的 Oku Slides，请重新启动应用程序
render-remote = 渲染进程无法完成该请求
registry-schema-remote = 小组件注册表无效：{ $message }
probe-not-probed = 尚未检查：分配给该包小组件的时间已用完，将在下次渲染时检查。
probe-too-many-files = 包中的小组件文件超过 { $max }个：此文件未加载。
probe-too-large = 超过 { $max } KiB：未加载。

import-unreadable = 无法读取此演示文稿
import-not-a-presentation = 此文件不是 PowerPoint 演示文稿
import-protected = 此演示文稿已加密或为旧版 .ppt 格式：请先另存为 .pptx
import-too-large = 此演示文稿太大，无法导入
import-output-too-large = 导入后的演示文稿将超过导入允许的大小
import-loss-list-levels = 列表级别已合并为一级
import-loss-merged-cells = 已拆分合并的单元格
import-loss-percent-stacked = 百分比堆积图已显示为堆积图
import-loss-chart-format = 未保留图表的数字格式 { $format }
import-loss-run-style = { $effect ->
    [underline] 下划线已丢失
    [strike] 删除线已丢失
    [baseline] 上标或下标已丢失
    [caps] 大写已丢失
    [font] 部分文字的字体已替换为文本框的字体
    [tracking] 部分文字的字距已替换为文本框的字距
    [weight] 常规文字段按文本框显示为粗体
    [slant] 正体文字段按文本框显示为斜体
   *[highlight] 突出显示已丢失
}
import-loss-transform = 旋转、翻转或裁剪已丢失
import-loss-color-dropped = 颜色 { $color } 在主题中没有相近的颜色：已使用主题颜色
import-loss-text-truncated = 文本已缩短以符合限制
import-loss-table-truncated = 表格已缩短以符合限制
import-loss-opaque = { $reason ->
    [media] 视频或声音未导入（无替代图像）
    [ole] 嵌入对象未导入（无替代图像）
    [math] 公式未导入（无替代图像）
    [smartart] SmartArt 未导入（无替代图像）
   *[unknown] 不支持的对象未导入（无替代图像）
}
import-loss-smartart-layout = 没有绘图的 SmartArt：其文本以卡片排列，图示布局已丢失
import-loss-unsupported-image = { $format } 格式的图像未导入
import-loss-hidden-slide = 隐藏的幻灯片已作为可见幻灯片导入
import-loss-effects = 阴影、映像和三维效果已丢失
import-loss-series-dropped = 饼图只显示一个系列：保留 { $kept }个，舍弃 { $dropped }个
import-loss-repeated-decor = 已省略在 { $slides }张幻灯片上重复的页眉或页脚“{ $text }”：主题会绘制自己的页眉页脚
import-loss-no-slot = 已省略内容：所选模板中没有可放置它的位置
import-loss-page-fitted = 源页面（{ $width } × { $height } pt）不是 16:9：内容已缩放并居中
import-loss-paragraph-style = 段落的对齐方式、行距或段间距已改为文本框的设置
import-loss-shape-style = { $effect ->
    [gradient] 渐变已显示为纯色
    [picture] 填充图片已拉伸至形状的边框
    [dash] 虚线已显示为实线
    [arrow] 未绘制线条箭头
   *[geometry] 未知形状已显示为矩形
}
import-loss-layout-dropped = 一个包中的版式过多：“{ $layout }”没有模板，其幻灯片使用空白组件，并以元素形式获得其装饰
import-loss-decor-truncated = 为适应包文件大小而截断了装饰：省略了{ $omitted }个元素
import-loss-table-style = 表格样式为近似：其自身的单元格背景、边框、列宽或文字大小已丢失
import-loss-overflow = 元素“{ $element }”超出了幻灯片边缘
import-loss-empty-data = 没有数据的表格或图表：未放置任何内容
import-reference-unavailable = 无视觉评分：{ $cause ->
    [timeout] 参考渲染耗时过长
    [memory] 参考渲染占用内存过多
    [pages] 参考渲染有{ $reference }页，而演示文稿有{ $deck }页
    [missing] 未找到 LibreOffice (soffice)；请在 import.soffice 中设置其路径
   *[failed] 参考渲染器无法读取该演示文稿
}
import-font-substituted = { $family } 已替换为度量兼容的 { $by }
import-font-missing = 找不到 { $family }：文本使用 { $by }
import-font-restricted = { $family }：其许可证禁止将部分字形嵌入演示文稿；这些样式的文本使用其其他字形
import-font-restricted-substituted = { $family } 不能嵌入演示文稿（许可证）：已替换为度量兼容的 { $by }
import-font-restricted-missing = { $family } 不能嵌入演示文稿（许可证）：文本使用 { $by }
import-font-too-large = { $family }：文件太大，无法嵌入演示文稿；文本使用 { $by }
import-font-missing-no-fallback = 找不到 { $family }，也找不到主题的正文字体：文本以引擎的默认字体显示
import-font-too-large-no-fallback = { $family }：文件太大，无法嵌入演示文稿，主题的正文字体也不可用：文本以引擎的默认字体显示
import-font-restricted-no-fallback = { $family } 不能嵌入演示文稿（许可证），主题的正文字体也不可用：文本以引擎的默认字体显示
import-warning-effect-ignored = 效果未导入：{ $detail }
import-warning-approximated = 近似呈现：{ $detail }
import-warning-unreadable-item = 已跳过无法读取的项目：{ $detail }
import-warning-unreadable-part = 无法读取的部件：{ $detail }
import-warning-missing-part = 缺少部件：{ $detail }
import-warning-unsupported-chart = 不支持的图表：{ $detail }
import-warning-unsupported-image = 不支持的图像格式：{ $detail }
import-warning-oversized-image = 图像过大，无法导入（像素数）：{ $detail }
import-warning-external-image = 文件外链接的图像，未导入：{ $detail }
import-warning-unsafe-svg = SVG 图像已清除不安全内容，或已丢弃：{ $detail }
import-warning-unknown-table-style = 未知的表格样式：{ $detail }
import-warning-unsafe-link = 链接被拒绝：{ $detail }
import-warning-nesting-too-deep = 嵌套过深，已截断：{ $detail }
import-warning-font-embedded-unreadable = 无法读取嵌入的字体：{ $detail }
import-warning-truncated = 警告过多：其余警告已省略
