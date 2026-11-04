# エージェント：サーフェス、確認、スレッド。

agent-surface-cancel = キャンセル
agent-surface-confirm = 確認
agent-surface-confirm-danger = はい、実行します
agent-surface-heard = 聞き取り：「{ $text }」
agent-surface-say-or-click = 答えを言うか、ボタンをクリックしてください。
agent-surface-said-too-early = 早すぎました：ウィンドウが表示されてから、もう一度言ってください。
agent-surface-click-to-confirm = 確定するにはボタンをクリックしてください。
agent-surface-verb-replace = 置き換え
agent-surface-verb-merge = 統合
agent-surface-verb-discard = 破棄
agent-surface-verb-delete = 削除
agent-surface-verb-use-api = api を使う
agent-surface-verb-install = インストール
agent-surface-verb-quit = アプリを終了
agent-surface-verb-close = 閉じる
agent-surface-verb-import = インポート
agent-surface-rewrite = 言い換え
agent-surface-pick = { $n } を選ぶ
agent-surface-keep = 元の文章を保持
agent-surface-edit = 適用前に編集
agent-surface-close = 閉じる
agent-surface-themes = テーマ
agent-surface-variants = バリエーション
agent-surface-permission-always = 常に
agent-surface-permission-title = エージェントが許可を求めています
agent-surface-permission-guides = このターンには、{ $names } の受信したガイドが含まれています。
agent-undo-turn = エージェント：{ $text }

agent-tools-unavailable = エージェントのツールを起動できませんでした。

agent-undo-apply = エージェントの変更
agent-undo-variant = バリエーション：{ $label }
agent-undo-param = 設定：{ $label }
agent-undo-rewrite = 言い換え

agent-confirm-save = プレゼンテーションを{ $path }として保存しますか？
agent-confirm-save-over = { $path }を置き換えますか？既存のファイルは上書きされます。
agent-confirm-open = { $path }を開きますか？
agent-open-elsewhere = このファイルは別のタブですでに開かれています。
agent-confirm-new = 新しいプレゼンテーションを開始しますか？
agent-confirm-new-dirty = 新しいプレゼンテーションを開始しますか？保存されていない変更は失われます。
agent-confirm-copy-pack = パック「{ $name }」のコピーを保存しますか？パック自体は変更されません。
agent-confirm-copy-edited-theme = テーマ「{ $name }」のコピーを保存しますか？テーマ自体は変更されません。
agent-confirm-delete-pack = パック「{ $name }」を削除しますか？元に戻せません。
agent-confirm-edit-theme = テーマ「{ $name }」の新しいバージョンを保存しますか？
agent-confirm-copy-theme = 組み込みテーマ「{ $name }」のバリエーションを作成しますか？組み込みテーマは変更されません。
agent-confirm-setting = { $key }を変更しますか：{ $old } → { $new }
agent-confirm-move = プレゼンテーションを { $path } に移動しますか？元のファイルは削除されます。
agent-confirm-merge = { $path } をこのプレゼンテーションに統合しますか？元に戻せません。
agent-confirm-save-copy = プレゼンテーションのコピーを { $path } として保存しますか？
agent-confirm-dictation-api = 音声入力の内容を { $host }（{ $url }）、モデル { $model }、{ $var } のシークレットで送信しますか？
agent-confirm-dictation-download = { $name } を使い、今すぐダウンロードしますか（{ $size }）？
agent-confirm-dictation-language = Appleの音声入力を使い、その言語を今すぐダウンロードしますか？
agent-confirm-agent = 返信が実行されていない時点で、アプリのエージェントを { $name }（{ $command }）に切り替えますか？
agent-confirm-agent-builtin = 返信が実行されていない時点で、アプリのエージェントを { $name } に切り替えますか？
agent-confirm-agent-model = ローカルエージェントのモデルをダウンロードしますか（{ $size }）？

agent-not-found = エージェントが見つかりません：{ $command } はそのパスにもアプリの PATH にもありません。[agent.presets] にフルパスを指定してください。
agent-spawn-failed = エージェントを起動できません：{ $command }。設定 [agent.presets]。
agent-no-http = このエージェントはOku Slidesのツールに接続できません。
agent-local-no-program = ローカルのエージェントはOku Slidesアプリの中でしか動作しません。
agent-local-needs-download = ローカルエージェントは、先にモデル({ $gigabytes }GB)をダウンロードする必要があります。
agent-local-intel-mac = ローカルエージェントには、Appleシリコン搭載のMacが必要です。
agent-local-unsupported-os = ローカルエージェントは、このシステムではまだ利用できません。
agent-local-apple-off = ローカルエージェントには Apple Intelligence が必要です。システム設定でオンにしてください。
agent-local-apple-not-ready = Apple のモデルはまだ準備中です。数分後にローカルエージェントをもう一度お試しください。
agent-local-apple-unsupported = Apple のモデルには、Apple Intelligence に対応し macOS 26 以降を搭載した Mac が必要です。
agent-model-download = モデルをダウンロード
agent-model-downloading = モデルをダウンロード中… { $percent }%
agent-model-cancel = キャンセル
agent-model-download-failed = モデルをダウンロードできませんでした。
agent-model-download-integrity = ダウンロードしたモデルが破損していたため、削除しました。
agent-model-download-incomplete = ダウンロードが不完全です：{ $expected }バイト中{ $got }バイト。
agent-model-download-cancelled = ダウンロードをキャンセルしました。
agent-fallback-notice = ローカルエージェント — { $reason }
agent-fallback-download = { $reason } モデル({ $model }、{ $gigabytes }GB)をダウンロードすると、ローカルエージェントが代わりに動作します。今すぐダウンロードしますか？
agent-fallback-unusable = 動作できるエージェントがありません。メインのエージェント：{ $main } ローカルエージェント：{ $local }
local-agent-no-route = ローカルのエージェントは何をすればよいか理解できませんでした。別の言い方で試してください。
local-agent-tool-limit = ローカルのエージェントは{ $n }ステップで完了せずに停止しました。
local-agent-invalid-call = ローカルのエージェントはツールを正しく使えませんでした。
local-agent-engine-failed = ローカルのエージェントのモデルが失敗しました。
local-agent-retry = もう一度お試しいただくか、エージェントのパネルで別のエージェントを選んでください。
agent-stopped = エージェントが停止しました。
agent-failed = エージェントが要求を拒否しました: { $message }
agent-error-work-dir = このタブのエージェント専用の作業フォルダーを準備できませんでした。
agent-tool-failed = ツールが予期せず失敗しました。

agent-turn-tools = { $tools }個のツール
agent-turn-notes = { $notes }件のメモ
agent-turn-secs = { $secs }秒
agent-turn-stopped = 停止しました。
agent-turn-interrupted = 中断されました。
agent-memory-lost = エージェントはこの会話を覚えていません。最初からやり直します。
agent-turn-max-tokens = エージェントが長さの上限に達しました。
agent-turn-max-turns = エージェントがステップ数の上限に達しました。
agent-turn-refusal = エージェントが拒否しました。
agent-turn-ended = ターン終了（{ $stop }）。
agent-choice-cancelled = 質問はキャンセルされました
agent-choice-answered = ✓ { $answer }
agent-report-why = 理由
agent-trace = { $name }：{ $count ->
   *[other] { $count }件の変更
}
agent-trace-data = { $name }：データを変更
agent-trace-removed = { $name }：存在しません
agent-trace-more = { $count ->
   *[other] ほか{ $count }件
}
agent-trace-title = 変更された要素

agent-error-file = ファイルを読み書きできませんでした。
agent-error-unexpected-action = 予期しない操作：{ $action }。
agent-error-unknown-surface = このカードはもう開いていません。
agent-error-expired = 過去のターンの操作は期限切れです。
agent-error-incomplete = このカードの操作は不完全です。
agent-error-no-field = この設定はもう存在しません。
agent-error-no-variant = このバリエーションはもう存在しません。
agent-error-arguments = ツールの引数が無効です（{ $detail }）。スキーマに沿って確認してください。
agent-error-unknown-tool = 不明なツール：{ $name }。
agent-error-pack-id = これはパックの id ではありません。packs を呼び出して、パックとその id を一覧してください。
agent-error-asset-unknown = この画像はプレゼンテーションにも貼り付けた画像にもありません。貼り付けた画像の ID を指定してください。
agent-error-denied = ユーザーが拒否しました。
agent-error-no-turn = 進行中のターンがありません。{ $tool } はターン中にのみ表示されます。
agent-error-tab-no-turn = タブを開く・表示する・移動するのは、ユーザーに応答している間だけにしてください。
agent-error-tab-limit = このターンではすでにタブを開いた・表示した・移動しました。そちらで終えるか、別のタブの前にユーザーに確認してください。
agent-error-continue-in-other-tab = この依頼は、開いたか表示したタブで続きます。ユーザーにそのタブで続けるよう伝えてください。このタブは当面ウィジェットを変更しません。
agent-error-pack-open-in-tab = このパックは別のタブで編集中です。先にそこで終えるようユーザーに伝えてください。
agent-pack-opened-in-tab = { $shown ->
   [yes] パックはタブ { $tab } で開き、現在表示されています。
  *[no] パックはタブ { $tab } で背後に開きました。ユーザーのウィンドウには別のタブが表示されています。
} { $draft ->
   [set] 依頼はそのタブの下書きで待機しています。ユーザーにそこで内容を確認してから送信するよう伝えてください。
   [kept] そのタブの下書きにはすでに内容があったため、そのままにしました。ユーザーにそこで続けるよう伝えてください。そのタブのエージェントはこの依頼を知りません。
   [too_long] 依頼がそのタブの下書きには長すぎました。ユーザーにそこで続けるよう伝えてください。そのタブのエージェントはこの依頼を知りません。
   [unavailable] 依頼をそのタブの下書きに渡せませんでした。ユーザーにそこで続けるよう伝えてください。そのタブのエージェントはこの依頼を知りません。
  *[none] ユーザーにそこで続けるよう伝えてください。そのタブのエージェントはこの依頼を知りません。
} このタブはそのままです。
agent-theme-opened-in-tab = { $shown ->
   [yes] テーマはタブ { $tab } で開き、現在表示されています。
  *[no] テーマはタブ { $tab } で背後に開きました。ユーザーのウィンドウには別のタブが表示されています。
} { $draft ->
   [set] 依頼はそのタブの下書きで待機しています。ユーザーにそこで内容を確認してから送信するよう伝えてください。
   [kept] そのタブの下書きにはすでに内容があったため、そのままにしました。ユーザーにそこで続けるよう伝えてください。そのタブのエージェントはこの依頼を知りません。
   [too_long] 依頼がそのタブの下書きには長すぎました。ユーザーにそこで続けるよう伝えてください。そのタブのエージェントはこの依頼を知りません。
   [unavailable] 依頼をそのタブの下書きに渡せませんでした。ユーザーにそこで続けるよう伝えてください。そのタブのエージェントはこの依頼を知りません。
  *[none] ユーザーにそこで続けるよう伝えてください。そのタブのエージェントはこの依頼を知りません。
} このタブはそのままです。
agent-error-turn-over = この呼び出しのターンは終了しました。何も表示されていません。
agent-error-no-box-turn = このボックスを関連付けるターンがありません：何も表示されませんでした。
agent-error-no-file = プレゼンテーションにはまだファイルがありません。絶対パスを指定してください。
agent-error-path-relative = パスは絶対パスである必要があります。
agent-error-path-parent = パスに「..」を含めることはできません。
agent-error-setting-locked = エージェントは設定 { $key } を変更できません。
agent-error-setting = 設定 { $key } を変更できませんでした。値を確認してください。
agent-error-history-index = 履歴をたどるにはバージョンの番号が必要です。
agent-error-no-pack = このプレゼンテーションはパックを固定していません。pack を指定してください（packs を参照）。
agent-error-history-not-opened = タブが時間内に履歴を開きませんでした。もう一度依頼してください。
agent-error-locale = 不明な言語コードです。同梱の言語コードか system を指定してください。
agent-error-empty-interface = 変更する内容がありません。scheme、locale、guides、folded、pile、notes、code のいずれかを指定してください。
agent-error-layout-range = { $field } は { $min }～{ $max } px にしてください。
agent-error-layout-min = { $field } は { $min } px 以上にしてください。
agent-error-empty-dictation = 変更する内容がありません。音声入力の設定を1つ以上指定してください。
agent-error-empty-speech = 変更するものがありません。読み上げの設定を少なくとも1つ指定してください。
agent-error-not-shown = このタブはウィンドウに表示中のタブではありません。ユーザーが表示してからもう一度依頼してください。
agent-error-not-presenting = このデッキはここでプレゼンテーション中ではありません。まず start アクションで開始してください。
agent-error-present-slide = スライドを表示するには、このデッキのスライドの id が必要です（deck_outline を参照）。
agent-error-undo-open-turn = undo_turn は実行中のターンを取り消しません。このターン自身の変更には undo を使ってください。
agent-error-app-unavailable = これは Oku Slides アプリだけが行えます。ここでは利用できません。
agent-error-app-closing = Oku Slides はすでに終了しようとしています。
agent-error-app-no-offer = ここには後回しにしたりインストールしたりできるアップデートがありません。まずアップデートを確認してください。
agent-error-app-failed = Oku Slides のアップデートに失敗しました。接続を確認するか、アップデートのバナーを使ってください。
agent-error-after-turn-taken = この応答はすでに、アプリの終了、アップデートのインストール、または自分のタブを閉じることで終わることになっています。
agent-error-no-pack-update = パックパネルはこのパックにそのような更新を提示していません。show_packs を参照してください。
agent-confirm-update-install = Oku Slides { $version } をインストールしますか？この応答が終わるとアプリが再起動します。
agent-after-turn-quit-unsaved = Oku Slides は終了しませんでした。依頼のあと、タブに未保存の変更ができています。
agent-error-workspace-unavailable = タブとウィンドウを変更できるのは Oku Slides アプリだけです。ここでは利用できません。
agent-error-workspace-unknown-tab = このウィンドウにはそのタブまたはウィンドウがありません。
agent-error-workspace-unknown-window = そのウィンドウはありません。
agent-error-workspace-index = その位置はタブバーの末尾より後ろです。
agent-error-workspace-closing = Oku Slides は終了中です。タブとウィンドウはもう変わりません。
agent-confirm-close-tab = タブ「{ $tab }」を閉じますか？
agent-confirm-discard-tab = タブ「{ $tab }」を閉じますか？保存されていない変更は失われます。
agent-confirm-close-window = このウインドウとそのタブを閉じますか？
agent-confirm-discard-window = このウインドウを閉じますか？{ $tabs } の保存されていない変更は失われます。
agent-error-window-last = これは唯一のウインドウです。代わりにタブを閉じるか、Oku Slides を終了してください。
agent-error-tab-index = タブを置く場所を指定してください。並べ替えには位置が必要です。
agent-error-tab-unused = 位置を取るのは並べ替えと移動だけで、ウインドウを取るのは移動だけです。
agent-after-turn-close-unsaved = タブは閉じられませんでした。依頼のあとで変更されています。
agent-after-turn-close-not-shown = タブは閉じられませんでした。応答の終了時、そのウインドウには別のタブが表示されていました。
agent-confirm-quit = Oku Slides を終了しますか？この応答が終わるとアプリは終了します。
agent-error-quit-unsaved = 保存されていない作業があるタブ：{ $count }。Oku Slides は終了していません。先にユーザーが保存するか閉じてください。
agent-error-tune-unavailable = 微調整を開始できませんでした。エージェントが利用できません。
agent-error-tune-not-started = 微調整は始まりませんでした。そのタブのエージェントは今は使えません。理由はそのタブに表示されます。
agent-confirm-import = { $path }を新しいタブにインポートしますか？
agent-import-opened-in-tab = { $shown ->
   [yes] プレゼンテーションはタブ { $tab } で開き、いま表示されています。
  *[no] プレゼンテーションはタブ { $tab } で背面に開きました。ユーザーのウィンドウは別のタブを表示しています。
} { $tune ->
   [started] その微調整はそのタブ自身の会話で始まります。
   [unavailable] その微調整を開始できませんでした。そのタブで依頼するようユーザーに伝えてください。
  *[no] そちらで続けるようユーザーに伝えてください。
} このタブはそのままです。
agent-error-import-hifi-target = ハイファイのインポートはパックもテーマも受け付けません。外すか、retrofit モードでインポートしてください。
agent-error-import-extension = インポートできるのは PowerPoint の .pptx ファイルだけです。
agent-error-a2ui-unknown = { $name } は render が受け付けるコンポーネントではありません。コンポーネント: { $known }。
agent-error-a2ui-elsewhere = render は { $name } を表示しません: { $tool } を使ってください。
agent-error-a2ui-host-only = { $name } はアプリ専用です: render では表示できません。
agent-error-themes-deck-mode = テーマ選択はスライドの編集中にのみ開きます。
agent-error-packs-deck-mode = プレゼンテーションのパックパネルはスライドの編集中にのみ開きます。
agent-error-usage-no-deck = このタブの使用量はどのプレゼンテーションにも属しません。プレゼンテーションに計上されるのは、そのプレゼンテーションから（ユーザーまたはエージェントが）開いたパックまたはテーマのタブか、プレゼンテーション自身のパックだけです。
agent-error-rewrite-text = 言い換えがテキストなしで返されました。
agent-error-paragraphs = 言い換えの段落が無効です。
agent-error-draft-too-long = 下書きが長すぎます。
agent-error-report-elsewhere = このウィンドウにはそのタブがありません。
agent-error-ui-too-large = ページのコンテキストが大きすぎます。
agent-error-seq-exhausted = ページの報告番号が範囲外です。
agent-error-draft-origin = 下書きを拒否しました：不明なビューです。
agent-error-draft-no-agent = このタブのエージェントはまだ起動していません：下書きを拒否しました。
agent-error-too-many-images = 下書きにはすでに画像がいっぱいです。
agent-error-image-not-base64 = 貼り付けた画像が最後まで届きませんでした。
agent-error-image-gone = 貼り付けた画像が使えなくなりました。もう一度貼り付けてください。
agent-error-unknown-agent = このエージェントは設定にもうありません。
agent-error-agent-locked = エージェントの選択は環境によって固定されています。
agent-error-agent-not-saved = エージェントの選択を保存できませんでした。
agent-error-unknown-model = このエージェントはそのモデルを提供していません。
agent-error-model-not-saved = モデルの選択を保存できませんでした。
voice-error-no-dictation = ここでは音声入力を使えません
voice-error-busy = 音声入力の実行中です。停止してからもう一度お試しください
voice-error-unknown-model = 不明な音声入力モデルです
voice-error-unavailable = このモデルは選択した言語に対応していません
voice-error-installed = このモデルはすでにインストールされています
voice-error-not-downloadable = このモデルはダウンロードしません
voice-error-nothing-to-cancel = 実行中のダウンロードはありません
voice-error-bad-silence = 無音は 0.5〜10秒にしてください
voice-error-bad-language = 不明な音声入力の言語です
voice-error-bad-url = API のアドレスは https:// で始めてください（http:// はこのコンピュータのみ）。ユーザー名、パラメータ、特殊文字は使えません
voice-error-bad-key-env = キーの変数は OPENAI_API_KEY のような名前にしてください：大文字、数字、_
voice-error-bad-api-model = API モデル名に使えるのは英字、数字と . _ : / - だけです。Azure のデプロイ名は英字、数字と . _ - だけです
voice-error-bad-api-version = Azure API バージョンに使えるのは英字、数字、. と - だけで、最大32文字です (例: 2025-03-01-preview)
voice-error-bad-api-headers = ヘッダーは Name: value; Name: $VARIABLE の形式で、最大8個まで書きます。Authorization, api-key, Host, Content-Type, Content-Length, Transfer-Encoding, Connection と重複は拒否されます
voice-error-api-reserved = API のアドレス、キーの変数、Azure バージョン、ヘッダーを設定できるのはユーザーだけです: show_dictation でボックスを開いて依頼してください
voice-error-hands-free-reserved = ハンズフリー聞き取りをオンにできるのはユーザーだけです。show_dictation でボックスを開き、ユーザーに頼んでください
voice-error-changed = 質問のあとで設定が変わりました。もう一度確認してください
voice-error-nothing-to-download = このエージェントにダウンロードするものはありません
voice-error-bad-rate = 話す速さは 0.5 から 2 の間にしてください
voice-error-unknown-voice = インターフェースの言語では不明な声です
voice-error-speech-unavailable = このコンピュータでは読み上げを利用できません
agent-tune-prompt = インポートしたプレゼンテーションを微調整する
agent-settings-unknown-selected = 設定 agent.selected = { $selected } は [agent.presets] のどのエージェントも指していません（既知: { $known }）
agent-settings-preset-launch = エージェント [agent.presets.{ $id }] には command と builtin のどちらか一方だけが必要です
