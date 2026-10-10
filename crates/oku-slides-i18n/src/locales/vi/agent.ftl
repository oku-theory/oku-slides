# Tác nhân AI: bề mặt, xác nhận, luồng.

agent-surface-cancel = Hủy
agent-surface-confirm = Xác nhận
agent-surface-confirm-danger = Có, thực hiện
agent-surface-rewrite = Diễn đạt lại
agent-surface-pick = Chọn { $n }
agent-surface-keep = Giữ bản gốc
agent-surface-edit = Chỉnh sửa trước khi áp dụng
agent-surface-close = Đóng
agent-surface-variants = Các biến thể
agent-surface-permission-always = vĩnh viễn
agent-surface-permission-title = Tác nhân AI xin một quyền
agent-undo-turn = Tác nhân AI: { $text }

agent-tools-unavailable = Không thể khởi động các công cụ của tác nhân AI.

agent-undo-apply = Thay đổi của tác nhân AI
agent-undo-variant = Biến thể: { $label }
agent-undo-param = Cài đặt: { $label }
agent-undo-rewrite = Diễn đạt lại

agent-confirm-save = Lưu bài trình bày dưới dạng { $path }?
agent-confirm-save-over = Thay thế { $path }? Tệp hiện có sẽ bị ghi đè.
agent-confirm-open = Mở { $path }?
agent-open-elsewhere = Tệp này đã được mở trong một thẻ khác.
agent-confirm-new = Bắt đầu bài trình bày mới?
agent-confirm-new-dirty = Bắt đầu bài trình bày mới? Các thay đổi chưa lưu sẽ bị mất.
agent-confirm-save-templates = Lưu bộ mẫu “{ $name }”?
agent-confirm-delete-set = Xóa bộ mẫu “{ $name }”? Không thể hoàn tác thao tác này.
agent-confirm-setting = Thay đổi { $key }: { $old } → { $new }?

agent-not-found = Không tìm thấy tác nhân AI: { $command }. Cài đặt [agent] command.
agent-no-http = Tác nhân AI này không thể kết nối với các công cụ của Oku Slides.
agent-stopped = Tác nhân AI đã dừng.
agent-failed = Tác nhân AI đã từ chối yêu cầu: { $message }
agent-tool-failed = Công cụ gặp lỗi ngoài dự kiến.

agent-turn-tools = { $tools } công cụ
agent-turn-notes = { $notes } ghi chú
agent-turn-secs = { $secs } giây
agent-turn-stopped = Đã dừng.
agent-turn-interrupted = Đã bị gián đoạn.
agent-memory-lost = Tác nhân không còn nhớ cuộc trò chuyện này: nó bắt đầu lại từ đầu.
agent-turn-max-tokens = Tác nhân AI đã đạt giới hạn độ dài.
agent-turn-max-turns = Tác nhân AI đã đạt giới hạn số bước.
agent-turn-refusal = Tác nhân AI đã từ chối.
agent-turn-ended = Kết thúc lượt ({ $stop }).
agent-choice-cancelled = Câu hỏi đã bị hủy
agent-choice-answered = ✓ { $answer }
agent-report-why = Tại sao

agent-error-file = Không thể đọc hoặc ghi tệp.
agent-error-unexpected-action = Hành động không mong đợi: { $action }.
agent-error-unknown-surface = Thẻ này không còn mở.
agent-error-expired = Các hành động của một lượt trước đã hết hạn.
agent-error-incomplete = Hành động của thẻ này chưa đầy đủ.
agent-error-no-field = Cài đặt này không còn tồn tại.
agent-error-no-variant = Biến thể này không còn tồn tại.
agent-error-arguments = Đối số của công cụ không hợp lệ ({ $detail }): hãy kiểm tra theo lược đồ của nó.
agent-error-unknown-tool = Công cụ không xác định: { $name }.
agent-error-denied = Người dùng đã từ chối.
agent-error-no-turn = Không có lượt nào đang diễn ra: { $tool } chỉ được hiển thị trong một lượt.
agent-error-turn-over = Lượt của lệnh gọi này đã kết thúc: không có gì được hiển thị.
agent-error-anchor-scope = Câu hỏi trong cuộc trò chuyện về tiện ích không thể neo vào trang chiếu: hãy hỏi mà không neo.
agent-error-no-file = Bản trình bày chưa có tệp: hãy cung cấp đường dẫn tuyệt đối.
agent-error-path-relative = Đường dẫn phải là đường dẫn tuyệt đối.
agent-error-path-parent = Đường dẫn không được chứa “..”.
agent-error-setting-locked = Tác nhân không thể thay đổi cài đặt { $key }.
agent-error-setting = Không thể thay đổi cài đặt { $key }: hãy kiểm tra giá trị của nó.
agent-error-rewrite-text = Bản viết lại trả về mà không có văn bản.
agent-error-paragraphs = Các đoạn của bản viết lại không hợp lệ.
agent-error-draft-too-long = Bản nháp quá dài.
agent-error-draft-origin = Bản nháp bị từ chối: không rõ chế độ xem.
agent-error-draft-no-agent = Tác nhân của thẻ này chưa khởi động: bản nháp bị từ chối.
