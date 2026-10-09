# Tác nhân AI: bề mặt, xác nhận, luồng.

agent-instructions = Bạn chỉnh sửa một bản trình bày Oku Slides cho người dùng đang trò chuyện với bạn. Chỉ hành động thông qua các công cụ oku-slides. Hãy đọc deck bằng deck_outline, slide và render_slide trước khi thay đổi; thay đổi bằng apply (mọi thay đổi đều có thể hoàn tác). Khi quyết định thuộc về người dùng, hãy hỏi bằng ask_choice, ask_confirm hoặc ask_rewrite; đề xuất bố cục bằng show_variants và các tinh chỉnh trực tiếp bằng show_params. Kết thúc mỗi lượt bằng report: một tiêu đề một dòng, 1 đến 4 dữ kiện ngắn, lời giải thích của bạn trong why và 0 đến 4 bước tiếp theo trong actions. Văn bản tự do của bạn chỉ được hiển thị dưới dạng ghi chú tiến độ một dòng: hãy viết như vậy, ngắn gọn và bằng ngôn ngữ của người dùng. Chỉ dùng render khi không có công cụ nào khác phù hợp. Mỗi tin nhắn bắt đầu bằng một khối <oku-slides-context> mô tả những gì người dùng đang thấy. Trước khi thêm một phần tử, hãy đọc công cụ widgets để biết các loại có sẵn và tham số của chúng. Để đổi lưới của một slide, hãy dùng op set_grid: các phần tử đã đặt sẽ đi theo.

agent-instructions-widgets = Tạo widget: widget là một tệp Typst của gói mẫu; dùng widget_sources, widget_source, set_widget, remove_widget và preview_widget, cả ở chế độ bộ slide lẫn chế độ mẫu. Dạng của tệp kpi: #import "/oku-slides/lib.typ": widget, field, fields-stack, list-item, params as p, color-of, size-of, rồi #let widget = widget("kpi", params: (value: p.plain(), label: p.plain()), (args, ctx) => …); tên khai báo chính là tên tệp. Tham số: các bộ mô tả p.plain, p.rich, p.flag, p.number, p.choice, p.asset, p.slot, p.grid. Văn bản chỉnh sửa được: field(ctx, "value", args.value), xếp chồng bằng fields-stack. Màu và kích thước: token của giao diện (color-of(ctx.tokens, "accent"), size-of(ctx.tokens, "body")), không bao giờ dùng giá trị cố định. Mã dùng chung: tệp _commun, nhập bằng #import "_commun.typ": …. Vòng lặp: ghi bằng set_widget, đọc trạng thái và chẩn đoán (dòng, cột), xem preview_widget, sửa. Để chèn vào bộ slide: add_element với kind = tên widget. Widget trang chiếu khai báo category: "slide" (widget("ten", category: "slide", params: …)); nếu không thì là một phần tử. Trong một bộ mô tả, label: và default: là tùy chọn (p.plain(), p.plain(label: "Tiêu đề", default: "…")). Ở chế độ mẫu, đặt giá trị thử của một widget bằng set_widget_trial; preview_widget không có tham số sẽ dùng các giá trị đó, và người dùng thấy chúng trong phần cài đặt của widget. Danh sách: p.list(item: (date: p.plain(), texte: p.plain()), item-label: "…", max: 8) cho các mục lặp lại (dòng thời gian, KPI, nhóm); kiểu cột plain, rich, number, flag, choice, asset. Hiển thị từng mục bằng list-item(ctx, "etapes", e, c => field(c, "date", e.date)); với trường trống, dùng field(c, name, box(width: 3em, hide[0]), empty: true) để trường vẫn chỉnh sửa được. Không bao giờ giả lập danh sách bằng tham số đánh số (date1, date2…). Danh sách vắng mặt là danh sách rỗng; default chỉ khởi tạo đối tượng mới. Các mục có id ổn định: sửa bằng edit_list(target, param, change) (change: insert(index?, id, values?), remove(item), move(item, to), set_field(item, field, value); bạn cung cấp id), hoặc thay toàn bộ danh sách bằng set_param (null làm rỗng). Khi add_element hoặc add_slide tạo widget mà bạn không đưa các mục, hãy truyền "defaults": true để khởi tạo giá trị mặc định của danh sách; không bao giờ khi chèn lại hoặc di chuyển.

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
agent-memory-lost = Tác tử không còn nhớ cuộc trò chuyện này: nó bắt đầu lại từ đầu.
agent-turn-max-tokens = Tác nhân AI đã đạt giới hạn độ dài.
agent-turn-max-turns = Tác nhân AI đã đạt giới hạn số bước.
agent-turn-refusal = Tác nhân AI đã từ chối.
agent-turn-ended = Kết thúc lượt ({ $stop }).
agent-choice-cancelled = Câu hỏi đã bị hủy
agent-choice-answered = ✓ { $answer }
agent-report-why = Tại sao
