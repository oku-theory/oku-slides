op-slide-not-found = Không tìm thấy trang chiếu: { $subject }
op-element-not-found = Không tìm thấy phần tử: { $subject }
op-duplicate-id = Mã định danh đã được dùng: { $subject }
op-index-out-of-range = Vị trí nằm ngoài phạm vi
op-invalid-value = Giá trị không hợp lệ cho { $subject }
op-invalid-value-because = Giá trị không hợp lệ cho { $subject }: { $reason }
op-value-no-value = tham số không có giá trị
op-value-no-dataset = biểu đồ không có tập dữ liệu
op-value-grid-tracks = lưới có từ 1 đến 24 cột và hàng
op-value-bad-item-id = mã của một mục là văn bản không rỗng, không có “/”
op-value-missing-item-id = mỗi mục cần có một mã
op-value-too-deep = các phần tử lồng nhau quá sâu
op-value-empty-path = đường dẫn trống
op-value-not-text = cần một văn bản hoặc null
op-value-empty-frame = chiều rộng và chiều cao phải lớn hơn 0
op-value-blocks-off-grid = trang chiếu tự sắp xếp các ô dạng dòng và lưới sẽ che chúng; trước tiên hãy chuyển từng phần tử sang một ô trống
op-value-outside-grid = ô nằm ngoài lưới
op-value-not-tokens = cần một đối tượng hoặc null
op-value-not-a-slot = tham số này không phải là ô chứa
op-value-out-of-bounds = số nằm ngoài khoảng cho phép
op-value-unreadable = giá trị không có dạng như mong đợi
op-unknown-widget = Tiện ích không xác định: { $subject }
op-unknown-param = Tham số không xác định: { $subject }
op-param-type-because = Giá trị không hợp lệ cho { $subject }: { $reason }
op-param-shape-text = { $nullable ->
    [yes] cần một văn bản hoặc null
   *[no] cần một văn bản
}
op-param-shape-choice = { $nullable ->
    [yes] cần một trong các giá trị của nó hoặc null
   *[no] cần một trong các giá trị của nó
}
op-param-shape-paragraphs = { $nullable ->
    [yes] cần một danh sách đoạn văn hoặc null
   *[no] cần một danh sách đoạn văn
}
op-param-shape-elements = { $nullable ->
    [yes] cần một danh sách phần tử hoặc null
   *[no] cần một danh sách phần tử
}
op-param-shape-flag = { $nullable ->
    [yes] cần một giá trị boolean hoặc null
   *[no] cần một giá trị boolean
}
op-param-shape-number = { $nullable ->
    [yes] cần một số hoặc null
   *[no] cần một số
}
op-param-shape-bounded-number = { $nullable ->
    [yes] cần một số nằm trong giới hạn hoặc null
   *[no] cần một số nằm trong giới hạn
}
op-param-shape-grid = { $nullable ->
    [yes] cần một lưới hoặc null
   *[no] cần một lưới
}
op-param-shape-chart-option = { $nullable ->
    [yes] cần một tùy chọn biểu đồ hoặc null
   *[no] cần một tùy chọn biểu đồ
}
op-param-shape-table-option = { $nullable ->
    [yes] cần một tùy chọn bảng hoặc null
   *[no] cần một tùy chọn bảng
}
op-param-shape-chart-or-table-option = { $nullable ->
    [yes] cần một tùy chọn biểu đồ hoặc bảng hoặc null
   *[no] cần một tùy chọn biểu đồ hoặc bảng
}
op-param-shape-items = { $nullable ->
    [yes] cần một danh sách mục hoặc null
   *[no] cần một danh sách mục
}
op-param-shape-list = { $nullable ->
    [yes] cần một danh sách hoặc null
   *[no] cần một danh sách
}
op-param-shape-add-element-only = { $nullable ->
    [yes] phần tử được thêm bằng add_element, hoặc null
   *[no] phần tử được thêm bằng add_element
}
op-param-shape-set-param-only = { $nullable ->
    [yes] giá trị được đặt bằng set_param, hoặc null
   *[no] giá trị được đặt bằng set_param
}
op-item-not-found = Không có mục { $subject }
op-list-full = Danh sách { $subject } đã đầy (tối đa { $max } mục)
op-list-minimum = Danh sách { $subject } cần ít nhất { $min } mục
op-list-cell-too-long = Một ô của danh sách { $subject } quá dài (cột { $column })
op-list-id-too-long = Mã của một mục trong danh sách { $subject } quá dài
op-list-too-many-items = Danh sách { $subject } có quá nhiều mục (tối đa { $max })
op-slot-rejects = { $subject } không được chấp nhận ở đây
op-wrong-category = Không thể dùng { $subject } ở đây
op-choice-out-of-range = Giá trị không được phép cho { $subject }
transaction-refused = Thao tác { $index } bị từ chối: { $reason }

sources-unreadable = Không đọc được nguồn
sources-not-sources = Đây không phải là kho nguồn Oku Slides
sources-unsupported-version = Nguồn này được tạo bởi phiên bản Oku Slides mới hơn
sources-bad-name = Tên tệp trong nguồn bị từ chối
sources-missing-deck = Nguồn không chứa bản trình bày
sources-too-large = Nguồn trong tệp này quá lớn để mở
sources-too-deep = Bản trình bày này lồng các phần tử quá sâu

issue-empty-slide-id = mã trang chiếu trống
issue-duplicate-slide-id = mã trang chiếu bị trùng
issue-duplicate-element-id = mã phần tử bị trùng
issue-empty-element-id = mã định danh trống
issue-cell-outside-grid = ô nằm ngoài lưới
issue-empty-frame = khung có kích thước bằng 0
issue-free-without-frame = phần tử tự do không có khung
issue-unknown-widget = tiện ích không xác định: { $kind }
issue-wrong-category = không thể dùng { $kind } ở đây
issue-unknown-param = tham số không xác định: { $param }
issue-param-type = giá trị không hợp lệ cho { $param }
issue-choice-out-of-range = giá trị không được phép cho { $param }
issue-slot-rejects = { $kind } không được chấp nhận ở đây
issue-placed-in-flow = phần tử đặt bằng khung trong một danh sách được dàn trang
issue-unknown-bind = { $param } được liên kết với một thiết lập bản trình bày không xác định
issue-slot-too-deep = các phần tử lồng nhau quá sâu
issue-missing-asset = thiếu tài nguyên: { $asset }
issue-pack-version-mismatch = đã loại bỏ phiên bản gói bị thay đổi: { $version }
issue-pack-version-missing = thiếu phiên bản gói trong nguồn: { $version }
issue-theme-version-mismatch = đã loại bỏ phiên bản chủ đề bị thay đổi: { $version }
issue-theme-version-missing = thiếu phiên bản chủ đề trong nguồn: { $version }
issue-unsupported-link = liên kết không được hỗ trợ: { $link }
issue-highlight-zero = màu tô sáng được đánh số từ 1
issue-unreadable-element = phần tử không đọc được, đã để riêng
issue-unreadable-item = mục danh sách không đọc được, đã để riêng
issue-unreadable-row = hàng bảng không đọc được, đã để riêng
issue-data-truncated = dữ liệu quá lớn, đã cắt bớt khi đọc
issue-unreadable-slide = trang chiếu không đọc được, đã để riêng
issue-unreadable-field = trường { $field } của bản trình bày không đọc được, đã đặt lại
op-invalid-chart = Biểu đồ { $subject }: { $reason }
op-invalid-table = Bảng { $subject }: { $reason }

replica-corrupted = Lịch sử bản trình bày trong tệp này bị hỏng
replica-too-large = Lịch sử bản trình bày trong tệp này quá lớn để mở
replica-write = Không thể cập nhật lịch sử bản trình bày
replica-unrelated = Hai bản trình bày này không có chung nguồn gốc
replica-unsupported-version = Bản trình bày này dùng định dạng mà phiên bản Oku Slides này không đọc được

widget-unreadable = Không thể đọc khai báo widget
widget-fault-empty-name = Widget { $widget }: một widget hoặc tham số không có tên
widget-fault-duplicate-widget = Widget { $widget } được khai báo hai lần
widget-fault-duplicate-param = Widget { $widget } khai báo cùng một tham số hai lần
widget-fault-empty-choice = Widget { $widget }: một lựa chọn không có giá trị
widget-fault-default-not-in-choice = Widget { $widget }: giá trị mặc định của lựa chọn không nằm trong các giá trị của nó
widget-fault-default-wrong-type = Widget { $widget }: một giá trị mặc định không khớp với kiểu của tham số của nó
widget-fault-bind-not-plain = Widget { $widget }: chỉ tham số văn bản thuần mới có thể liên kết với bản trình bày
widget-fault-unknown-slot-kind = Widget { $widget }: một slot chấp nhận widget phần tử không xác định
widget-fault-empty-list = Widget { $widget }: danh sách cần ít nhất một cột
widget-fault-column-type = Widget { $widget }: cột của danh sách có thể là plain, rich, number, flag, choice hoặc asset; với nội dung tự do cho mỗi mục, hãy dùng p.slot với một widget con
widget-fault-reserved-name = Widget { $widget }: tên dành riêng: id, kind hoặc tên chứa /
widget-fault-list-bounds = Widget { $widget }: giá trị tối đa của danh sách thấp hơn giá trị tối thiểu
widget-fault-list-default-too-large = Widget { $widget }: giá trị mặc định của danh sách vượt quá giới hạn kích thước danh sách
widget-incompatible-override = Widget { $widget } thay thế widget có sẵn nhưng thay đổi hoặc bỏ tham số { $param }
widget-incompatible-category = Widget { $widget } thay thế widget có sẵn bằng một danh mục khác
issue-empty-item-id = danh sách { $param } có một mục không có mã định danh
issue-duplicate-item-id = danh sách { $param } có mã định danh mục bị trùng
issue-list-bounds = danh sách { $param } có quá ít hoặc quá nhiều mục
