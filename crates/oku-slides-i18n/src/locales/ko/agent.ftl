# 에이전트: 서피스, 확인, 스레드.

agent-surface-cancel = 취소
agent-surface-confirm = 확인
agent-surface-confirm-danger = 예, 실행합니다
agent-surface-rewrite = 다시 쓰기
agent-surface-pick = { $n } 선택
agent-surface-keep = 원본 유지
agent-surface-edit = 적용 전에 수정
agent-surface-close = 닫기
agent-surface-themes = 테마
agent-surface-variants = 변형
agent-surface-permission-always = 항상
agent-surface-permission-title = 에이전트가 권한을 요청합니다
agent-undo-turn = 에이전트: { $text }

agent-tools-unavailable = 에이전트의 도구를 시작할 수 없습니다.

agent-undo-apply = 에이전트 변경
agent-undo-variant = 변형: { $label }
agent-undo-param = 설정: { $label }
agent-undo-rewrite = 다시 쓰기

agent-confirm-save = 프레젠테이션을 { $path }(으)로 저장할까요?
agent-confirm-save-over = { $path }을(를) 바꿀까요? 기존 파일을 덮어씁니다.
agent-confirm-open = { $path }을(를) 열까요?
agent-open-elsewhere = 이 파일은 이미 다른 탭에서 열려 있습니다.
agent-confirm-new = 새 프레젠테이션을 시작할까요?
agent-confirm-new-dirty = 새 프레젠테이션을 시작할까요? 저장하지 않은 변경 사항이 사라집니다.
agent-confirm-copy-pack = 팩 “{ $name }”의 사본을 저장할까요? 팩 자체는 그대로 유지됩니다.
agent-confirm-copy-edited-theme = 테마 “{ $name }”의 사본을 저장할까요? 테마 자체는 그대로 유지됩니다.
agent-confirm-delete-pack = 팩 “{ $name }”을(를) 삭제할까요? 되돌릴 수 없습니다.
agent-confirm-edit-theme = 테마 “{ $name }”의 새 버전을 저장할까요?
agent-confirm-copy-theme = 기본 제공 테마 “{ $name }”의 변형을 만들까요? 기본 제공 테마는 바뀌지 않습니다.
agent-confirm-setting = { $key } 변경: { $old } → { $new }?
agent-confirm-move = 프레젠테이션을 { $path }(으)로 옮길까요? 이전 파일은 삭제됩니다.
agent-confirm-merge = { $path }을(를) 이 프레젠테이션에 병합할까요? 되돌릴 수 없습니다.
agent-confirm-save-copy = 프레젠테이션 사본을 { $path }(으)로 저장할까요?
agent-confirm-dictation-api = 받아쓴 내용을 { $host }({ $url }), 모델 { $model }, { $var }의 비밀과 함께 보낼까요?
agent-confirm-dictation-download = { $name }을(를) 사용하고 지금 다운로드할까요({ $size })?
agent-confirm-dictation-language = Apple 받아쓰기를 사용하고 그 언어를 지금 다운로드할까요?
agent-confirm-agent = 실행 중인 응답이 없을 때 앱 에이전트를 { $name }({ $command })(으)로 바꿀까요?
agent-confirm-agent-builtin = 실행 중인 응답이 없을 때 앱 에이전트를 { $name }(으)로 바꿀까요?
agent-confirm-agent-model = 로컬 에이전트 모델을 다운로드할까요({ $size })?

agent-not-found = 에이전트를 찾을 수 없습니다: { $command }. 설정 [agent.presets].
agent-spawn-failed = 에이전트를 시작할 수 없습니다: { $command }. 설정 [agent.presets].
agent-no-http = 이 에이전트는 Oku Slides의 도구에 연결할 수 없습니다.
agent-local-no-program = 로컬 에이전트는 Oku Slides 앱 안에서만 실행됩니다.
agent-local-needs-download = 로컬 에이전트는 먼저 모델({ $gigabytes }GB)을 내려받아야 합니다.
agent-local-intel-mac = 로컬 에이전트에는 Apple 실리콘이 탑재된 Mac이 필요합니다.
agent-local-unsupported-os = 로컬 에이전트는 아직 이 시스템에서 사용할 수 없습니다.
agent-model-download = 모델 내려받기
agent-model-downloading = 모델 내려받는 중… { $percent }%
agent-model-cancel = 취소
agent-model-download-failed = 모델을 내려받지 못했습니다.
agent-model-download-integrity = 내려받은 모델이 손상되어 삭제했습니다.
agent-model-download-incomplete = 내려받기가 완료되지 않았습니다: { $expected }바이트 중 { $got }바이트.
agent-model-download-cancelled = 내려받기를 취소했습니다.
local-agent-no-route = 로컬 에이전트가 무엇을 해야 할지 이해하지 못했습니다. 다르게 말해 보세요.
local-agent-tool-limit = 로컬 에이전트가 { $n }단계 후 끝내지 못하고 멈췄습니다.
local-agent-invalid-call = 로컬 에이전트가 도구를 올바르게 사용하지 못했습니다.
local-agent-engine-failed = 로컬 에이전트의 모델이 실패했습니다.
local-agent-retry = 다시 시도하거나, 에이전트 패널에서 다른 에이전트를 선택하세요.
agent-stopped = 에이전트가 중지되었습니다.
agent-failed = 에이전트가 요청을 거부했습니다: { $message }
agent-tool-failed = 도구가 예기치 않게 실패했습니다.

agent-turn-tools = 도구 { $tools }개
agent-turn-notes = 메모 { $notes }개
agent-turn-secs = { $secs }초
agent-turn-stopped = 중지됨.
agent-turn-interrupted = 중단됨.
agent-memory-lost = 에이전트가 이 대화를 더 이상 기억하지 못합니다. 처음부터 다시 시작합니다.
agent-turn-max-tokens = 에이전트가 길이 한도에 도달했습니다.
agent-turn-max-turns = 에이전트가 단계 한도에 도달했습니다.
agent-turn-refusal = 에이전트가 거절했습니다.
agent-turn-ended = 턴 종료 ({ $stop }).
agent-choice-cancelled = 질문이 취소되었습니다
agent-choice-answered = ✓ { $answer }
agent-report-why = 이유

agent-error-file = 파일을 읽거나 쓸 수 없습니다.
agent-error-unexpected-action = 예기치 않은 동작: { $action }.
agent-error-unknown-surface = 이 카드는 더 이상 열려 있지 않습니다.
agent-error-expired = 지난 턴의 동작이 만료되었습니다.
agent-error-incomplete = 이 카드의 동작이 불완전합니다.
agent-error-no-field = 이 설정은 더 이상 없습니다.
agent-error-no-variant = 이 변형은 더 이상 없습니다.
agent-error-arguments = 도구의 인수가 올바르지 않습니다({ $detail }). 스키마에 맞는지 확인하세요.
agent-error-unknown-tool = 알 수 없는 도구: { $name }.
agent-error-pack-id = 팩의 id가 아닙니다. packs를 호출해 팩과 그 id 목록을 확인하세요.
agent-error-asset-unknown = 이 이미지는 프레젠테이션에도 붙여넣은 이미지에도 없습니다. 붙여넣은 이미지의 ID를 사용하세요.
agent-error-denied = 사용자가 거절했습니다.
agent-error-no-turn = 진행 중인 턴이 없습니다. { $tool }은(는) 턴 중에만 표시됩니다.
agent-error-tab-no-turn = 팩 또는 테마 탭은 사용자에게 응답하는 동안에만 열거나 표시하세요.
agent-error-tab-limit = 이 턴에서 이미 팩 또는 테마 탭을 열었거나 표시했습니다. 그곳에서 마치거나, 다른 탭으로 옮기기 전에 사용자에게 물어보세요.
agent-error-continue-in-other-tab = 이 요청은 열었거나 표시한 탭에서 이어집니다. 사용자에게 그곳에서 계속하도록 안내하세요. 이 탭은 당분간 위젯을 변경하지 않습니다.
agent-error-pack-open-in-tab = 이 팩은 다른 탭에서 편집 중입니다. 사용자에게 먼저 그곳에서 마치도록 안내하세요.
agent-pack-opened-in-tab = { $shown ->
   [yes] 팩이 { $tab } 탭에서 열렸으며 지금 표시되어 있습니다.
  *[no] 팩이 { $tab } 탭에서 뒤쪽에 열렸습니다. 사용자의 창에는 다른 탭이 표시되어 있습니다.
} 사용자에게 그곳에서 계속하도록 안내하세요. 그 탭의 에이전트는 이 요청을 모릅니다. 이 탭은 그대로 유지됩니다.
agent-theme-opened-in-tab = { $shown ->
   [yes] 테마가 { $tab } 탭에서 열렸으며 지금 표시되어 있습니다.
  *[no] 테마가 { $tab } 탭에서 뒤쪽에 열렸습니다. 사용자의 창에는 다른 탭이 표시되어 있습니다.
} 사용자에게 그곳에서 계속하도록 안내하세요. 그 탭의 에이전트는 이 요청을 모릅니다. 이 탭은 그대로 유지됩니다.
agent-error-turn-over = 이 호출의 턴이 끝났습니다. 아무것도 표시되지 않았습니다.
agent-error-no-file = 프레젠테이션에 아직 파일이 없습니다. 절대 경로를 지정하세요.
agent-error-path-relative = 경로는 절대 경로여야 합니다.
agent-error-path-parent = 경로에 “..”가 포함되면 안 됩니다.
agent-error-setting-locked = 에이전트는 { $key } 설정을 변경할 수 없습니다.
agent-error-setting = { $key } 설정을 변경할 수 없습니다. 값을 확인하세요.
agent-error-history-index = 기록을 탐색하려면 버전 번호가 필요합니다.
agent-error-no-pack = 이 프레젠테이션은 팩을 고정하지 않습니다. pack을 지정하세요(packs 참고).
agent-error-history-not-opened = 탭이 제때 기록을 열지 못했습니다. 다시 요청하세요.
agent-error-present = 여기서는 발표할 것이 없습니다. 슬라이드가 없거나 탭이 팩, 기록 또는 읽기 전용 모드입니다.
agent-error-locale = 알 수 없는 언어 코드입니다. 제공되는 언어 코드나 system을 지정하세요.
agent-error-empty-interface = 바꿀 내용이 없습니다. scheme, locale 또는 guides를 지정하세요.
agent-error-empty-dictation = 바꿀 내용이 없습니다. 받아쓰기 설정을 하나 이상 지정하세요.
agent-error-not-shown = 이 탭은 창에 표시된 탭이 아닙니다. 사용자가 표시한 뒤 다시 요청하세요.
agent-error-themes-deck-mode = 테마 선택기는 슬라이드를 편집하는 동안에만 열립니다.
agent-error-packs-deck-mode = 프레젠테이션 팩 패널은 슬라이드를 편집하는 동안에만 열립니다.
agent-error-rewrite-text = 다시 쓰기가 텍스트 없이 돌아왔습니다.
agent-error-paragraphs = 다시 쓰기의 단락이 올바르지 않습니다.
agent-error-draft-too-long = 초안이 너무 깁니다.
agent-error-draft-origin = 초안이 거부되었습니다: 알 수 없는 보기입니다.
agent-error-draft-no-agent = 이 탭의 에이전트가 아직 시작되지 않았습니다: 초안이 거부되었습니다.
agent-error-too-many-images = 초안에 이미 이미지가 가득 찼습니다.
agent-error-image-not-base64 = 붙여넣은 이미지가 온전히 도착하지 않았습니다.
agent-error-image-gone = 붙여넣은 이미지를 더 이상 사용할 수 없습니다. 다시 붙여넣으세요.
agent-error-unknown-agent = 이 에이전트는 더 이상 설정에 없습니다.
agent-error-agent-locked = 에이전트 선택은 환경에 의해 지정됩니다.
agent-error-agent-not-saved = 에이전트 선택을 저장하지 못했습니다.
agent-error-unknown-model = 이 에이전트는 그 모델을 제공하지 않습니다.
agent-error-model-not-saved = 모델 선택을 저장하지 못했습니다.
voice-error-no-dictation = 여기서는 받아쓰기를 사용할 수 없습니다
voice-error-busy = 받아쓰기가 진행 중입니다. 멈춘 뒤 다시 시도하세요
voice-error-unknown-model = 알 수 없는 받아쓰기 모델입니다
voice-error-unavailable = 이 모델은 선택한 언어를 지원하지 않습니다
voice-error-installed = 이 모델은 이미 설치되어 있습니다
voice-error-not-downloadable = 이 모델은 다운로드하지 않습니다
voice-error-nothing-to-cancel = 진행 중인 다운로드가 없습니다
voice-error-bad-silence = 무음은 0.5~10초여야 합니다
voice-error-bad-language = 알 수 없는 받아쓰기 언어입니다
voice-error-bad-url = API 주소는 https://로 시작해야 하며(http://는 이 컴퓨터에만), 사용자 이름, 매개변수, 특수 문자는 쓸 수 없습니다
voice-error-bad-key-env = 키 변수는 OPENAI_API_KEY 같은 이름이어야 합니다: 대문자, 숫자, _
voice-error-bad-api-model = API 모델 이름에는 문자, 숫자와 . _ : / - 만 쓸 수 있고, Azure 배포 이름에는 문자, 숫자와 . _ - 만 쓸 수 있습니다
voice-error-bad-api-version = Azure API 버전에는 문자, 숫자, . 와 - 만 쓸 수 있고 최대 32자입니다(예: 2025-03-01-preview)
voice-error-bad-api-headers = 헤더는 Name: value; Name: $VARIABLE 형식으로 최대 8개까지 씁니다. Authorization, api-key, Host, Content-Type, Content-Length, Transfer-Encoding, Connection 와 중복은 거부됩니다
voice-error-api-reserved = API 주소, 키 변수, Azure 버전과 헤더는 사용자만 설정합니다: show_dictation으로 상자를 열고 사용자에게 요청하세요
voice-error-changed = 질문 이후 설정이 바뀌었습니다. 다시 물어보세요
voice-error-nothing-to-download = 이 에이전트에는 다운로드할 것이 없습니다
agent-tune-prompt = 가져온 프레젠테이션 미세 조정
agent-settings-unknown-selected = 설정 agent.selected = { $selected }은(는) [agent.presets]의 어떤 에이전트도 가리키지 않습니다(알려진 항목: { $known })
agent-settings-preset-launch = 에이전트 [agent.presets.{ $id }]에는 command와 builtin 중 정확히 하나가 필요합니다
