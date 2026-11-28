data-cell-type = 값이 “{ $column }” 열에 맞지 않습니다 (이 열의 내용: { $type ->
    [number] 숫자
    [integer] 정수
    [bool] 참 또는 거짓
    [date] 날짜, YYYY-MM-DD
   *[text] 최대 16KB 텍스트
})
data-column-type = “{ $column }” 열은 손실 없이 { $type ->
    [number] 숫자 열로
    [integer] 정수 열로
    [bool] 참/거짓 열로
    [date] 날짜 열로
   *[text] 텍스트 열로
} 바꿀 수 없습니다. 해당 행: { $rows }{ $more ->
    [0] {""}
   *[other] {""}, 그 외 { $more }개
}
data-no-row = { $row } 행이 없습니다
data-no-column = { $column } 열이 없습니다
data-id-taken = ID { $id }는 이 표에서 이미 사용 중입니다
data-id-invalid = ID “{ $id }”는 유효하지 않습니다: 영문자, 숫자, - 또는 _를 1~64자로 사용하세요
data-id-missing = 새 행, 열 또는 표에 ID가 없습니다
data-too-large = { $what ->
    [rows] 데이터가 행 한도({ $max }개)를 넘게 됩니다
    [columns] 데이터가 열 한도({ $max }개)를 넘게 됩니다
    [text] 텍스트 셀이 한도({ $max }바이트)를 넘게 됩니다
   *[overlay] 데이터가 편집된 셀 한도({ $max }개)를 넘게 됩니다
}
data-invalid-name = “{ $name }”는 유효한 데이터 이름이 아닙니다: 영문 소문자, 숫자, _를 사용하고 숫자로 시작하지 마세요
data-non-finite = 유한하지 않은 숫자가 { $count }개 있습니다 (NaN 또는 무한대)
data-columns-invalid = 열 목록이 비어 있거나 같은 열을 두 번 지정합니다
data-row-width = 행의 셀 수가 표의 열 { $max }개보다 많습니다
data-needs-base = 이 데이터는 행을 기본 파일에 보관하며, 이 편집은 아직 그 파일을 읽을 수 없습니다
data-base-row-cells = 기본 파일이 있는 표는 행의 ID만 지정하고 셀은 지정하지 않습니다
data-row-cut = 머리글보다 셀이 많은 행 { $count }개에서 남는 셀을 삭제했습니다
data-base-invalid = 데이터의 기본 파일이 손상되었거나 표와 일치하지 않습니다
data-source-mismatch = 데이터의 소스가 종류와 맞지 않습니다. 수동 데이터는 표를, 쿼리 데이터는 쿼리를 가집니다
data-inline-query = 쿼리 데이터는 인라인일 수 없습니다. 인라인 데이터는 항상 표에서 만들어집니다
data-query-parse = 쿼리를 읽을 수 없습니다: { $line }행 { $column }열을 확인하세요
data-query-construct = 쿼리가 “{ $construct }”를 사용하지만 데이터 쿼리에서는 지원하지 않습니다
data-query-too-long = 쿼리가 { $max }바이트보다 깁니다
data-query-too-deep = 쿼리가 너무 깊이 중첩되어 있습니다
data-query-rewrite = 엔진용으로 쿼리를 준비할 수 없습니다. Oku Slides의 버그입니다
data-query-unknown-function = 쿼리가 “{ $function }”를 호출하지만 데이터 쿼리에서는 지원하지 않습니다
data-query-run = 쿼리가 실패했습니다: { $reason }
data-query-input-failed = 입력 “{ $input }”이(가) 실패했습니다
data-query-too-large = { $what ->
    [rows] 쿼리 결과가 행 한도({ $max }개)를 넘습니다. 결과는 잘리지 않습니다
    [columns] 쿼리 결과가 열 한도({ $max }개)를 넘습니다. 결과는 잘리지 않습니다
   *[text] 쿼리 결과의 텍스트 셀이 한도({ $max }바이트)를 넘습니다. 결과는 잘리지 않습니다
}
data-query-column-type = 결과 열 “{ $column }”의 형식은 데이터에 담을 수 없습니다. 텍스트, 숫자, 정수, 불리언, 날짜 중 하나로 변환하세요
data-query-duplicate-column = 결과 열 두 개의 이름이 “{ $column }”입니다. AS로 하나의 이름을 바꾸세요
data-query-float-operand = “{ $operand }”에는 소수가 있어 “{ $function }”를 적용할 수 없습니다. 먼저 정수로 변환하세요
data-query-timeout = 쿼리가 { $seconds }초를 넘겨 중지되었습니다
