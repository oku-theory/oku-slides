data-cell-type = một giá trị không hợp với cột “{ $column }”, cột này chứa { $type ->
    [number] số
    [integer] số nguyên
    [bool] đúng hoặc sai
    [date] ngày (YYYY-MM-DD)
   *[text] văn bản tối đa 16 KB
}
data-column-type = không thể đổi cột “{ $column }” thành { $type ->
    [number] cột số
    [integer] cột số nguyên
    [bool] cột đúng/sai
    [date] cột ngày
   *[text] cột văn bản
} mà không mất dữ liệu, xem các hàng { $rows }{ $more ->
    [0] {""}
   *[other] {" "}và { $more } hàng khác
}
data-no-row = hàng { $row } không tồn tại
data-no-column = cột { $column } không tồn tại
data-id-taken = mã { $id } đã được dùng trong bảng này
data-id-invalid = mã “{ $id }” không hợp lệ: dùng từ 1 đến 64 chữ cái, chữ số, - hoặc _
data-id-missing = một hàng, cột hoặc bảng mới chưa có mã
data-too-large = { $what ->
    [rows] dữ liệu sẽ vượt giới hạn { $max } hàng
    [columns] dữ liệu sẽ vượt giới hạn { $max } cột
    [text] một ô văn bản sẽ vượt giới hạn { $max } byte
   *[overlay] dữ liệu sẽ vượt giới hạn { $max } ô đã sửa
}
data-invalid-name = “{ $name }” không phải tên dữ liệu hợp lệ: dùng chữ thường, chữ số và _, không bắt đầu bằng chữ số
data-non-finite = { $count } số không hữu hạn (NaN hoặc vô cực)
data-columns-invalid = danh sách cột trống hoặc nêu một cột hai lần
data-row-width = một hàng có nhiều ô hơn { $max } cột của bảng
data-needs-base = dữ liệu này lưu các hàng trong một tệp nền mà thao tác sửa này chưa đọc được
data-base-row-cells = bảng có tệp nền chỉ ghi mã của các hàng, không ghi ô
data-row-cut = { $count } hàng có nhiều ô hơn tiêu đề; các ô thừa đã bị xóa
data-base-invalid = tệp cơ sở của dữ liệu bị hỏng hoặc không khớp với bảng của nó
data-source-mismatch = nguồn của dữ liệu không khớp với loại: dữ liệu thủ công chứa một bảng, dữ liệu truy vấn chứa một truy vấn
data-inline-query = dữ liệu truy vấn không thể là dữ liệu nhúng: dữ liệu nhúng luôn đến từ một bảng
data-query-parse = không đọc được truy vấn: hãy kiểm tra dòng { $line }, cột { $column }
data-query-construct = truy vấn dùng “{ $construct }”, điều mà truy vấn dữ liệu không hỗ trợ
data-query-too-long = truy vấn dài hơn { $max } byte
data-query-too-deep = truy vấn lồng nhau quá sâu
data-query-rewrite = không thể chuẩn bị truy vấn cho bộ máy; đây là lỗi của Oku Slides
data-query-unknown-function = truy vấn gọi “{ $function }”, hàm mà truy vấn dữ liệu không hỗ trợ
data-query-run = truy vấn thất bại: { $reason }
data-query-input-failed = đầu vào “{ $input }” thất bại
data-query-too-large = { $what ->
    [rows] kết quả truy vấn vượt giới hạn { $max } hàng; kết quả không bao giờ bị cắt
    [columns] kết quả truy vấn vượt giới hạn { $max } cột; kết quả không bao giờ bị cắt
   *[text] một ô văn bản của kết quả truy vấn vượt giới hạn { $max } byte; kết quả không bao giờ bị cắt
}
data-query-column-type = cột kết quả “{ $column }” có kiểu mà dữ liệu không chứa được: hãy chuyển nó thành văn bản, số, số nguyên, giá trị logic hoặc ngày
data-query-duplicate-column = hai cột kết quả cùng tên “{ $column }”: hãy đổi tên một cột bằng AS
data-query-float-operand = không thể áp dụng “{ $function }” cho “{ $operand }” vì nó chứa số thập phân: hãy chuyển nó thành số nguyên trước
data-query-timeout = truy vấn chạy quá { $seconds } giây và đã bị dừng
