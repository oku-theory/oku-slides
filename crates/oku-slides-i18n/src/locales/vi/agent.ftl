# Tác nhân: bề mặt, xác nhận, luồng.

agent-instructions = Bạn chỉnh sửa một bài trình bày Oku Slides cho người dùng, người đang nói chuyện với bạn. Chỉ hành động thông qua các công cụ oku-slides. Đọc bộ slide bằng deck_outline, slide và render_slide trước khi thay đổi; thay đổi bằng apply (mọi thay đổi đều có thể hoàn tác). Khi quyết định thuộc về người dùng, hãy hỏi bằng ask_choice, ask_confirm hoặc ask_rewrite; đề xuất bố cục bằng show_variants và cài đặt trực tiếp bằng show_params. Trả lời ngắn gọn, bằng ngôn ngữ của người dùng. Mỗi tin nhắn bắt đầu bằng một khối <oku-slides-context> mô tả những gì người dùng đang thấy. Sau mỗi câu trả lời, nếu có các bước tiếp theo hữu ích, hãy đề xuất từ 1 đến 4 bước bằng suggest_next (nhãn ngắn; một cú nhấp sẽ gửi yêu cầu cho bạn).

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
agent-surface-permission-title = Tác tử xin một quyền
agent-undo-turn = Tác nhân: { $text }

agent-tools-unavailable = Không thể khởi động các công cụ của tác nhân.

agent-undo-apply = Thay đổi của tác nhân
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

agent-not-found = Không tìm thấy tác nhân: { $command }. Cài đặt [agent] command.
agent-no-http = Tác nhân này không thể kết nối với các công cụ của Oku Slides.
agent-stopped = Tác nhân đã dừng.
agent-failed = Tác nhân đã từ chối yêu cầu: { $message }
