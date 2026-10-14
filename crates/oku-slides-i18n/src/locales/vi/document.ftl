pdf-unreadable = Không đọc được PDF
pdf-unsupported-version = PDF này được tạo bởi phiên bản Oku Slides mới hơn
pdf-corrupted = Nguồn trong PDF này bị hỏng
pdf-write = Không thể ghi PDF

render-compile = Không thể kết xuất bản trình bày
render-pdf = Xuất PDF thất bại
render-no-page = Trang này không tồn tại
render-image = Không thể tạo hình ảnh của slide

render-setup = Không thể khởi động trình kết xuất
render-missing-image = thiếu hình ảnh
diagnostic-error = lỗi
diagnostic-warning = cảnh báo
diagnostic = { $level } ({ $place }): { $message }
diagnostic-unplaced = { $level }: { $message }
diagnostic-hint = gợi ý: { $hint }
render-chart-failed = không thể vẽ biểu đồ

registry-bad-pack-id = Định danh gói mẫu không hợp lệ: { $id }
registry-compile = Không thể biên dịch các widget của gói mẫu
registry-missing = Gói mẫu không khai báo widget của nó
render-timeout = Kết xuất mất hơn { $seconds } giây và đã bị dừng
render-out-of-memory = Kết xuất cần hơn { $mib } MiB bộ nhớ và đã bị dừng
render-worker-lost = Tiến trình kết xuất đã dừng
render-frame = Thông điệp không đọc được từ tiến trình kết xuất
render-page-too-large = Trang chiếu này quá lớn để xem trước (hơn { $mib } MiB, thường do một hình ảnh rất lớn). Bạn vẫn có thể xuất nó dưới dạng PDF hoặc PNG.
render-stale = Bản kết xuất đã bị thay thế; hãy thử lại
render-pack-disabled = Các widget của gói “{ $pack }” bị tắt trong phiên này
registry-schema-remote = Sổ đăng ký widget không hợp lệ: { $message }
probe-not-probed = Chưa kiểm tra: đã hết thời gian dành cho các widget của gói. Sẽ kiểm tra ở lần hiển thị tiếp theo.
probe-too-many-files = Gói có hơn { $max } tệp widget: tệp này không được tải.
probe-too-large = Lớn hơn { $max } KiB: không được tải.

import-unreadable = Không thể đọc bản trình bày này
import-not-a-presentation = Tệp này không phải là bản trình bày PowerPoint
import-protected = Bản trình bày này đã được mã hóa hoặc ở định dạng .ppt cũ: hãy lưu thành .pptx trước
import-too-large = Bản trình bày này quá lớn để nhập
import-output-too-large = Bản trình bày thu được sẽ vượt quá kích thước cho phép khi nhập
import-loss-list-levels = Các cấp danh sách được gộp thành một cấp
import-loss-merged-cells = Các ô đã hợp nhất được tách ra
import-loss-percent-stacked = Biểu đồ xếp chồng 100% được hiển thị dạng xếp chồng
import-loss-run-style = { $effect ->
    [underline] Mất gạch chân
    [strike] Mất gạch ngang
    [baseline] Mất chỉ số trên hoặc chỉ số dưới
    [caps] Mất chữ in hoa
    [font] Phông chữ của một đoạn được thay bằng phông của hộp
    [tracking] Khoảng cách chữ của một đoạn được thay bằng khoảng cách của hộp
    [weight] Đoạn thường hiển thị in đậm như hộp
    [slant] Đoạn đứng hiển thị in nghiêng như hộp
   *[highlight] Mất tô sáng
}
import-loss-transform = Mất xoay, lật hoặc cắt xén
import-loss-color-dropped = Màu { $color } không có màu gần giống trong chủ đề: đã dùng màu của chủ đề
import-loss-text-truncated = Văn bản đã được rút gọn để nằm trong giới hạn
import-loss-table-truncated = Bảng đã được rút gọn để nằm trong giới hạn
import-loss-opaque = { $reason ->
    [media] Không nhập video hoặc âm thanh (không có ảnh thay thế)
    [ole] Không nhập đối tượng nhúng (không có ảnh thay thế)
    [math] Không nhập phương trình (không có ảnh thay thế)
    [smartart] Không nhập SmartArt (không có ảnh thay thế)
   *[unknown] Không nhập đối tượng không được hỗ trợ (không có ảnh thay thế)
}
import-loss-unsupported-image = Không nhập ảnh định dạng { $format }
import-loss-hidden-slide = Trang chiếu ẩn được nhập dưới dạng hiển thị
import-loss-effects = Mất bóng đổ, phản chiếu và hiệu ứng 3D
import-loss-series-dropped = Biểu đồ tròn chỉ hiển thị một chuỗi: giữ { $kept }, bỏ { $dropped }
import-loss-repeated-decor = Đã bỏ đầu trang hoặc chân trang “{ $text }” lặp lại trên { $slides } trang chiếu: chủ đề tự vẽ của nó
import-loss-no-slot = Đã bỏ nội dung: mẫu đã chọn không có chỗ cho nó
import-loss-page-fitted = Trang nguồn { $width } × { $height } pt không phải 16:9: nội dung đã được co giãn và căn giữa
import-loss-paragraph-style = Căn lề, giãn dòng hoặc khoảng cách đoạn của đoạn văn được đưa về theo hộp
import-loss-shape-style = { $effect ->
    [gradient] Dải màu được hiển thị thành màu đơn
    [picture] Ảnh tô được kéo giãn theo khung của hình
    [dash] Nét đứt được hiển thị liền
   *[geometry] Hình không xác định được hiển thị thành hình chữ nhật
}
import-loss-layout-dropped = Quá nhiều bố cục cho một gói: “{ $layout }” không có mẫu; các trang chiếu của nó dùng widget trống và nhận phần trang trí dưới dạng phần tử
import-loss-decor-truncated = Phần trang trí bị cắt để vừa kích thước tệp gói: bỏ qua { $omitted } phần tử
import-loss-table-style = Kiểu bảng được ước lượng: nền ô, đường viền, độ rộng cột hoặc cỡ chữ riêng của bảng bị mất
import-font-substituted = Đã thay { $family } bằng { $by }, có số đo tương thích
import-font-missing = Không tìm thấy { $family }: văn bản dùng phông chữ { $by }
import-font-restricted = { $family }: giấy phép cấm nhúng một số kiểu của phông vào bản trình bày
import-font-restricted-substituted = Không thể nhúng { $family } vào bản trình bày (giấy phép): đã thay bằng { $by }, có số đo tương thích
import-font-restricted-missing = Không thể nhúng { $family } vào bản trình bày (giấy phép): văn bản dùng phông chữ { $by }
import-font-too-large = { $family }: tệp quá lớn để nhúng vào bộ trình chiếu; văn bản dùng phông chữ { $by }
import-font-missing-no-fallback = Không tìm thấy { $family } cũng như phông chữ nội dung của chủ đề: văn bản hiển thị bằng phông chữ mặc định của bộ máy
import-warning-effect-ignored = Không nhập hiệu ứng: { $detail }
import-warning-approximated = Hiển thị gần đúng: { $detail }
import-warning-unreadable-item = Đã bỏ qua mục không đọc được: { $detail }
import-warning-unreadable-part = Phần không đọc được: { $detail }
import-warning-missing-part = Thiếu phần: { $detail }
import-warning-unsupported-chart = Biểu đồ không được hỗ trợ: { $detail }
import-warning-unsupported-image = Định dạng ảnh không được hỗ trợ: { $detail }
import-warning-external-image = Ảnh được liên kết bên ngoài tệp, không được nhập: { $detail }
import-warning-unsafe-svg = Ảnh SVG đã được loại bỏ nội dung không an toàn, hoặc bị bỏ qua: { $detail }
import-warning-unknown-table-style = Kiểu bảng không xác định: { $detail }
import-warning-unsafe-link = Liên kết bị từ chối: { $detail }
import-warning-nesting-too-deep = Lồng nhau quá sâu, đã cắt: { $detail }
import-warning-font-embedded-unreadable = Không đọc được phông chữ nhúng: { $detail }
import-warning-truncated = Quá nhiều cảnh báo: các cảnh báo tiếp theo bị bỏ qua
