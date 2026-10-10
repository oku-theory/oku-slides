pdf-unreadable = PDF を読み込めません
pdf-unsupported-version = この PDF は新しいバージョンの Oku Slides で書き込まれています
pdf-corrupted = この PDF のソースが破損しています
pdf-write = PDF を書き込めませんでした

render-compile = プレゼンテーションをレンダリングできませんでした
render-pdf = PDF の書き出しに失敗しました
render-no-page = このページは存在しません
render-setup = レンダラーを起動できませんでした
render-image = スライドの画像を生成できませんでした
render-missing-image = 画像がありません
diagnostic-error = エラー
diagnostic-warning = 警告
diagnostic = { $level }（{ $place }）：{ $message }
diagnostic-unplaced = { $level }：{ $message }
diagnostic-hint = ヒント：{ $hint }
render-chart-failed = グラフを描画できませんでした

registry-bad-pack-id = テンプレートパックの識別子が無効です: { $id }
registry-compile = テンプレートパックのウィジェットをコンパイルできませんでした
registry-missing = テンプレートパックがウィジェットを宣言していません
render-timeout = レンダリングが { $seconds }秒を超えたため中断しました
render-out-of-memory = レンダリングが { $mib } MiB を超えるメモリを必要としたため中断しました
render-worker-lost = レンダリングプロセスが停止しました
render-frame = レンダリングプロセスからのメッセージを読み取れません
render-page-too-large = このスライドはプレビューするには大きすぎます（{ $mib } MiB 超。多くは非常に大きな画像が原因です）。PDF または PNG としては書き出せます。
render-stale = レンダリングは置き換えられました。もう一度お試しください
render-pack-disabled = パック「{ $pack }」のウィジェットはこのセッションでは無効です
registry-schema-remote = ウィジェットレジストリが無効です: { $message }
probe-not-probed = 未確認：パックのウィジェットに割り当てられた時間を使い切りました。次の描画時に確認します。
probe-too-many-files = パックのウィジェットファイルが { $max }個を超えています：このファイルは読み込まれません。
probe-too-large = { $max } KiB を超えています：読み込まれません。

import-unreadable = このプレゼンテーションを読み込めません
import-not-a-presentation = このファイルは PowerPoint プレゼンテーションではありません
import-protected = このプレゼンテーションは暗号化されているか旧形式の .ppt です。先に .pptx として保存してください
import-too-large = このプレゼンテーションは大きすぎてインポートできません
import-loss-list-levels = リストの階層を 1 段にまとめました
import-loss-merged-cells = 結合セルを解除しました
import-loss-percent-stacked = 100% 積み上げグラフを積み上げグラフとして表示しました
import-loss-run-style = { $effect ->
    [underline] 下線が失われました
    [strike] 取り消し線が失われました
    [baseline] 上付きまたは下付きが失われました
    [caps] 大文字表示が失われました
   *[highlight] 蛍光ペンが失われました
}
import-loss-transform = 回転、反転、トリミングが失われました
import-loss-color-dropped = 色 { $color } に近いテーマの色がありません。テーマの色を使用しました
import-loss-text-truncated = 上限に収まるようテキストを短くしました
import-loss-table-truncated = 上限に収まるよう表を短くしました
import-loss-opaque = { $reason ->
    [media] 動画または音声はインポートされませんでした (代替画像なし)
    [ole] 埋め込みオブジェクトはインポートされませんでした (代替画像なし)
    [math] 数式はインポートされませんでした (代替画像なし)
    [smartart] SmartArt はインポートされませんでした (代替画像なし)
   *[unknown] 未対応のオブジェクトはインポートされませんでした (代替画像なし)
}
import-loss-unsupported-image = { $format } 形式の画像はインポートされませんでした
import-loss-hidden-slide = 非表示のスライドを表示状態でインポートしました
import-loss-effects = 影、反射、3D 効果が失われました
import-loss-series-dropped = 円グラフは 1 系列のみ表示します。{ $kept } 系列を保持、{ $dropped } 系列を除外しました
import-loss-repeated-decor = { $slides }枚のスライドで繰り返されるヘッダーまたはフッター「{ $text }」を省きました。テーマが独自に描画します
import-loss-no-slot = コンテンツを省きました。選んだテンプレートに配置できる場所がありません
import-warning-effect-ignored = 効果はインポートされませんでした: { $detail }
import-warning-approximated = 近似して表示しました: { $detail }
import-warning-unreadable-item = 読み取れない項目をスキップしました: { $detail }
import-warning-unreadable-part = 読み取れないパーツ: { $detail }
import-warning-missing-part = パーツがありません: { $detail }
import-warning-unsupported-chart = 未対応のグラフ: { $detail }
import-warning-unsupported-image = 未対応の画像形式: { $detail }
import-warning-unknown-table-style = 不明な表のスタイル: { $detail }
import-warning-unsafe-link = リンクを拒否しました: { $detail }
import-warning-nesting-too-deep = 入れ子が深すぎるため切り詰めました: { $detail }
import-warning-font-embedded-unreadable = 埋め込みフォントを読み取れません: { $detail }
import-warning-truncated = 警告が多すぎます。以降の警告は省略されます
