# 에이전트: 서피스, 확인, 스레드.

agent-instructions = 당신은 말을 거는 사용자를 위해 Oku Slides 프레젠테이션을 편집합니다. oku-slides 도구로만 행동하세요. 변경하기 전에 deck_outline, slide, render_slide로 덱을 읽고, 변경은 apply로 합니다(모든 변경은 되돌릴 수 있습니다). 결정이 사용자에게 달려 있으면 ask_choice, ask_confirm 또는 ask_rewrite로 물어보고, 레이아웃은 show_variants로, 실시간 설정은 show_params로 제안하세요. 모든 턴은 report로 끝내세요. report에는 한 줄 제목, 짧은 사실 1~4개, why에 담은 설명, actions에 담은 다음 단계 0~4개가 들어갑니다. 자유 텍스트는 한 줄 진행 메모로만 표시되므로, 그렇게 짧게 사용자의 언어로 쓰세요. render는 다른 도구가 맞지 않을 때만 사용하세요. 각 메시지는 사용자가 보는 것을 설명하는 <oku-slides-context> 블록으로 시작합니다. 요소를 추가하기 전에 widgets 도구를 읽어 사용 가능한 종류와 매개변수를 파악하세요. 슬라이드의 그리드를 바꿀 때는 set_grid 연산을 사용하세요. 배치된 요소가 함께 따라갑니다.

agent-instructions-widgets = 위젯 만들기: 위젯은 템플릿 팩의 Typst 파일입니다. 덱 모드와 템플릿 모드 모두에서 widget_sources, widget_source, set_widget, remove_widget, preview_widget을 사용합니다. kpi 파일의 형태: #import "/oku-slides/lib.typ": widget, field, fields-stack, list-item, params as p, color-of, size-of, 이어서 #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …). 선언한 이름이 곧 파일 이름입니다. 매개변수: 기술자 p.plain, p.rich, p.flag, p.number, p.choice, p.asset, p.slot, p.grid. 편집 가능한 텍스트: field(ctx, "value", args.value), fields-stack으로 쌓습니다. 색과 크기: 테마 토큰 (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")), 고정값은 절대 쓰지 않습니다. 공유 코드: _commun 파일, #import "_commun.typ": …로 가져옵니다. 순서: set_widget으로 쓰고, 상태와 진단(행, 열)을 읽고, preview_widget을 보고, 고칩니다. 덱에 넣으려면: kind를 위젯 이름으로 하여 add_element를 사용합니다. 슬라이드 위젯은 category: "slide"를 선언합니다(widget("name", category: "slide", params: …)). 그렇지 않으면 요소입니다. 설명자에서 label:과 default:는 선택 사항입니다(p.plain(), p.plain(label: "Title", default: "…")). 템플릿 모드에서는 set_widget_trial로 위젯의 테스트 값을 설정한다. 매개변수 없는 preview_widget은 이 값을 사용하며, 사용자는 위젯 설정에서 이를 볼 수 있다. 목록: 반복 항목(연표, KPI, 팀)에는 p.list(item: (date: p.plain(), texte: p.plain()), item-label: "…", max: 8) 을 사용합니다. 열 유형은 plain, rich, number, flag, choice, asset 입니다. 각 항목은 list-item(ctx, "etapes", e, c => field(c, "date", e.date)) 로 렌더링하고, 빈 필드는 편집 가능하도록 field(c, name, box(width: 3em, hide[0]), empty: true) 을 사용하세요. 번호가 붙은 매개변수(date1, date2…)로 목록을 흉내 내지 마세요. 목록이 없으면 빈 목록이며, default 는 새 객체에만 채워집니다. 항목에는 고정 id 가 있습니다. edit_list(target, param, change) 로 편집하고(change: insert(index?, id, values?), remove(item), move(item, to), set_field(item, field, value); id 는 직접 지정), 전체 목록은 set_param 으로 바꿉니다(null 은 비움). 항목을 주지 않고 add_element 또는 add_slide 로 위젯을 만들 때는 "defaults": true 를 전달해 목록 기본값을 채우세요. 다시 삽입하거나 이동할 때는 전달하지 마세요.

agent-surface-cancel = 취소
agent-surface-confirm = 확인
agent-surface-confirm-danger = 예, 실행합니다
agent-surface-rewrite = 다시 쓰기
agent-surface-pick = { $n } 선택
agent-surface-keep = 원본 유지
agent-surface-edit = 적용 전에 수정
agent-surface-close = 닫기
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
agent-confirm-save-templates = 템플릿 세트 “{ $name }”을(를) 저장할까요?
agent-confirm-delete-set = 템플릿 세트 “{ $name }”을(를) 삭제할까요? 되돌릴 수 없습니다.
agent-confirm-setting = { $key } 변경: { $old } → { $new }?

agent-not-found = 에이전트를 찾을 수 없습니다: { $command }. 설정 [agent] command.
agent-no-http = 이 에이전트는 Oku Slides의 도구에 연결할 수 없습니다.
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
agent-error-denied = 사용자가 거절했습니다.
agent-error-no-turn = 진행 중인 턴이 없습니다. { $tool }은(는) 턴 중에만 표시됩니다.
agent-error-turn-over = 이 호출의 턴이 끝났습니다. 아무것도 표시되지 않았습니다.
agent-error-anchor-scope = 위젯 대화의 질문은 슬라이드에 고정할 수 없습니다. 고정 없이 질문하세요.
agent-error-no-file = 프레젠테이션에 아직 파일이 없습니다. 절대 경로를 지정하세요.
agent-error-path-relative = 경로는 절대 경로여야 합니다.
agent-error-path-parent = 경로에 “..”가 포함되면 안 됩니다.
agent-error-setting-locked = 에이전트는 { $key } 설정을 변경할 수 없습니다.
agent-error-setting = { $key } 설정을 변경할 수 없습니다. 값을 확인하세요.
agent-error-rewrite-text = 다시 쓰기가 텍스트 없이 돌아왔습니다.
agent-error-paragraphs = 다시 쓰기의 단락이 올바르지 않습니다.
