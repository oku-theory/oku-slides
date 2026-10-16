op-slide-not-found = 슬라이드를 찾을 수 없습니다: { $subject }
op-element-not-found = 요소를 찾을 수 없습니다: { $subject }
op-duplicate-id = 이미 사용 중인 식별자입니다: { $subject }
op-index-out-of-range = 위치가 범위를 벗어났습니다
op-invalid-value = { $subject }의 값이 올바르지 않습니다
op-invalid-value-because = { $subject }의 값이 올바르지 않습니다: { $reason }
op-value-no-value = 매개변수에 값이 없습니다
op-value-no-dataset = 차트에 데이터 세트가 없습니다
op-value-grid-tracks = 그리드의 열과 행은 1~24개입니다
op-value-bad-item-id = 항목 ID는 “/”가 없는 비어 있지 않은 텍스트입니다
op-value-missing-item-id = 모든 항목에는 ID가 필요합니다
op-value-too-deep = 요소가 너무 깊이 중첩되어 있습니다
op-value-empty-path = 경로가 비어 있습니다
op-value-not-text = 텍스트 또는 null이어야 합니다
op-value-empty-frame = 너비와 높이는 0보다 커야 합니다
op-value-blocks-off-grid = 흐름 슬롯은 슬라이드가 배치하며 그리드를 쓰면 가려집니다. 먼저 각 요소를 빈 슬롯으로 옮기세요
op-value-outside-grid = 셀이 그리드 밖에 있습니다
op-value-not-tokens = 객체 또는 null이어야 합니다
op-value-not-a-slot = 이 매개변수는 슬롯이 아닙니다
op-value-out-of-bounds = 숫자가 허용 범위를 벗어났습니다
op-value-unreadable = 값의 형식이 예상과 다릅니다
op-value-valign = top, middle 또는 bottom이어야 합니다
op-value-path = 경로는 M, L, C, Q, Z 명령과 그 숫자로 이루어지며 M으로 시작합니다
op-unknown-widget = 알 수 없는 위젯: { $subject }
op-unknown-param = 알 수 없는 매개변수: { $subject }
op-param-type-because = { $subject }의 값이 올바르지 않습니다: { $reason }
op-param-shape-text = { $nullable ->
    [yes] 텍스트 또는 null만 허용됩니다
   *[no] 텍스트만 허용됩니다
}
op-param-shape-choice = { $nullable ->
    [yes] 값 중 하나 또는 null만 허용됩니다
   *[no] 값 중 하나만 허용됩니다
}
op-param-shape-paragraphs = { $nullable ->
    [yes] 단락 목록 또는 null만 허용됩니다
   *[no] 단락 목록만 허용됩니다
}
op-param-shape-elements = { $nullable ->
    [yes] 요소 목록 또는 null만 허용됩니다
   *[no] 요소 목록만 허용됩니다
}
op-param-shape-flag = { $nullable ->
    [yes] 불리언 또는 null만 허용됩니다
   *[no] 불리언만 허용됩니다
}
op-param-shape-number = { $nullable ->
    [yes] 숫자 또는 null만 허용됩니다
   *[no] 숫자만 허용됩니다
}
op-param-shape-bounded-number = { $nullable ->
    [yes] 허용 범위 안의 숫자 또는 null만 허용됩니다
   *[no] 허용 범위 안의 숫자만 허용됩니다
}
op-param-shape-grid = { $nullable ->
    [yes] 그리드 또는 null만 허용됩니다
   *[no] 그리드만 허용됩니다
}
op-param-shape-chart-option = { $nullable ->
    [yes] 차트 옵션 또는 null만 허용됩니다
   *[no] 차트 옵션만 허용됩니다
}
op-param-shape-table-option = { $nullable ->
    [yes] 표 옵션 또는 null만 허용됩니다
   *[no] 표 옵션만 허용됩니다
}
op-param-shape-chart-or-table-option = { $nullable ->
    [yes] 차트 또는 표 옵션 또는 null만 허용됩니다
   *[no] 차트 또는 표 옵션만 허용됩니다
}
op-param-shape-items = { $nullable ->
    [yes] 항목 목록 또는 null만 허용됩니다
   *[no] 항목 목록만 허용됩니다
}
op-param-shape-list = { $nullable ->
    [yes] 목록 또는 null만 허용됩니다
   *[no] 목록만 허용됩니다
}
op-param-shape-add-element-only = { $nullable ->
    [yes] 요소는 add_element로 추가합니다, 또는 null
   *[no] 요소는 add_element로 추가합니다
}
op-param-shape-set-param-only = { $nullable ->
    [yes] 값은 set_param으로 설정합니다, 또는 null
   *[no] 값은 set_param으로 설정합니다
}
op-param-shape-path-option = { $nullable ->
    [yes] 도형 경로(키 d가 있는 객체) 또는 null이어야 합니다
   *[no] 도형 경로(키 d가 있는 객체)여야 합니다
}
op-item-not-found = 항목 { $subject }을(를) 찾을 수 없습니다
op-list-full = 목록 { $subject }이(가) 가득 찼습니다(항목 최대 { $max }개)
op-list-minimum = 목록 { $subject }에는 항목이 최소 { $min }개 있어야 합니다
op-list-cell-too-long = 목록 { $subject }의 셀이 너무 깁니다(열 { $column })
op-list-id-too-long = 목록 { $subject }의 항목 ID가 너무 깁니다
op-list-too-many-items = 목록 { $subject }의 항목이 너무 많습니다(최대 { $max }개)
op-slot-rejects = { $subject }은(는) 여기에 허용되지 않습니다
op-wrong-category = { $subject }은(는) 여기에서 사용할 수 없습니다
op-choice-out-of-range = { $subject }에 허용되지 않는 값입니다
transaction-refused = 작업 { $index }이(가) 거부되었습니다: { $reason }

sources-unreadable = 소스를 읽을 수 없습니다
sources-not-sources = Oku Slides 소스 아카이브가 아닙니다
sources-unsupported-version = 더 새로운 버전의 Oku Slides에서 작성된 소스입니다
sources-bad-name = 소스에 허용되지 않는 파일 이름이 있습니다
sources-missing-deck = 소스에 프레젠테이션이 없습니다
sources-too-large = 이 파일의 소스가 너무 커서 열 수 없습니다
sources-too-deep = 이 덱은 요소를 너무 깊게 중첩합니다

issue-empty-slide-id = 슬라이드 식별자가 비어 있음
issue-duplicate-slide-id = 슬라이드 식별자 중복
issue-duplicate-element-id = 요소 식별자 중복
issue-empty-element-id = 식별자가 비어 있음
issue-cell-outside-grid = 셀이 그리드 밖에 있음
issue-empty-frame = 크기가 0인 프레임
issue-free-without-frame = 프레임이 없는 자유 요소
issue-unknown-widget = 알 수 없는 위젯: { $kind }
issue-wrong-category = { $kind }은(는) 여기에서 사용할 수 없습니다
issue-unknown-param = 알 수 없는 매개변수: { $param }
issue-param-type = { $param }의 값이 잘못되었습니다
issue-choice-out-of-range = { $param }에 허용되지 않는 값입니다
issue-slot-rejects = { $kind }은(는) 여기에서 허용되지 않습니다
issue-placed-in-flow = 배치된 목록에 프레임으로 놓인 요소
issue-unknown-bind = { $param }이(가) 알 수 없는 덱 설정에 연결되어 있습니다
issue-slot-too-deep = 요소가 너무 깊이 중첩되었습니다
issue-invalid-style = 잘못된 스타일 값: { $field } ({ $reason ->
        [valign] { op-value-valign }
        [out-of-bounds] { op-value-out-of-bounds }
       *[other] { op-value-unreadable }
    })
issue-missing-asset = 누락된 에셋: { $asset }
issue-pack-version-mismatch = 변경된 팩 버전을 버렸습니다: { $version }
issue-pack-version-missing = 소스에 팩 버전이 없습니다: { $version }
issue-theme-version-mismatch = 변경된 테마 버전을 버렸습니다: { $version }
issue-theme-version-missing = 소스에 테마 버전이 없습니다: { $version }
issue-unsupported-link = 지원되지 않는 링크: { $link }
issue-highlight-zero = 강조 색상 번호는 1부터 시작합니다
issue-unreadable-element = 읽을 수 없는 요소를 따로 두었습니다
issue-slot-repaired = 알려진 슬롯이 없는 요소를 { $slot }에 배치했습니다
issue-unreadable-item = 읽을 수 없는 목록 항목을 따로 두었습니다
issue-unreadable-row = 읽을 수 없는 표 행을 따로 두었습니다
issue-data-truncated = 데이터가 너무 커서 읽을 때 잘랐습니다
issue-unreadable-slide = 읽을 수 없는 슬라이드를 따로 두었습니다
issue-unreadable-field = 읽을 수 없는 덱 항목 { $field }을(를) 초기화했습니다
op-invalid-chart = 차트 { $subject }: { $reason }
op-invalid-table = 표 { $subject }: { $reason }

replica-corrupted = 이 파일의 프레젠테이션 기록이 손상되었습니다
replica-too-large = 이 파일의 프레젠테이션 기록이 너무 커서 열 수 없습니다
replica-write = 프레젠테이션 기록을 업데이트할 수 없습니다
replica-unrelated = 이 두 프레젠테이션은 공통된 출처가 없습니다
replica-unsupported-version = 이 프레젠테이션은 이 버전의 Oku Slides에서 읽을 수 없는 형식을 사용합니다
replica-too-deep = 이 프레젠테이션을 저장할 수 없습니다. 삭제된 요소가 기록 안에 너무 깊이 중첩되어 있습니다. 최근 이동을 실행 취소한 다음 다시 저장하세요.

widget-unreadable = 위젯 선언을 읽을 수 없습니다
widget-fault-empty-name = 위젯 { $widget }: 위젯 또는 매개변수에 이름이 없습니다
widget-fault-duplicate-widget = 위젯 { $widget }이(가) 두 번 선언되었습니다
widget-fault-duplicate-param = 위젯 { $widget }이(가) 동일한 매개변수를 두 번 선언합니다
widget-fault-empty-choice = 위젯 { $widget }: 선택 항목에 값이 없습니다
widget-fault-default-not-in-choice = 위젯 { $widget }: 선택 항목의 기본값이 값 목록에 없습니다
widget-fault-default-wrong-type = 위젯 { $widget }: 기본값이 매개변수의 타입과 일치하지 않습니다
widget-fault-bind-not-plain = 위젯 { $widget }: 일반 텍스트 매개변수만 프레젠테이션에 바인딩할 수 있습니다
widget-fault-unknown-slot-kind = 위젯 { $widget }: 슬롯이 알 수 없는 요소 위젯을 허용합니다
widget-fault-empty-list = 위젯 { $widget }: 목록에는 최소 한 개의 열이 필요합니다
widget-fault-column-type = 위젯 { $widget }: 목록 열은 plain, rich, number, flag, choice, asset(또는 중첩된 p.list, 한 단계) 중 하나여야 합니다. 항목별 자유 콘텐츠에는 자식 위젯과 함께 p.slot을 사용하세요
widget-fault-reserved-name = 위젯 { $widget }: 예약된 이름: id, kind 또는 /를 포함하는 이름
widget-fault-list-bounds = 위젯 { $widget }: 목록의 최댓값이 최솟값보다 작습니다
widget-fault-list-default-too-large = 위젯 { $widget }: 목록의 기본값이 목록 크기 제한을 초과합니다
widget-fault-list-depth = 위젯 { $widget }: 중첩된 목록에는 목록 열을 넣을 수 없습니다. 목록은 한 단계까지만 중첩됩니다
widget-fault-list-default-below-min = 위젯 { $widget }: 중첩된 목록의 기본값 항목 수가 최솟값보다 적습니다
widget-incompatible-override = 위젯 { $widget }이(가) 기본 위젯을 대체하지만 매개변수 { $param }을(를) 변경하거나 제거합니다
widget-incompatible-category = 위젯 { $widget }이(가) 기본 위젯을 다른 범주로 대체합니다
issue-empty-item-id = 목록 { $param }에 식별자가 없는 항목이 있습니다
issue-duplicate-item-id = 목록 { $param }에 식별자가 중복된 항목이 있습니다
issue-invalid-item-id = 목록 { $param }에 슬래시가 들어간 항목 식별자가 있습니다
issue-list-bounds = 목록 { $param }의 항목이 너무 적거나 너무 많습니다
