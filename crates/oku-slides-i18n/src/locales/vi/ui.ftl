# Giao diện ứng dụng (React). Được ui/src/i18n.ts tải qua yêu cầu « messages ».
ui-loading = Đang tải…

## Tài liệu

ui-brand = Oku Slides
ui-brand-templates = Oku Slides · mẫu
ui-title-rename = Đổi tên bản trình bày
ui-title-field = Tiêu đề bản trình bày
ui-status-unsaved = có thay đổi chưa lưu
ui-status-saved = đã lưu
ui-status-never-saved = chưa lưu
ui-status-builtin = có sẵn: lưu sẽ tạo một bản sao
ui-caption-templates = { $count ->
   *[other] { $count } mẫu
} · { $status }
ui-caption-slides = { $count ->
   *[other] { $count } trang chiếu
} · { $status }
ui-template-default-name = Mẫu
ui-template-variant-name = { $name } (biến thể)
ui-confirm-quit-templates = Thoát mà không lưu các mẫu đã sửa?

## Thông báo

ui-toast-saved = Đã lưu bản trình bày của bạn
ui-toast-templates-saved = Đã lưu mẫu: { $name }
ui-toast-select-zone = Chọn vùng để đặt: “{ $text }”

## Thanh công cụ

ui-toolbar-undo = Hoàn tác
ui-toolbar-undo-label = Hoàn tác: { $label }
ui-toolbar-redo = Làm lại
ui-toolbar-redo-label = Làm lại: { $label }
ui-toolbar-grid-hide = Ẩn lưới
ui-toolbar-grid-show = Hiện lưới
ui-toolbar-grid-hide-title = Ẩn lưới (⌘')
ui-toolbar-grid-show-title = Hiện lưới (⌘')
ui-toolbar-theme = Chủ đề
ui-toolbar-theme-default = Mặc định
ui-toolbar-save-copy = Lưu bản sao
ui-toolbar-save = Lưu
ui-toolbar-finish = Xong
ui-toolbar-templates = Mẫu
ui-toolbar-templates-title = Tạo và chỉnh sửa mẫu trang chiếu
ui-toolbar-open = Mở
ui-toolbar-add-label = Thêm vào trang chiếu
ui-toolbar-add = Thêm
ui-add-kind = { $kind ->
    [card] Thẻ
    [shape] Hình
    [chart] Biểu đồ
   *[text] Văn bản
}
ui-toolbar-remove-template = Xóa mẫu
ui-toolbar-remove-slide = Xóa trang chiếu
ui-toolbar-remove-hint = Trang chiếu { $n } · hoàn tác bằng ⌘Z

## Nhãn hoàn tác

ui-undo-add = { $kind ->
    [card] Thêm thẻ
    [shape] Thêm hình
    [chart] Thêm biểu đồ
   *[text] Thêm văn bản
}
ui-undo-new-template = Mẫu mới: { $name }
ui-undo-new-slide = Trang chiếu mới: { $name }
ui-undo-grid = Lưới { $columns } × { $rows }
ui-undo-templates = Mẫu: { $name }
ui-undo-move-template = Di chuyển mẫu
ui-undo-move-slide = Di chuyển trang chiếu
ui-undo-remove-slide = Xóa trang chiếu
ui-undo-remove = Xóa
ui-undo-cut = Cắt
ui-undo-paste = Dán
ui-undo-place-on-grid = Đặt lên lưới
ui-undo-move-on-grid = Di chuyển trên lưới
ui-undo-resize-on-grid = Đổi kích thước trên lưới
ui-undo-change-anchor = Đổi điểm neo
ui-undo-move = Di chuyển
ui-undo-resize = Đổi kích thước
ui-undo-edit-text = Sửa văn bản
ui-undo-dictation = Đọc chính tả
ui-undo-rename-set = Đổi tên bộ mẫu
ui-undo-rename-deck = Đổi tên bản trình bày
ui-undo-theme = Đổi chủ đề
ui-undo-rename-template = Đổi tên mẫu
ui-undo-describe-template = Mô tả mẫu
ui-undo-chart-kind = Đổi loại biểu đồ
ui-undo-chart-data = Sửa dữ liệu
ui-undo-chart-color = Tô màu một chuỗi
ui-undo-chart-stack = { $stacked ->
    [yes] Xếp chồng các chuỗi
   *[no] Tách các chuỗi
}

## Phần tử mới

ui-new-text = Văn bản mới
ui-new-card-title = Tiêu đề thẻ
ui-new-card-detail = Chi tiết

## Danh sách trang chiếu

ui-pile-slides = Trang chiếu
ui-pile-templates = Mẫu
ui-pile-add-slide = + Trang chiếu mới
ui-pile-add-template = + Mẫu mới
ui-pile-drag-hint = Kéo để di chuyển · ⌥↑ ⌥↓
ui-slide-alt = Trang chiếu { $n }

## Vùng làm việc

ui-stage-rendering = Đang kết xuất…
ui-stage-elements = Các phần tử của trang chiếu
ui-stage-edit-text = Sửa văn bản
ui-stage-hint-field = Enter để xác nhận · Esc để hủy
ui-stage-hint-block = ⌘ Enter để xác nhận · mỗi đoạn một dòng, “- ” cho gạch đầu dòng
ui-field = { $kind ->
    [title] Tiêu đề
    [subtitle] Phụ đề
    [kicker] Dòng dẫn
    [text] Văn bản
   *[other] { $kind }
}
ui-element-kind = { $kind ->
    [text] Văn bản
    [card] Thẻ
    [image] Hình ảnh
    [shape] Hình
    [group] Nhóm
   *[other] { $kind }
}
ui-grid-cell = cột { $cols } · hàng { $rows }

## Ghi chú trang chiếu

ui-notes = Công cụ trang chiếu
ui-stepper-less = { $label }: bớt một
ui-stepper-more = { $label }: thêm một

## Đọc chính tả

ui-dictation = Đọc chính tả
ui-dictation-start = Đọc chính tả
ui-dictation-start-title = Đọc chính tả (⌘D)
ui-dictation-finish = Kết thúc đọc chính tả
ui-dictation-finish-title = Kết thúc (⌘D)
ui-dictation-cancel = Hủy
ui-dictation-preparing = Đang chuẩn bị đọc chính tả…
ui-dictation-heard-nothing = Không nghe thấy gì: hãy nói gần micrô hơn một chút
ui-dictation-to-cursor = tại con trỏ
ui-dictation-to-field = thay thế { $field }
ui-dictation-to-block = sau khối đã chọn
ui-dictation-new-block = khối văn bản mới
ui-dictation-select-zone = chọn một vùng
ui-dictation-downloading = Đang tải xuống mô hình { $name }
ui-dictation-download-progress = { $percent }% · { $received } / { $total } · chỉ một lần
ui-dictation-listening = Đang nghe · { $time }
ui-dictation-transcribing = Đang chuyển câu cuối thành văn bản…
ui-dictation-stop-in = Dừng sau { $duration }
ui-dictation-speak = Hãy nói, văn bản sẽ xuất hiện ở đây…
ui-dictation-auto-stop = dừng sau { $duration } im lặng
ui-dictation-to-download = { $size } cần tải xuống
ui-dictation-settings = Cài đặt đọc chính tả
ui-dictation-model-title = Mô hình nhận dạng
ui-dictation-model-intro = Whisper, chạy trên máy tính này (CTranslate2). Mọi mô hình đều hiểu nhiều ngôn ngữ.
ui-dictation-note-turbo = Khuyên dùng, nhanh và chính xác
ui-dictation-note-large = Chính xác nhất, chậm hơn
ui-dictation-note-base = Nhẹ, cho máy cấu hình thấp
ui-dictation-installed = đã cài đặt
ui-dictation-not-installed = cần tải xuống
ui-dictation-language = Ngôn ngữ nói
ui-dictation-language-auto = Tự động nhận diện
ui-dictation-auto-stop-label = Tự dừng sau khoảng im lặng
ui-dictation-silence = Thời lượng im lặng
ui-dictation-phrases-note = Chỉ những đoạn có tiếng nói mới được chuyển thành văn bản, từng câu một, trong khi bạn nói.
ui-dictation-download-now = Tải xuống ngay ({ $size })
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } giây
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } GB
ui-size-mb = { $value } MB

## Bộ chọn mẫu

ui-picker-new-slide = Trang chiếu mới
ui-picker-new-template = Mẫu mới, dựa trên…
ui-picker-set = Mẫu
ui-picker-set-mine = { $name } · của tôi
ui-picker-keys = ← → để duyệt · Enter để thêm · Esc để đóng
ui-picker-close = Đóng
ui-picker-previous = Mẫu trước
ui-picker-next = Mẫu tiếp theo
ui-picker-list = Mẫu trang chiếu

## Biểu đồ

ui-chart-panel = Biểu đồ
ui-chart-kinds = Loại biểu đồ
ui-chart-kind = { $kind ->
    [line] Đường
    [pie] Tròn
    [scatter] Phân tán
    [radar] Radar
    [waterfall] Thác nước
   *[bar] Cột
}
ui-chart-no-data = Biểu đồ này không có bảng dữ liệu.
ui-chart-options = Tùy chọn biểu đồ
ui-chart-stack = Xếp chồng
ui-chart-waterfall-hint = Đánh dấu Tổng để có cột tổng.
ui-chart-colors = Màu của chuỗi
ui-chart-color-of = Màu của { $name }
ui-chart-color-auto = Tự động
ui-chart-color-pick = Màu { $n }
ui-chart-column-name = Tên cột { $col }
ui-chart-remove-column = Xóa cột
ui-chart-cell = Hàng { $row }, cột { $col }
ui-chart-remove-row = Xóa hàng
ui-chart-add-row = + Hàng
ui-chart-add-column = + Cột
ui-chart-series = Chuỗi { $n }

## Tệp

ui-file-open-prompt = Đường dẫn tệp cần mở (.oku-slides.pdf hoặc .json)
ui-file-save-prompt = Lưu thành (đường dẫn đầy đủ)
