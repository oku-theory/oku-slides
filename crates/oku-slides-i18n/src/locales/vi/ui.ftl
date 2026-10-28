# Giao diện ứng dụng (React). Được ui/src/i18n.ts tải qua yêu cầu « messages ».
ui-loading = Đang tải…

## Tài liệu

ui-brand = Oku Slides
ui-brand-pack = { $kind ->
    [layouts] { $name } · gói bố cục
   *[other] { $name } · gói widget
}
ui-title-rename = Đổi tên bản trình bày
ui-title-field = Tiêu đề bản trình bày
ui-status-unsaved = có thay đổi chưa lưu
ui-status-saved = đã lưu
ui-status-never-saved = chưa lưu
ui-caption-pack = { $kind ->
    [layouts] { $count ->
       *[other] { $count } bố cục
    }
   *[other] { $count ->
       *[other] { $count } widget
    }
} · { $status }
ui-caption-slides = { $count ->
   *[other] { $count } trang chiếu
} · { $status }
ui-confirm-quit-pack = Thoát mà không lưu gói đã sửa?
ui-confirm-quit-theme = Thoát mà không lưu chủ đề đã sửa?
ui-confirm-quit-read-only-theme = Không thể lưu { $theme } ở đây. Đóng thẻ này và mất các thay đổi của nó?
ui-confirm-update-opener = Cập nhật “{ $deck }” lên phiên bản này của gói “{ $pack }”?
ui-close-writes-pending = Thẻ này vẫn đang lưu: hãy đóng lại sau giây lát.
ui-confirm-close-writes-pending = Thẻ này vẫn đang lưu. Vẫn đóng và mất những gì chưa được lưu?
ui-confirm-quit-read-only-pack = Không thể lưu { $pack } ở đây. Đóng thẻ này và mất các thay đổi của nó? Để giữ lại, hãy nhờ tác nhân AI lưu nó thành một gói mới.

## Thông báo

ui-toast-copied = Đã lưu bản sao
ui-toast-field-gone = Trường bạn đang sửa không còn nữa: nội dung đã nhập bị mất.
ui-toast-edit-replaced = Văn bản này đã thay đổi trong lúc bạn sửa: phiên bản của bạn đã thay thế nó. Hoàn tác sẽ khôi phục phiên bản kia.

## Thanh công cụ

ui-toolbar-undo = Hoàn tác
ui-toolbar-undo-label = Hoàn tác: { $label }
ui-toolbar-redo = Làm lại
ui-toolbar-redo-label = Làm lại: { $label }
ui-divider-pile = Độ rộng chồng trang chiếu
ui-divider-notes = Độ rộng luồng của tác nhân AI
ui-divider-code = Kích thước mã
ui-toolbar-theme = Chủ đề
ui-toolbar-finish = Xong
ui-pack-deck-only = Chỉ được lưu trong bài trình bày này
ui-toolbar-open = Mở
ui-menu = Menu
ui-toolbar-add-label = Thêm vào trang chiếu
ui-toolbar-packs = Gói của bản trình bày
ui-toolbar-packs-update = Gói của bản trình bày — có bản cập nhật
ui-toolbar-thumbs-show = Hiện hình thu nhỏ
ui-toolbar-thumbs-hide = Ẩn hình thu nhỏ
ui-toolbar-remove-slide = Xóa trang chiếu
ui-toolbar-remove-hint = Trang chiếu { $n } · hoàn tác bằng ⌘Z

## Nhãn hoàn tác

ui-undo-add-widget = Thêm “{ $name }”
ui-undo-new-slide = Trang chiếu mới: { $name }
ui-undo-move-slide = Di chuyển trang chiếu
ui-undo-remove-slide = Xóa trang chiếu
ui-undo-remove = Xóa
ui-undo-paste = Dán
ui-undo-place-on-grid = Đặt lên lưới
ui-undo-move-on-grid = Di chuyển trên lưới
ui-undo-resize-on-grid = Đổi kích thước trên lưới
ui-undo-change-anchor = Đổi điểm neo
ui-undo-move = Di chuyển
ui-undo-resize = Đổi kích thước
ui-undo-edit-slide = Sửa trang chiếu
ui-undo-rename-pack = Đổi tên gói
ui-undo-rename-deck = Đổi tên bản trình bày

## Danh sách trang chiếu

ui-pile-slides = Trang chiếu
ui-pile-layouts = Bố cục
ui-pile-add-slide = Trang chiếu mới
ui-pile-drag-hint = Kéo để di chuyển · ⌥↑ ⌥↓
ui-slide-alt = Trang chiếu { $n }

## Vùng làm việc

ui-stage-rendering = Đang kết xuất…
ui-stage-widget-pack = Mở một widget từ chồng, hoặc tạo widget mới.
ui-stage-elements = Các phần tử của trang chiếu
ui-stage-fields = Các trường của trang chiếu
ui-stage-edit-text = Sửa văn bản
ui-stage-hint-plain = Enter hoặc Tab để sang mục tiếp · Esc để hủy
ui-stage-hint-rich = Tab để sang mục tiếp · ⌘ Enter để xác nhận · “- ” cho gạch đầu dòng · ⌘B đậm, ⌘I nghiêng, ⌘K liên kết
ui-field = { $kind ->
    [title] Tiêu đề
    [subtitle] Phụ đề
    [kicker] Dòng dẫn
    [text] Văn bản
    [body] Nội dung
    [author] Tác giả
    [date] Ngày
    [organization] Tổ chức
   *[other] { $kind }
}
ui-element-kind = { $kind ->
    [text] Văn bản
    [card] Thẻ
    [image] Hình ảnh
    [shape] Hình
    [chart] Biểu đồ
    [table] Bảng
    [stack] Ngăn xếp
   *[other] { $kind }
}
ui-grid-cell = cột { $cols } · hàng { $rows }
ui-ghost-zone = Vùng

## Ghi chú trang chiếu

ui-notes = Công cụ trang chiếu
ui-stepper-less = { $label }: bớt một
ui-stepper-more = { $label }: thêm một
ui-list-item = mục
ui-list-add = Thêm { $item }
ui-list-remove = Xóa { $item }
ui-list-move = Di chuyển { $item }
ui-list-full = Danh sách đã đầy ({ $max })
ui-undo-list-move = Di chuyển { $item }
ui-agent-color-other = Màu khác
ui-agent-thread = Tác nhân AI
ui-agent-new-conversation = Cuộc trò chuyện mới
ui-agent-undo-turn = Hoàn tác lượt này
ui-agent-detach = Mở tác nhân trong cửa sổ riêng
ui-agent-attach = Đưa tác nhân trở lại cửa sổ chính
ui-agent-window-empty = Không có tác nhân cho thẻ này
ui-agent-kept-in-draft = Không có chỗ để ghi: văn bản được giữ trong bản nháp
ui-draft = Bản nháp
ui-draft-send = Gửi cho tác nhân AI
ui-draft-send-title = Gửi cho tác nhân AI (⌘↩)
ui-draft-discard = Bỏ
ui-pasted-image = { $type } { $width }×{ $height }
ui-draft-images = Ảnh đã dán
ui-draft-remove-image = Xóa ảnh này
ui-draft-send-needs-text = Hãy nói cần làm gì với ảnh này
ui-agent-state-idle = Tác nhân AI
ui-agent-state-starting = Tác nhân AI đang khởi động…
ui-agent-state-working = Tác nhân AI đang làm việc…
ui-agent-dock = Đang thực hiện
ui-agent-dock-settings = Cài đặt
ui-agent-jump = ↓ Lượt hiện tại
ui-agent-tool-input = Đối số
ui-agent-tool-output = Kết quả
ui-agent-sheet = Câu hỏi của tác nhân AI
ui-agent-sheet-keys = 1–4 chọn · Enter xác nhận · E chỉnh sửa · Esc hủy
ui-agent-variants = Biến thể
ui-agent-drawer-keys = Nhấp hoặc 1–4 · Esc đóng
ui-agent-confirm = Xác nhận của tác nhân AI

## Đọc chính tả

ui-dictation = Đọc chính tả
ui-dictation-start = Đọc chính tả
ui-dictation-start-title = Đọc chính tả (⌘D)
ui-dictation-type = Viết cho tác nhân AI
ui-dictation-type-title = Viết cho tác nhân AI (⌘K)
ui-dictation-agent-stop = Dừng tác nhân AI (⌘⇧D hoặc Esc)
ui-dictation-agent-working = Tác nhân AI đang làm việc…
ui-dictation-agent-working-hint = ⌘⇧D hoặc Esc để dừng
ui-dictation-finish = Kết thúc đọc chính tả
ui-dictation-finish-title = Kết thúc (⌘D)
ui-dictation-cancel = Hủy
ui-dictation-preparing = Đang chuẩn bị đọc chính tả…
ui-dictation-heard-nothing = Không nghe thấy gì: hãy nói gần micrô hơn một chút
ui-dictation-to-cursor = tại con trỏ
ui-dictation-to-draft = vào bản nháp
ui-dictation-downloading = Đang tải xuống mô hình { $name }
ui-dictation-download-progress = { $percent }% · { $received } / { $total } · chỉ một lần
ui-dictation-listening = Đang nghe · { $time }
ui-dictation-transcribing = Đang chuyển câu cuối thành văn bản…
ui-dictation-stop-in = Dừng sau { $duration }
ui-dictation-speak = Hãy nói, văn bản sẽ xuất hiện ở đây…
ui-dictation-auto-stop = dừng sau { $duration } im lặng
ui-dictation-to-download = { $size } cần tải xuống
ui-dictation-settings = Cài đặt
ui-dictation-language-auto = Tự động nhận diện
ui-dictation-model-unavailable = Ngôn ngữ không được hỗ trợ
ui-dictation-language-auto-resolved = Tự động ({ $language })
ui-dictation-to-download-language = Ngôn ngữ cần tải xuống
ui-dictation-api-key-missing = Chưa đặt { $var }
ui-seconds = { NUMBER($value, maximumFractionDigits: 1) } giây
ui-size-gb = { NUMBER($value, maximumFractionDigits: 1) } GB
ui-size-mb = { $value } MB

## Bộ chọn mẫu

ui-picker-new-slide = Trang chiếu mới
ui-picker-set-mine = { $name } · của tôi
ui-picker-set-received = { $name } · đã nhận
ui-picker-keys = ← → để duyệt · Enter để thêm · Esc để đóng
ui-picker-close = Đóng
ui-picker-list = Bố cục
ui-widget-picker-filter = Lọc theo tên, nhãn hoặc mô tả
ui-widget-picker-empty = Không có widget nào khớp với “{ $query }”.
ui-widget-picker-list = Widget
ui-widget-picker-keys = Phím mũi tên để duyệt · Enter để thêm · Esc để đóng

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
ui-chart-add-row = Hàng
ui-chart-add-column = Cột
ui-chart-series = Chuỗi { $n }

## Bảng

ui-table-panel = Bảng
ui-table-options = Tùy chọn bảng
ui-table-header-row = Hàng tiêu đề
ui-table-header-column = Cột đầu tiên làm tiêu đề
ui-table-striped = Hàng xen kẽ
ui-table-alignment = Căn lề cột
ui-table-align-of = Căn lề của { $name }
ui-table-align = { $align ->
    [start] Đầu
    [center] Giữa
    [end] Cuối
   *[auto] Tự động
}
ui-table-column = Cột { $n }
ui-table-no-data = Bảng này không có dữ liệu.
ui-show-as = Hiển thị dưới dạng
ui-show-as-table = Hiển thị dưới dạng bảng
ui-show-as-chart = Hiển thị dưới dạng biểu đồ

## Tệp

ui-file-open-prompt = Đường dẫn tệp cần mở (.oku-slides.pdf, .json hoặc .pptx)
ui-file-filter-presentations = Bài trình bày
ui-file-filter-pptx = PowerPoint
ui-file-save-prompt = Lưu thành (đường dẫn đầy đủ)

## Thẻ

ui-tabs = Bản trình bày đang mở
ui-tab-new = Thẻ mới
ui-tab-close = Đóng thẻ
ui-tab-unsaved = Thay đổi chưa lưu
ui-tab-pack = Gói
ui-tab-theme = Chủ đề
ui-tab-move-new-window = Chuyển sang cửa sổ mới
ui-tab-move-to = Chuyển sang cửa sổ { $n }
ui-tab-close-others = Đóng các thẻ khác
ui-close-cancel = Hủy
ui-disk-replaced-question = { $file } đã bị thay trên ổ đĩa bằng một bản trình bày khác.
ui-disk-replaced-overwrite = Ghi đè
ui-disk-replaced-save-as = Lưu thành…
ui-same-deck-question = { $file } là một phiên bản của bản trình bày đang mở trong thẻ “{ $title }”.
ui-same-deck-merge = Hợp nhất vào thẻ đó
ui-same-deck-apart = Mở riêng
ui-history = Lịch sử
ui-history-version = Phiên bản lúc { $date }
ui-history-restore = Khôi phục
ui-history-back = Quay lại
ui-history-anonymous = Ẩn danh
ui-history-slider = Các phiên bản của bản trình bày
ui-history-removed-since = đã xóa từ đó
ui-history-changed-since = đã thay đổi từ đó
ui-history-added-since = số trang chiếu thêm từ đó: { $count }
ui-help = Trợ giúp
ui-help-caption = Trang { $index } / { $count } · chỉ đọc
ui-slide-previous = Trang trước
ui-slide-next = Trang sau
ui-present = Bản trình bày
ui-present-slide = Trang chiếu { $index } / { $count }
ui-present-screen = Trình chiếu toàn màn hình
ui-help-close = Đóng trợ giúp
ui-help-search = Tìm trong trợ giúp
ui-help-search-placeholder = Tìm…
ui-help-search-empty = Không có kết quả
ui-help-search-count = { $count } kết quả
ui-widgets-files = Tệp widget
ui-widgets-common = Dùng chung
ui-widgets-new = Widget mới
ui-widgets-name = Tên widget (chữ thường, chữ số và dấu gạch nối)
ui-widgets-name-taken = Tên này đã được dùng
ui-widgets-name-letter = Tên phải bắt đầu bằng một chữ cái
ui-widgets-valid = Hợp lệ
ui-widgets-errors = Lỗi
ui-widgets-empty = Gói này chưa có widget nào.
ui-widgets-preview = Xem trước widget
ui-widgets-no-preview = Không có bản xem trước
ui-widgets-field-bound = Liên kết với bản trình bày: chỉ đọc ở đây
ui-widgets-source = Mã nguồn
ui-widgets-unsaved = Chưa lưu
ui-widgets-in-error = Widget có lỗi
ui-widgets-none-in-error = Không widget nào có lỗi
ui-widgets-section = Widget
ui-widgets-open = Mở widget { $name }
ui-widgets-delete-named = Xóa { $name }
ui-widgets-confirm-delete = Xóa widget “{ $name }”?
ui-widgets-rename-hint = Nhấp đúp hoặc nhấn F2 để đổi tên
ui-pack-tabs = Tab của gói
ui-pack-layouts-tab = Bố cục
ui-theme-name = Tên chủ đề
ui-theme-unsaved = Thay đổi chủ đề chưa lưu
ui-widgets-close-tab = Đóng { $name }
ui-widgets-removed-by-agent = Tác nhân AI đã xóa widget { $name }.

## Nhập

ui-import-title = Nhập bài trình bày PowerPoint
ui-import-mode = Chế độ nhập
ui-import-hifi = Bản sao trung thực
ui-import-hifi-note = Giữ giao diện của bản gốc, với chủ đề và bố cục được tạo cho bài trình bày này.
ui-import-retrofit = Dùng bố cục của tôi
ui-import-retrofit-note = Dựng lại các trang chiếu bằng các gói bố cục và một chủ đề trong thư viện.
ui-import-layout-packs = Gói bố cục
ui-import-theme-default = Chủ đề mặc định
ui-import-no-packs = Thư viện không có gói bố cục nào.
ui-import-tune = Tinh chỉnh với tác nhân AI
ui-import-tune-note = Tác nhân AI so sánh từng trang chiếu với bản gốc và sửa các chỗ lệch.
ui-import-tune-note-retrofit = Tác nhân AI so sánh từng trang chiếu với bản gốc và thiết kế widget cho các hình mà bố cục của bạn không thể hiện được.
ui-import-tune-unavailable = Không có tác nhân AI nào dùng được: { $reason }
ui-import-submit = Nhập
ui-import-busy = Đang nhập…
ui-update-available = Oku Slides { $version } đã có sẵn.
ui-update-install = Cài đặt và khởi động lại
ui-update-see-release = Xem bản phát hành
ui-update-later = Để sau
ui-update-installing = Đang tải bản cập nhật… { $percent }%
ui-update-downloading = Đang tải bản cập nhật…
ui-update-up-to-date = Oku Slides đã là bản mới nhất.
ui-update-check = Kiểm tra bản cập nhật…
ui-about = Giới thiệu Oku Slides
ui-about-version = Phiên bản { $version }
ui-about-changelog = Có gì mới
ui-about-breaking = không tương thích
ui-about-close = Đóng

ui-shortcut-theme = chủ đề, giao diện, theme, themes
