# 앱 인터페이스 (React). ui/src/i18n.ts에서 « messages » 요청으로 불러옵니다.
ui-loading = 불러오는 중…

## 문서

ui-brand = Oku Slides
ui-brand-templates = Oku Slides · 템플릿
ui-title-rename = 프레젠테이션 이름 바꾸기
ui-title-field = 프레젠테이션 제목
ui-status-unsaved = 저장되지 않은 변경 사항
ui-status-saved = 저장됨
ui-status-never-saved = 아직 저장되지 않음
ui-status-builtin = 기본 제공: 저장하면 사본이 만들어집니다
ui-caption-templates = { $count ->
   *[other] 템플릿 { $count }개
} · { $status }
ui-caption-slides = { $count ->
   *[other] 슬라이드 { $count }개
} · { $status }
ui-template-default-name = 템플릿
ui-template-variant-name = { $name } (변형)
ui-confirm-quit-templates = 수정한 템플릿을 저장하지 않고 종료할까요?

## 알림

ui-toast-templates-saved = 템플릿을 저장했습니다: { $name }
ui-toast-moved = 프레젠테이션을 이동했습니다
ui-toast-copied = 사본을 저장했습니다
ui-toast-select-zone = 배치할 영역을 선택하세요: “{ $text }”

## 도구 모음

ui-toolbar-undo = 실행 취소
ui-toolbar-undo-label = 실행 취소: { $label }
ui-toolbar-redo = 다시 실행
ui-toolbar-redo-label = 다시 실행: { $label }
ui-toolbar-grid-hide = 그리드 숨기기
ui-toolbar-grid-show = 그리드 표시
ui-toolbar-theme = 테마
ui-toolbar-theme-default = 기본값
ui-toolbar-save-copy = 사본 저장
ui-toolbar-finish = 완료
ui-toolbar-templates = 템플릿
ui-toolbar-open = 열기
ui-merge = PDF와 병합…
ui-menu = 메뉴
ui-menu-move = 이동…
ui-language = 언어
ui-language-system = 시스템 언어
ui-toolbar-add-label = 슬라이드에 추가
ui-add-kind = { $kind ->
    [card] 카드
    [shape] 도형
    [chart] 차트
   *[text] 텍스트
}
ui-toolbar-remove-template = 템플릿 삭제
ui-toolbar-remove-slide = 슬라이드 삭제
ui-toolbar-remove-hint = 슬라이드 { $n } · ⌘Z로 실행 취소

## 실행 취소 레이블

ui-undo-add = { $kind ->
    [card] 카드 추가
    [shape] 도형 추가
    [chart] 차트 추가
   *[text] 텍스트 추가
}
ui-undo-new-template = 새 템플릿: { $name }
ui-undo-new-slide = 새 슬라이드: { $name }
ui-undo-grid = 그리드 { $columns } × { $rows }
ui-undo-templates = 템플릿: { $name }
ui-undo-move-template = 템플릿 이동
ui-undo-move-slide = 슬라이드 이동
ui-undo-remove-slide = 슬라이드 삭제
ui-undo-remove = 삭제
ui-undo-cut = 잘라내기
ui-undo-paste = 붙여넣기
ui-undo-place-on-grid = 그리드에 배치
ui-undo-move-on-grid = 그리드에서 이동
ui-undo-resize-on-grid = 그리드에서 크기 조정
ui-undo-change-anchor = 고정 위치 변경
ui-undo-move = 이동
ui-undo-resize = 크기 조정
ui-undo-edit-text = 텍스트 편집
ui-undo-dictation = 받아쓰기
ui-undo-rename-set = 세트 이름 바꾸기
ui-undo-rename-deck = 프레젠테이션 이름 바꾸기
ui-undo-theme = 테마 변경
ui-undo-rename-template = 템플릿 이름 바꾸기
ui-undo-describe-template = 템플릿 설명 작성
ui-undo-chart-kind = 차트 유형 변경
ui-undo-chart-data = 데이터 편집
ui-undo-chart-color = 계열 색 지정
ui-undo-chart-stack = { $stacked ->
    [yes] 계열 누적
   *[no] 계열 누적 해제
}

## 새 요소

ui-new-text = 새 텍스트
ui-new-card-title = 카드 제목
ui-new-card-detail = 세부 정보

## 슬라이드 목록

ui-pile-slides = 슬라이드
ui-pile-templates = 템플릿
ui-pile-add-slide = + 새 슬라이드
ui-pile-add-template = + 새 템플릿
ui-pile-drag-hint = 드래그하여 이동 · ⌥↑ ⌥↓
ui-slide-alt = 슬라이드 { $n }

## 스테이지

ui-stage-rendering = 렌더링 중…
ui-stage-elements = 슬라이드 요소
ui-stage-edit-text = 텍스트 편집
ui-stage-hint-field = Enter로 확인 · Esc로 취소
ui-stage-hint-block = ⌘ Enter로 확인 · 단락마다 한 줄, 글머리 기호는 “- ” · ⌘B 굵게, ⌘I 기울임, ⌘K 링크
ui-field = { $kind ->
    [title] 제목
    [subtitle] 부제목
    [kicker] 머리말
    [text] 텍스트
   *[other] { $kind }
}
ui-element-kind = { $kind ->
    [text] 텍스트
    [card] 카드
    [image] 이미지
    [shape] 도형
    [group] 그룹
   *[other] { $kind }
}
ui-grid-cell = { $cols }열 · { $rows }행

## 슬라이드 노트

ui-notes = 슬라이드 도구
ui-stepper-less = { $label }: 하나 줄이기
ui-stepper-more = { $label }: 하나 늘리기
ui-agent-color-other = 다른 색
ui-agent-thread = 에이전트
ui-agent-new-conversation = 새 대화
ui-agent-tools = 도구 { $count }개
ui-draft = 초안
ui-draft-insert = 삽입
ui-draft-insert-title = 그대로 삽입 (⌘⇧↩)
ui-draft-send = 에이전트에게 보내기
ui-draft-send-title = 에이전트에게 보내기 (⌘↩)
ui-draft-discard = 버리기
ui-agent-state-idle = 에이전트
ui-agent-state-starting = 에이전트 시작 중…
ui-agent-state-working = 에이전트가 작업 중…
ui-agent-stopped = 중지됨.
ui-agent-ended = 턴 종료 ({ $stop }).
ui-agent-stop-max-tokens = 에이전트가 길이 한도에 도달했습니다.
ui-agent-stop-max-turns = 에이전트가 단계 한도에 도달했습니다.
ui-agent-stop-refusal = 에이전트가 거절했습니다.
ui-agent-sheet = 에이전트의 질문
ui-agent-sheet-keys = 1–4 선택 · Enter 확인 · E 편집 · Esc 취소
ui-agent-variants = 변형
ui-agent-drawer-keys = 클릭 또는 1–4 · Esc로 닫기
ui-agent-confirm = 에이전트 확인

## 받아쓰기

ui-dictation = 받아쓰기
ui-dictation-start = 받아쓰기
ui-dictation-start-title = 받아쓰기 (⌘D)
ui-dictation-agent-stop = 에이전트 중지 (⌘⇧D 또는 Esc)
ui-dictation-agent-working = 에이전트가 작업 중… ⌘⇧D 또는 Esc로 중지
ui-dictation-finish = 받아쓰기 종료
ui-dictation-finish-title = 종료 (⌘D)
ui-dictation-cancel = 취소
ui-dictation-preparing = 받아쓰기 준비 중…
ui-dictation-heard-nothing = 들리지 않았습니다. 마이크에 조금 더 가까이 말씀하세요
ui-dictation-to-cursor = 커서 위치에
ui-dictation-to-draft = 초안으로
ui-dictation-to-field = { $field } 바꾸기
ui-dictation-to-block = 선택한 블록 뒤에
ui-dictation-new-block = 새 텍스트 블록
ui-dictation-select-zone = 영역 선택
ui-dictation-downloading = { $name } 모델 다운로드 중
ui-dictation-download-progress = { $percent }% · { $total } 중 { $received } · 최초 한 번만
ui-dictation-listening = 듣는 중 · { $time }
ui-dictation-transcribing = 마지막 문장을 변환하는 중…
ui-dictation-stop-in = { $duration } 후 중지
ui-dictation-speak = 말씀하세요. 텍스트가 여기에 표시됩니다…
ui-dictation-auto-stop = { $duration } 동안 조용하면 중지
ui-dictation-to-download = { $size } 다운로드 필요
ui-dictation-settings = 받아쓰기 설정
ui-dictation-model-title = 인식 모델
ui-dictation-model-intro = 이 컴퓨터에서 실행되는 Whisper (CTranslate2). 모든 모델이 여러 언어를 지원합니다.
ui-dictation-note-turbo = 권장, 빠르고 정확함
ui-dictation-note-large = 가장 정확하지만 느림
ui-dictation-note-base = 가벼움, 사양이 낮은 컴퓨터용
ui-dictation-installed = 설치됨
ui-dictation-not-installed = 다운로드 필요
ui-dictation-language = 음성 언어
ui-dictation-language-auto = 자동 감지
ui-dictation-auto-stop-label = 다음 시간 동안 조용하면 자동 중지
ui-dictation-silence = 무음 시간
ui-dictation-phrases-note = 말하는 동안 음성 구간만 문장 단위로 변환됩니다.
ui-dictation-download-now = 지금 다운로드 ({ $size })
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) }초
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } GB
ui-size-mb = { $value } MB

## 템플릿 선택기

ui-picker-new-slide = 새 슬라이드
ui-picker-new-template = 새 템플릿, 기반으로 할 템플릿…
ui-picker-set = 템플릿
ui-picker-set-mine = { $name } · 내 템플릿
ui-picker-keys = ← →로 탐색 · Enter로 추가 · Esc로 닫기
ui-picker-close = 닫기
ui-picker-previous = 이전 템플릿
ui-picker-next = 다음 템플릿
ui-picker-list = 슬라이드 템플릿

## 차트

ui-chart-panel = 차트
ui-chart-kinds = 차트 유형
ui-chart-kind = { $kind ->
    [line] 선형
    [pie] 원형
    [scatter] 분산형
    [radar] 방사형
    [waterfall] 폭포형
   *[bar] 막대형
}
ui-chart-no-data = 이 차트에는 데이터 표가 없습니다.
ui-chart-options = 차트 옵션
ui-chart-stack = 누적
ui-chart-waterfall-hint = 합계 막대로 만들려면 「합계」를 선택하세요.
ui-chart-colors = 계열 색상
ui-chart-color-of = { $name } 색상
ui-chart-color-auto = 자동
ui-chart-color-pick = 색상 { $n }
ui-chart-column-name = { $col }열 이름
ui-chart-remove-column = 열 삭제
ui-chart-cell = { $row }행, { $col }열
ui-chart-remove-row = 행 삭제
ui-chart-add-row = 행
ui-chart-add-column = 열
ui-chart-series = 계열 { $n }

## 파일

ui-file-open-prompt = 열 파일의 경로 (.oku-slides.pdf 또는 .json)
ui-file-save-prompt = 다른 이름으로 저장 (전체 경로)

## 탭

ui-tabs = 열린 프레젠테이션
ui-tab-new = 새 탭
ui-tab-close = 탭 닫기
ui-tab-unsaved = 저장하지 않은 변경 사항
ui-tab-templates = 템플릿
ui-tab-move-new-window = 새 창으로 이동
ui-tab-move-to = { $n }번 창으로 이동
ui-tab-close-others = 다른 탭 닫기
ui-close-cancel = 취소
ui-disk-replaced-question = { $file }이(가) 디스크에서 다른 프레젠테이션으로 바뀌었습니다.
ui-disk-replaced-overwrite = 덮어쓰기
ui-disk-replaced-save-as = 다른 이름으로 저장…
ui-same-deck-question = { $file }은(는) “{ $title }” 탭에 열려 있는 프레젠테이션의 다른 버전입니다.
ui-same-deck-merge = 그 탭에 병합
ui-same-deck-apart = 따로 열기
ui-history = 기록
ui-history-version = { $date } 버전
ui-history-restore = 복원
ui-history-back = 돌아가기
ui-history-anonymous = 익명
ui-history-slider = 프레젠테이션 버전
ui-history-removed-since = 이후 삭제됨
ui-history-changed-since = 이후 변경됨
ui-history-added-since = 이후 추가된 슬라이드: { $count }
