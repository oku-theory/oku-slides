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
