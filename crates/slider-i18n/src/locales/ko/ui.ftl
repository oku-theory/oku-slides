# 앱 인터페이스 (React). ui/src/i18n.ts에서 « messages » 요청으로 불러옵니다.
ui-loading = 불러오는 중…

## 문서

ui-brand = Écho
ui-brand-templates = Écho · 템플릿
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

ui-toast-saved = 프레젠테이션을 저장했습니다
ui-toast-templates-saved = 템플릿을 저장했습니다: { $name }
ui-toast-select-zone = 배치할 영역을 선택하세요: “{ $text }”

## 도구 모음

ui-toolbar-undo = 실행 취소
ui-toolbar-undo-label = 실행 취소: { $label }
ui-toolbar-redo = 다시 실행
ui-toolbar-redo-label = 다시 실행: { $label }
ui-toolbar-grid-hide = 그리드 숨기기
ui-toolbar-grid-show = 그리드 표시
ui-toolbar-grid-hide-title = 그리드 숨기기 (⌘')
ui-toolbar-grid-show-title = 그리드 표시 (⌘')
ui-toolbar-theme = 테마
ui-toolbar-theme-default = 기본값
ui-toolbar-save-copy = 사본 저장
ui-toolbar-save = 저장
ui-toolbar-finish = 완료
ui-toolbar-templates = 템플릿
ui-toolbar-templates-title = 슬라이드 템플릿 만들기 및 편집
ui-toolbar-open = 열기
ui-toolbar-add-label = 슬라이드에 추가
ui-toolbar-add = 추가
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
ui-stage-hint-block = ⌘ Enter로 확인 · 단락마다 한 줄, 글머리 기호는 “- ”
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

## 받아쓰기

ui-dictation = 받아쓰기
ui-dictation-start = 받아쓰기
ui-dictation-start-title = 받아쓰기 (⌘D)
ui-dictation-finish = 받아쓰기 종료
ui-dictation-finish-title = 종료 (⌘D)
ui-dictation-cancel = 취소
ui-dictation-preparing = 받아쓰기 준비 중…
ui-dictation-heard-nothing = 들리지 않았습니다. 마이크에 조금 더 가까이 말씀하세요
ui-dictation-to-cursor = 커서 위치에
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
   *[bar] 막대형
}
ui-chart-no-data = 이 차트에는 데이터 표가 없습니다.
ui-chart-options = 차트 옵션
ui-chart-stack = 누적
ui-chart-column-name = { $col }열 이름
ui-chart-remove-column = 열 삭제
ui-chart-cell = { $row }행, { $col }열
ui-chart-remove-row = 행 삭제
ui-chart-add-row = + 행
ui-chart-add-column = + 열
ui-chart-series = 계열 { $n }

## 파일

ui-file-open-prompt = 열 파일의 경로 (.echo.pdf 또는 .json)
ui-file-save-prompt = 다른 이름으로 저장 (전체 경로)
