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
registry-duplicate-pack = プレゼンテーションの2つのウィジェットパックが { $pack } を共有しています。
registry-compile = テンプレートパックのウィジェットをコンパイルできませんでした
registry-missing = テンプレートパックがウィジェットを宣言していません
render-timeout = レンダリングが { $seconds }秒を超えたため中断しました
render-out-of-memory = レンダリングが { $mib } MiB を超えるメモリを必要としたため中断しました
render-worker-lost = レンダリングプロセスが停止しました
render-frame = レンダリングプロセスからのメッセージを読み取れません
render-page-too-large = このスライドはプレビューするには大きすぎます（{ $mib } MiB 超。多くは非常に大きな画像が原因です）。PDF または PNG としては書き出せます。
render-stale = レンダリングは置き換えられました。もう一度お試しください
render-pack-disabled = パック「{ $pack }」のウィジェットはこのセッションでは無効です
render-protocol = レンダリングプロセスは別のバージョンの Oku Slides のものです。アプリケーションを再起動してください
render-remote = レンダリングプロセスは要求を完了できませんでした
registry-schema-remote = ウィジェットレジストリが無効です: { $message }
probe-not-probed = 未確認：パックのウィジェットに割り当てられた時間を使い切りました。次の描画時に確認します。
probe-too-many-files = パックのウィジェットファイルが { $max }個を超えています：このファイルは読み込まれません。
probe-too-large = { $max } KiB を超えています：読み込まれません。

import-unreadable = このプレゼンテーションを読み込めません
import-not-a-presentation = このファイルは PowerPoint プレゼンテーションではありません
import-protected = このプレゼンテーションは暗号化されているか旧形式の .ppt です。先に .pptx として保存してください
import-too-large = このプレゼンテーションは大きすぎてインポートできません
import-output-too-large = 変換後のプレゼンテーションがインポートで許可されるサイズを超えます
import-loss-list-levels = リストの階層を 1 段にまとめました
import-loss-merged-cells = 結合セルを解除しました
import-loss-percent-stacked = 100% 積み上げグラフを積み上げグラフとして表示しました
import-loss-chart-format = グラフの数値書式 { $format } は保持されません
import-loss-run-style = { $effect ->
    [underline] 下線が失われました
    [strike] 取り消し線が失われました
    [baseline] 上付きまたは下付きが失われました
    [caps] 大文字表示が失われました
    [font] 一部のフォントがボックスのフォントに置き換えられました
    [tracking] 一部の文字間隔がボックスの文字間隔に置き換えられました
    [weight] 標準の部分がボックスに合わせて太字で表示されました
    [slant] 斜体でない部分がボックスに合わせて斜体で表示されました
   *[highlight] 蛍光ペンが失われました
}
import-loss-transform = 回転、反転、トリミングが失われました
import-loss-color-dropped = 色 { $color } に近いテーマの色がありません。テーマの色を使用しました
import-loss-text-truncated = 上限に収まるようテキストを短くしました
import-loss-table-truncated = 上限に収まるよう表を短くしました
import-loss-chart-truncated = 上限に収まるようグラフのデータを短くしました
import-loss-opaque = { $reason ->
    [media] 動画または音声はインポートされませんでした (代替画像なし)
    [ole] 埋め込みオブジェクトはインポートされませんでした (代替画像なし)
    [math] 数式はインポートされませんでした (代替画像なし)
    [smartart] SmartArt はインポートされませんでした (代替画像なし)
   *[unknown] 未対応のオブジェクトはインポートされませんでした (代替画像なし)
}
import-loss-smartart-layout = 描画のない SmartArt: テキストをカードとして配置し、図のレイアウトは失われました
import-loss-unsupported-image = { $format } 形式の画像はインポートされませんでした
import-loss-hidden-slide = 非表示のスライドを表示状態でインポートしました
import-loss-effects = 影、反射、3D 効果が失われました
import-loss-series-dropped = 円グラフは 1 系列のみ表示します。{ $kept } 系列を保持、{ $dropped } 系列を除外しました
import-loss-repeated-decor = { $slides }枚のスライドで繰り返されるヘッダーまたはフッター「{ $text }」を省きました。テーマが独自に描画します
import-loss-no-slot = コンテンツを省きました。選んだテンプレートに配置できる場所がありません
import-loss-moved-to-notes = テキストを発表者ノートに移しました。表紙に配置できる場所がありません
import-loss-items-dropped = スライドの要素が多すぎます：{ $dropped }個の要素を省きました
import-loss-page-fitted = 元のページ（{ $width } × { $height } pt）は 16:9 ではありません。内容を拡大縮小して中央に配置しました
import-loss-paragraph-style = 段落の配置、行間、段落間隔、またはインデントがボックスに合わせられました
import-loss-shape-style = { $effect ->
    [gradient] グラデーションが単色で表示されました
    [picture] 塗りつぶしの画像が図形の枠に合わせて引き伸ばされました
    [dash] 破線が実線で表示されました
    [arrow] 線の矢印は描画されません
   *[geometry] 不明な図形が四角形で表示されました
}
import-loss-layout-dropped = 1つのパックにはレイアウトが多すぎます：「{ $layout }」にはテンプレートがなく、そのスライドは空のウィジェットを使い、装飾を要素として受け取ります
import-loss-decor-truncated = パックファイルのサイズに収めるため装飾を切り詰めました：{ $omitted }個の要素を省きました
import-loss-table-style = 表のスタイルを近似しました：独自のセルの背景、罫線、列幅、行の高さ、文字サイズは失われます
import-loss-overflow = 要素「{ $element }」がスライドの端からはみ出しています
import-loss-empty-data = データのない表またはグラフ：何も配置していません
import-reference-unavailable = 見た目の差のスコアなし：{ $cause ->
    [timeout] 参照レンダリングに時間がかかりすぎました
    [memory] 参照レンダリングがメモリを使いすぎました
    [pages] 参照レンダリングは { $reference }ページで、デッキは{ $deck }ページです
    [missing] LibreOffice (soffice) が見つかりません。import.soffice にパスを指定してください
   *[failed] 参照レンダラーがプレゼンテーションを読めませんでした
}
import-region-suspect = 参照レンダラーの既知の誤りとして除外した差：{ $limit ->
    [autofit] PowerPoint が倍率を保存していないと、ボックスに合わせたテキストを縮小します
    [tracking] 文字間隔を広げたテキストを幅広く描きます
    [chart] グラフをおおまかにしか描きません
    [default_font] プレゼンテーションの既定値だけがフォントを指定するテキストを代替フォントで描きます
    [no_substitute] このフォントの代替がなく、別のフォントで描きます
    [smart_art_style] SmartArt のテキストを独自のスタイルで描きます
   *[smart_art] SmartArt のテキストを小さく描きすぎます
}
import-font-substituted = { $family } をメトリクス互換の { $by } に置き換えました
import-font-missing = { $family } が見つかりません。テキストは { $by } を使います
import-font-restricted = { $family }：ライセンスにより一部のスタイルをプレゼンテーションに埋め込めません。そのスタイルのテキストは他のスタイルを使います
import-font-restricted-substituted = { $family } はライセンスによりプレゼンテーションに埋め込めません。メトリクス互換の { $by } に置き換えました
import-font-restricted-missing = { $family } はライセンスによりプレゼンテーションに埋め込めません。テキストは { $by } を使います
import-font-too-large = { $family }：ファイルが大きすぎてデッキに埋め込めません。テキストは { $by } を使います
import-font-missing-no-fallback = { $family } もテーマの本文フォントも見つかりません：テキストはエンジンの既定フォントで表示されます
import-font-too-large-no-fallback = { $family }：ファイルが大きすぎてデッキに埋め込めず、テーマの本文フォントもありません：テキストはエンジンの既定フォントで表示されます
import-font-restricted-no-fallback = { $family } はライセンスによりプレゼンテーションに埋め込めず、テーマの本文フォントもありません：テキストはエンジンの既定フォントで表示されます
import-warning-effect-ignored = 効果はインポートされませんでした: { $detail }
import-warning-approximated = 近似して表示しました: { $detail }
import-warning-unreadable-item = 読み取れない項目をスキップしました: { $detail }
import-warning-unreadable-part = 読み取れないパーツ: { $detail }
import-warning-missing-part = パーツがありません: { $detail }
import-warning-unsupported-chart = 未対応のグラフ: { $detail }
import-warning-unsupported-image = 未対応の画像形式: { $detail }
import-warning-oversized-image = 画像が大きすぎてインポートできません（ピクセル数）: { $detail }
import-warning-external-image = ファイル外にリンクされた画像はインポートされません: { $detail }
import-warning-unsafe-svg = SVG 画像から安全でない内容を除去したか、画像を破棄しました: { $detail }
import-warning-unknown-table-style = 不明な表のスタイル: { $detail }
import-warning-unsafe-link = リンクを拒否しました: { $detail }
import-warning-nesting-too-deep = 入れ子が深すぎるため切り詰めました: { $detail }
import-warning-font-embedded-unreadable = 埋め込みフォントを読み取れません: { $detail }
import-warning-truncated = 警告が多すぎます。以降の警告は省略されます
