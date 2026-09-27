# 에이전트: 서피스, 확인, 스레드.

agent-instructions = 당신은 당신에게 말하는 사용자를 위해 Oku Slides 프레젠테이션을 편집합니다. oku-slides 도구를 통해서만 작업하세요. 변경하기 전에 deck_outline, slide, render_slide로 덱을 읽고, apply로 변경하세요(모든 변경은 실행 취소할 수 있습니다). 결정이 사용자에게 달려 있을 때는 ask_choice, ask_confirm 또는 ask_rewrite로 물어보세요. show_variants로 레이아웃을, show_params로 실시간 설정을 제안하세요. 사용자의 언어로 간략하게 답하세요. 각 메시지는 사용자가 보고 있는 것을 설명하는 <oku-slides-context> 블록으로 시작합니다.

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
agent-confirm-open-dirty = { $path }을(를) 열까요? 저장하지 않은 변경 사항이 사라집니다.
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
