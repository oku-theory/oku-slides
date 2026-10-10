pdf-unreadable = PDF를 읽을 수 없습니다
pdf-unsupported-version = 더 새로운 버전의 Oku Slides에서 작성된 PDF입니다
pdf-corrupted = 이 PDF의 소스가 손상되었습니다
pdf-write = PDF를 저장할 수 없습니다

render-compile = 프레젠테이션을 렌더링할 수 없습니다
render-pdf = PDF 내보내기에 실패했습니다
render-no-page = 존재하지 않는 페이지입니다
render-image = 슬라이드의 이미지를 생성할 수 없습니다

render-setup = 렌더러를 시작할 수 없습니다
render-missing-image = 이미지 없음
diagnostic-error = 오류
diagnostic-warning = 경고
diagnostic = { $level } ({ $place }): { $message }
diagnostic-unplaced = { $level }: { $message }
diagnostic-hint = 힌트: { $hint }
render-chart-failed = 차트를 그릴 수 없음

registry-bad-pack-id = 잘못된 템플릿 팩 식별자: { $id }
registry-compile = 템플릿 팩의 위젯을 컴파일할 수 없습니다
registry-missing = 템플릿 팩이 위젯을 선언하지 않았습니다
render-timeout = 렌더링이 { $seconds }초를 넘겨 중단되었습니다
render-out-of-memory = 렌더링에 { $mib } MiB가 넘는 메모리가 필요해 중단되었습니다
render-worker-lost = 렌더링 프로세스가 중지되었습니다
render-frame = 렌더링 프로세스의 메시지를 읽을 수 없습니다
render-page-too-large = 이 슬라이드는 미리 보기에 너무 큽니다({ $mib } MiB 초과, 대개 매우 큰 이미지 때문). PDF 또는 PNG로는 계속 내보낼 수 있습니다.
render-stale = 렌더링이 다른 요청으로 대체되었습니다. 다시 시도하세요
render-pack-disabled = '{ $pack }' 팩의 위젯은 이 세션에서 비활성화되었습니다
registry-schema-remote = 위젯 레지스트리가 올바르지 않습니다: { $message }
probe-not-probed = 아직 확인되지 않음: 팩의 위젯에 할당된 시간이 끝났습니다. 다음 렌더링 때 확인합니다.
probe-too-many-files = 팩의 위젯 파일이 { $max }개를 넘습니다: 이 파일은 불러오지 않습니다.
probe-too-large = { $max } KiB를 넘습니다: 불러오지 않습니다.

import-unreadable = 이 프레젠테이션을 읽을 수 없습니다
import-not-a-presentation = 이 파일은 PowerPoint 프레젠테이션이 아닙니다
import-protected = 이 프레젠테이션은 암호화되었거나 이전 .ppt 형식입니다. 먼저 .pptx로 저장하세요
import-too-large = 이 프레젠테이션은 너무 커서 가져올 수 없습니다
import-loss-list-levels = 목록 수준을 한 단계로 합쳤습니다
import-loss-merged-cells = 병합된 셀을 분리했습니다
import-loss-percent-stacked = 100% 누적 차트를 누적 차트로 표시했습니다
import-loss-run-style = { $effect ->
    [underline] 밑줄이 사라졌습니다
    [strike] 취소선이 사라졌습니다
    [baseline] 위 첨자 또는 아래 첨자가 사라졌습니다
    [caps] 대문자 표시가 사라졌습니다
   *[highlight] 강조 표시가 사라졌습니다
}
import-loss-transform = 회전, 대칭 또는 자르기가 사라졌습니다
import-loss-color-dropped = 색 { $color }에 가까운 테마 색이 없습니다. 테마 색을 사용했습니다
import-loss-text-truncated = 한도에 맞게 텍스트를 줄였습니다
import-loss-table-truncated = 한도에 맞게 표를 줄였습니다
import-loss-opaque = { $reason ->
    [media] 동영상 또는 소리를 가져오지 않았습니다 (대체 이미지 없음)
    [ole] 포함된 개체를 가져오지 않았습니다 (대체 이미지 없음)
    [math] 수식을 가져오지 않았습니다 (대체 이미지 없음)
    [smartart] SmartArt를 가져오지 않았습니다 (대체 이미지 없음)
   *[unknown] 지원되지 않는 개체를 가져오지 않았습니다 (대체 이미지 없음)
}
import-loss-unsupported-image = { $format } 형식의 이미지를 가져오지 않았습니다
import-loss-hidden-slide = 숨긴 슬라이드를 표시 상태로 가져왔습니다
import-loss-effects = 그림자, 반사, 3D 효과가 사라졌습니다
import-loss-series-dropped = 원형 차트는 계열 하나만 표시합니다. { $kept }개 유지, { $dropped }개 제외
import-loss-repeated-decor = 슬라이드 { $slides }장에서 반복되는 머리글 또는 바닥글 “{ $text }”을(를) 생략했습니다. 테마가 자체적으로 그립니다
import-loss-no-slot = 콘텐츠를 생략했습니다. 선택한 템플릿에 놓을 자리가 없습니다
import-warning-effect-ignored = 효과를 가져오지 못했습니다: { $detail }
import-warning-approximated = 근사치로 표시했습니다: { $detail }
import-warning-unreadable-item = 읽을 수 없는 항목을 건너뛰었습니다: { $detail }
import-warning-unreadable-part = 읽을 수 없는 부분: { $detail }
import-warning-missing-part = 누락된 부분: { $detail }
import-warning-unsupported-chart = 지원되지 않는 차트: { $detail }
import-warning-unsupported-image = 지원되지 않는 이미지 형식: { $detail }
import-warning-unknown-table-style = 알 수 없는 표 스타일: { $detail }
import-warning-unsafe-link = 링크가 거부되었습니다: { $detail }
import-warning-nesting-too-deep = 중첩이 너무 깊어 잘렸습니다: { $detail }
import-warning-font-embedded-unreadable = 포함된 글꼴을 읽을 수 없습니다: { $detail }
import-warning-truncated = 경고가 너무 많습니다. 이후 경고는 생략됩니다
