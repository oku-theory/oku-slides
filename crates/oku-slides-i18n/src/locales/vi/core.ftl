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
op-value-valign = cần top, middle hoặc bottom
op-value-path = đường dẫn gồm các lệnh M, L, C, Q và Z kèm số của chúng, bắt đầu bằng M
knowledge-path-empty = đường dẫn trống
knowledge-path-not-absolute = đường dẫn phải bắt đầu bằng /
knowledge-path-bad-segment = một đoạn bị trống, kết thúc bằng dấu chấm hoặc dùng ký tự ngoài a-z, 0-9, dấu chấm, dấu gạch dưới và dấu gạch ngang
knowledge-path-dot-segment = một đoạn không được bắt đầu bằng dấu chấm
knowledge-path-reserved = một đoạn là tên hệ thống được dành riêng
knowledge-path-too-deep = đường dẫn có quá nhiều đoạn
knowledge-path-too-long = đường dẫn quá dài
knowledge-path-not-markdown = tên tệp phải kết thúc bằng .md
knowledge-frontmatter-missing = tệp phải bắt đầu bằng một khối tiêu đề nằm giữa các dòng ---
knowledge-frontmatter-unterminated = khối tiêu đề thiếu dòng --- đóng
knowledge-frontmatter-too-large = khối tiêu đề quá lớn
knowledge-frontmatter-not-yaml = tiêu đề không phải là YAML hợp lệ
knowledge-frontmatter-not-mapping = tiêu đề phải là một ánh xạ có khóa dạng văn bản
knowledge-frontmatter-duplicate-key = khóa "{ $key }" xuất hiện hai lần trong tiêu đề
knowledge-frontmatter-alias = tiêu đề không được dùng neo hoặc bí danh YAML
knowledge-frontmatter-tag = tiêu đề không được dùng thẻ YAML
knowledge-frontmatter-too-deep = tiêu đề lồng nhau quá sâu
knowledge-frontmatter-after-document-end = tiêu đề có văn bản sau phần kết thúc tài liệu YAML ...
knowledge-frontmatter-type-missing = tiêu đề cần một kiểu không để trống
knowledge-frontmatter-bad-timestamp = giá trị "{ $key }" trong tiêu đề phải là ngày giờ RFC 3339 kèm độ lệch múi giờ
knowledge-frontmatter-not-allowed = khóa "{ $key }" không được phép trong tiêu đề của tệp này
knowledge-reason-not-found = không có tệp nào ở đường dẫn này
knowledge-reason-exists = đã có một tệp ở đường dẫn này
knowledge-reason-id-missing = tệp mới cần có mã định danh
knowledge-reason-id-invalid = mã định danh của tệp gồm 32 chữ số thập lục phân viết thường
knowledge-reason-id-taken = một tệp khác đã dùng mã định danh này
knowledge-reason-id-mismatch = tệp ở đường dẫn này có mã định danh khác
knowledge-reason-edit-empty-old = văn bản cần thay thế không được để trống
knowledge-reason-edit-no-match = văn bản cần thay thế không có trong tệp
knowledge-reason-edit-ambiguous = văn bản cần thay thế xuất hiện nhiều lần trong tệp
knowledge-reason-too-many-files = gói có quá nhiều tệp
knowledge-reason-file-too-large = tệp quá lớn
knowledge-reason-total-too-large = gói quá lớn
op-knowledge = Tệp tri thức { $path }: { $reason }
op-unknown-widget = Tiện ích không xác định: { $subject }
op-unknown-namespace = Bản trình bày này không có gói { $pack }.
op-pack-required = Gói { $pack } cần cho { $by }; hãy gỡ gói đó trước.
op-pack-in-use = Gói { $pack } vẫn đang được { $count } mục sử dụng.
op-pack-requirement-missing = Gói { $pack } cần { $required }, nhưng bản trình bày này không có.
op-base-required = Mỗi bản trình bày luôn giữ gói tiện ích cơ bản.
op-pack-kind-conflict = Bản trình bày này đã có gói { $pack } thuộc loại kia (tiện ích hoặc bố cục).
op-theme-cycle = Các chủ đề mở rộng lẫn nhau thành vòng lặp: { $chain }.
op-theme-closure = Các chủ đề được ghim trong bản trình bày này không đúng là chủ đề của nó và các chủ đề cha ({ $namespace }).
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
op-param-shape-path-option = { $nullable ->
    [yes] cần một đường dẫn hình (đối tượng có khóa d) hoặc null
   *[no] cần một đường dẫn hình (đối tượng có khóa d)
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
issue-invalid-style = giá trị kiểu không hợp lệ: { $field } ({ $reason ->
        [valign] { op-value-valign }
        [out-of-bounds] { op-value-out-of-bounds }
       *[other] { op-value-unreadable }
    })
issue-missing-asset = thiếu tài nguyên: { $asset }
issue-pack-version-mismatch = đã loại bỏ phiên bản gói bị thay đổi: { $version }
issue-pack-version-missing = thiếu phiên bản gói trong nguồn: { $version }
issue-pack-pin-mismatch = Gói được nhúng { $pack } không khớp với những gì bản trình bày mong đợi.
issue-pack-tokens-invalid = Gói được nhúng { $pack } có mã thông báo thành phần không hợp lệ: { $reason }
issue-pack-kind-mismatch = Gói được nhúng { $pack } không thuộc loại mà bản trình bày mong đợi (tiện ích hoặc bố cục).
issue-pack-kind-conflict = Không gian tên { $pack } được ghim đồng thời làm gói tiện ích và gói bố cục: chỉ có thể dùng một.
issue-pack-requirement-missing = Gói { $pack } cần { $required }, nhưng bản trình bày này không có.
issue-theme-version-mismatch = đã loại bỏ phiên bản chủ đề bị thay đổi: { $version }
issue-theme-version-missing = thiếu phiên bản chủ đề trong nguồn: { $version }
issue-theme-pin-mismatch = Chủ đề được nhúng { $namespace } không khớp với những gì bản trình bày mong đợi.
issue-theme-closure = Các chủ đề được ghim trong bản trình bày này không đúng là chủ đề của nó và các chủ đề cha ({ $namespace }).
issue-usage-line-invalid = dòng { $line } của nhật ký sử dụng không đọc được, đã bỏ
issue-usage-line-forged = lượt sử dụng { $turn } được ghi hai lần với nội dung khác nhau, đã bỏ một bản
issue-usage-too-large = nhật ký sử dụng quá lớn, đã bỏ
issue-usage-unreadable = nhật ký sử dụng không đọc được, đã bỏ
issue-usage-lines-dropped = { $count ->
   *[other] đã bỏ thêm { $count } dòng của nhật ký sử dụng
}
usage-range-bad-date = “{ $date }” không phải là ngày: hãy dùng YYYY-MM-DD.
usage-range-reversed = Ngày bắt đầu { $since } sau ngày kết thúc { $until }.
pack-token-entry-invalid = Mã thông báo thành phần “{ $path }” không hợp lệ trong tệp kê khai của gói.
pack-manifest-json = Không đọc được tệp kê khai của gói: { $reason }
pack-manifest-order = Thứ tự các mục của gói không hợp lệ tại “{ $name }”.
issue-unsupported-link = liên kết không được hỗ trợ: { $link }
issue-highlight-zero = màu tô sáng được đánh số từ 1
issue-unreadable-element = phần tử không đọc được, đã để riêng
issue-slot-repaired = phần tử không có ô chứa đã biết, được đặt vào { $slot }
issue-unreadable-item = mục danh sách không đọc được, đã để riêng
issue-unreadable-row = hàng bảng không đọc được, đã để riêng
issue-data-truncated = dữ liệu quá lớn, đã cắt bớt khi đọc
issue-unreadable-slide = trang chiếu không đọc được, đã để riêng
issue-unreadable-field = trường { $field } của bản trình bày không đọc được, đã đặt lại
issue-knowledge-unreadable = mục tri thức { $id } không đọc được, đã để riêng
issue-knowledge-entries-dropped = mục tri thức bị loại bỏ mà không đọc: { $count } ({ $cause ->
        [files] quá nhiều tệp để đọc
        [file-bytes] tệp quá lớn để đọc
        [total-bytes] bộ tri thức quá lớn để đọc
       *[unreadable] thêm mục không đọc được
    })
issue-knowledge-over-limit = bộ tri thức vượt quá một giới hạn: { $limit ->
        [knowledge-reason-too-many-files] { knowledge-reason-too-many-files }
        [knowledge-reason-file-too-large] { knowledge-reason-file-too-large }
       *[knowledge-reason-total-too-large] { knowledge-reason-total-too-large }
    } (số tệp liên quan: { $count }, tệp đầu tiên { $id }); mọi tệp đều được giữ, nhưng các lần ghi vượt giới hạn sẽ bị từ chối
issue-knowledge-path-invalid = tệp tri thức { $id } đã được chuyển vào /recovered: { $reason ->
        [knowledge-path-empty] { knowledge-path-empty }
        [knowledge-path-not-absolute] { knowledge-path-not-absolute }
        [knowledge-path-bad-segment] { knowledge-path-bad-segment }
        [knowledge-path-dot-segment] { knowledge-path-dot-segment }
        [knowledge-path-reserved] { knowledge-path-reserved }
        [knowledge-path-too-deep] { knowledge-path-too-deep }
        [knowledge-path-too-long] { knowledge-path-too-long }
       *[knowledge-path-not-markdown] { knowledge-path-not-markdown }
    }
issue-knowledge-path-collision = có nhiều tệp tri thức tại { $path }: các tệp khác đã được đổi tên
issue-knowledge-invalid = tệp tri thức { $path } không hợp lệ: { $reason ->
        [knowledge-frontmatter-missing] { knowledge-frontmatter-missing }
        [knowledge-frontmatter-unterminated] { knowledge-frontmatter-unterminated }
        [knowledge-frontmatter-too-large] { knowledge-frontmatter-too-large }
        [knowledge-frontmatter-not-yaml] { knowledge-frontmatter-not-yaml }
        [knowledge-frontmatter-not-mapping] { knowledge-frontmatter-not-mapping }
        [knowledge-frontmatter-duplicate-key] { knowledge-frontmatter-duplicate-key }
        [knowledge-frontmatter-alias] { knowledge-frontmatter-alias }
        [knowledge-frontmatter-tag] { knowledge-frontmatter-tag }
        [knowledge-frontmatter-too-deep] { knowledge-frontmatter-too-deep }
        [knowledge-frontmatter-after-document-end] { knowledge-frontmatter-after-document-end }
        [knowledge-frontmatter-type-missing] { knowledge-frontmatter-type-missing }
        [knowledge-frontmatter-bad-timestamp] { knowledge-frontmatter-bad-timestamp }
       *[knowledge-frontmatter-not-allowed] { knowledge-frontmatter-not-allowed }
    }
op-invalid-chart = Biểu đồ { $subject }: { $reason }
op-invalid-table = Bảng { $subject }: { $reason }

replica-corrupted = Lịch sử bản trình bày trong tệp này bị hỏng
replica-too-large = Lịch sử bản trình bày trong tệp này quá lớn để mở
replica-write = Không thể cập nhật lịch sử bản trình bày
replica-unrelated = Hai bản trình bày này không có chung nguồn gốc
replica-unsupported-version = Bản trình bày này dùng định dạng mà phiên bản Oku Slides này không đọc được
replica-too-deep = Không thể lưu bản trình bày này: các phần tử đã xóa lồng nhau quá sâu trong lịch sử của nó. Hãy hoàn tác các lần di chuyển gần nhất rồi lưu lại.

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
widget-fault-column-type = Widget { $widget }: cột của danh sách có thể là plain, rich, number, flag, choice hoặc asset (hoặc một p.list lồng nhau, một cấp); với nội dung tự do cho mỗi mục, hãy dùng p.slot với một widget con
widget-fault-reserved-name = Widget { $widget }: tên dành riêng: id, kind hoặc tên chứa /
widget-fault-list-bounds = Widget { $widget }: giá trị tối đa của danh sách thấp hơn giá trị tối thiểu
widget-fault-list-default-too-large = Widget { $widget }: giá trị mặc định của danh sách vượt quá giới hạn kích thước danh sách
widget-fault-list-depth = Widget { $widget }: danh sách lồng nhau không thể chứa cột danh sách; danh sách chỉ lồng được một cấp
widget-fault-list-default-below-min = Widget { $widget }: giá trị mặc định của danh sách lồng nhau có ít mục hơn mức tối thiểu
widget-fault-slide-sizing = Widget { $widget } là một slide: chỉ widget phần tử mới khai báo grow hoặc typography
widget-fault-pack-category = Widget { $widget } không khớp với gói của nó: gói bố cục chứa widget slide, gói widget chứa widget phần tử
widget-fault-unqualified = Widget { $widget } không có không gian tên gói
widget-fault-too-long = Widget { $widget }: một tên, nhãn, giá trị, mô tả hoặc danh sách vượt quá giới hạn
issue-empty-item-id = danh sách { $param } có một mục không có mã định danh
issue-duplicate-item-id = danh sách { $param } có mã định danh mục bị trùng
issue-invalid-item-id = danh sách { $param } có mã định danh mục chứa dấu gạch chéo
issue-list-bounds = danh sách { $param } có quá ít hoặc quá nhiều mục
