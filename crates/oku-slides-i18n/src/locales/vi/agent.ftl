# Tác nhân AI: bề mặt, xác nhận, luồng.

agent-surface-cancel = Hủy
agent-surface-confirm = Xác nhận
agent-surface-confirm-danger = Có, thực hiện
agent-surface-rewrite = Diễn đạt lại
agent-surface-pick = Chọn { $n }
agent-surface-keep = Giữ bản gốc
agent-surface-edit = Chỉnh sửa trước khi áp dụng
agent-surface-close = Đóng
agent-surface-themes = Chủ đề
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
agent-confirm-copy-pack = Lưu một bản sao của gói “{ $name }”? Bản thân gói vẫn giữ nguyên.
agent-confirm-copy-edited-theme = Lưu một bản sao của chủ đề “{ $name }”? Bản thân chủ đề vẫn giữ nguyên.
agent-confirm-delete-pack = Xóa gói “{ $name }”? Không thể hoàn tác thao tác này.
agent-confirm-edit-theme = Lưu phiên bản mới của chủ đề “{ $name }”?
agent-confirm-copy-theme = Tạo một biến thể của chủ đề tích hợp “{ $name }”? Chủ đề tích hợp vẫn giữ nguyên.
agent-confirm-setting = Thay đổi { $key }: { $old } → { $new }?
agent-confirm-move = Chuyển bản trình bày sang { $path }? Tệp cũ sẽ bị xóa.
agent-confirm-merge = Hợp nhất { $path } vào bản trình bày này? Không thể hoàn tác.
agent-confirm-save-copy = Lưu một bản sao của bản trình bày thành { $path }?
agent-confirm-dictation-api = Gửi những gì bạn đọc chính tả tới { $host } ({ $url }), mô hình { $model }, kèm bí mật của { $var }?
agent-confirm-dictation-download = Dùng { $name } và tải xuống ngay ({ $size })?
agent-confirm-dictation-language = Dùng đọc chính tả của Apple và tải ngôn ngữ của nó ngay?
agent-confirm-agent = Chuyển tác nhân của ứng dụng sang { $name } ({ $command }) khi không còn phản hồi nào đang chạy?
agent-confirm-agent-builtin = Chuyển tác nhân của ứng dụng sang { $name } khi không còn phản hồi nào đang chạy?
agent-confirm-agent-model = Tải xuống mô hình của tác nhân cục bộ ({ $size })?

agent-not-found = Không tìm thấy tác nhân AI: { $command }. Cài đặt [agent.presets].
agent-spawn-failed = Không thể khởi động tác nhân AI: { $command }. Cài đặt [agent.presets].
agent-no-http = Tác nhân AI này không thể kết nối với các công cụ của Oku Slides.
agent-local-no-program = Tác nhân AI cục bộ chỉ chạy được bên trong ứng dụng Oku Slides.
agent-local-needs-download = Tác tử cục bộ cần tải mô hình của nó xuống trước ({ $gigabytes } GB).
agent-local-intel-mac = Tác tử cục bộ cần một máy Mac dùng chip Apple.
agent-local-unsupported-os = Tác tử cục bộ chưa khả dụng trên hệ thống này.
agent-model-download = Tải mô hình xuống
agent-model-downloading = Đang tải mô hình xuống… { $percent }%
agent-model-cancel = Hủy
agent-model-download-failed = Không thể tải mô hình xuống.
agent-model-download-integrity = Mô hình đã tải xuống bị hỏng và đã bị xóa.
agent-model-download-incomplete = Tải xuống chưa hoàn tất: { $got } byte trên { $expected }.
agent-model-download-cancelled = Đã hủy tải xuống.
local-agent-no-route = Tác nhân AI cục bộ không hiểu cần làm gì. Hãy thử nói theo cách khác.
local-agent-tool-limit = Tác nhân AI cục bộ đã dừng sau { $n } bước mà chưa hoàn tất.
local-agent-invalid-call = Tác nhân AI cục bộ không dùng được các công cụ đúng cách.
local-agent-engine-failed = Mô hình của tác nhân AI cục bộ đã gặp lỗi.
local-agent-retry = Hãy thử lại, hoặc chọn một tác nhân AI khác trong bảng tác nhân AI.
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
agent-error-pack-id = Đây không phải id của một gói: hãy gọi packs để liệt kê các gói và id của chúng.
agent-error-asset-unknown = Hình ảnh này không có trong bản trình bày cũng như trong các hình ảnh đã dán: hãy dùng mã định danh của một hình ảnh đã dán.
agent-error-denied = Người dùng đã từ chối.
agent-error-no-turn = Không có lượt nào đang diễn ra: { $tool } chỉ được hiển thị trong một lượt.
agent-error-tab-no-turn = Chỉ mở hoặc hiển thị thẻ gói hay thẻ chủ đề khi đang trả lời người dùng.
agent-error-tab-limit = Bạn đã mở hoặc hiển thị một thẻ gói hay thẻ chủ đề trong lượt này; hãy hoàn tất ở đó, hoặc hỏi người dùng trước khi dùng thẻ khác.
agent-error-continue-in-other-tab = Yêu cầu này tiếp tục trong thẻ bạn đã mở hoặc hiển thị: hãy bảo người dùng tiếp tục ở đó; thẻ này tạm thời không thay đổi widget nào.
agent-error-pack-open-in-tab = Gói này đang được chỉnh sửa trong một thẻ khác: hãy bảo người dùng hoàn tất ở đó trước.
agent-pack-opened-in-tab = { $shown ->
   [yes] Gói đã mở trong thẻ { $tab }, hiện đang được hiển thị.
  *[no] Gói đã mở trong thẻ { $tab } ở phía sau: cửa sổ của người dùng đang hiển thị một thẻ khác.
} { $draft ->
   [set] Yêu cầu đang chờ trong bản nháp của thẻ đó: hãy bảo người dùng xem lại ở đó rồi gửi đi.
   [kept] Bản nháp của thẻ đó đã có nội dung nên được giữ nguyên. Hãy bảo người dùng tiếp tục ở đó: tác nhân của thẻ đó không biết yêu cầu này.
   [too_long] Yêu cầu quá dài so với bản nháp của thẻ đó. Hãy bảo người dùng tiếp tục ở đó: tác nhân của thẻ đó không biết yêu cầu này.
   [unavailable] Không thể chuyển yêu cầu vào bản nháp của thẻ đó. Hãy bảo người dùng tiếp tục ở đó: tác nhân của thẻ đó không biết yêu cầu này.
  *[none] Hãy bảo người dùng tiếp tục ở đó: tác nhân của thẻ đó không biết yêu cầu này.
} Thẻ này vẫn giữ nguyên.
agent-theme-opened-in-tab = { $shown ->
   [yes] Chủ đề đã mở trong thẻ { $tab }, hiện đang được hiển thị.
  *[no] Chủ đề đã mở trong thẻ { $tab } ở phía sau: cửa sổ của người dùng đang hiển thị một thẻ khác.
} { $draft ->
   [set] Yêu cầu đang chờ trong bản nháp của thẻ đó: hãy bảo người dùng xem lại ở đó rồi gửi đi.
   [kept] Bản nháp của thẻ đó đã có nội dung nên được giữ nguyên. Hãy bảo người dùng tiếp tục ở đó: tác nhân của thẻ đó không biết yêu cầu này.
   [too_long] Yêu cầu quá dài so với bản nháp của thẻ đó. Hãy bảo người dùng tiếp tục ở đó: tác nhân của thẻ đó không biết yêu cầu này.
   [unavailable] Không thể chuyển yêu cầu vào bản nháp của thẻ đó. Hãy bảo người dùng tiếp tục ở đó: tác nhân của thẻ đó không biết yêu cầu này.
  *[none] Hãy bảo người dùng tiếp tục ở đó: tác nhân của thẻ đó không biết yêu cầu này.
} Thẻ này vẫn giữ nguyên.
agent-error-turn-over = Lượt của lệnh gọi này đã kết thúc: không có gì được hiển thị.
agent-error-no-file = Bản trình bày chưa có tệp: hãy cung cấp đường dẫn tuyệt đối.
agent-error-path-relative = Đường dẫn phải là đường dẫn tuyệt đối.
agent-error-path-parent = Đường dẫn không được chứa “..”.
agent-error-setting-locked = Tác nhân không thể thay đổi cài đặt { $key }.
agent-error-setting = Không thể thay đổi cài đặt { $key }: hãy kiểm tra giá trị của nó.
agent-error-history-index = Cần chỉ số của một phiên bản để duyệt lịch sử.
agent-error-no-pack = Bản trình bày này không ghim gói nào: hãy cho pack (xem packs).
agent-error-history-not-opened = Thẻ đã không mở lịch sử kịp thời: hãy yêu cầu lại.
agent-error-present = Không có gì để trình chiếu ở đây: không có trang chiếu nào, hoặc thẻ đang ở chế độ gói, lịch sử hay chỉ đọc.
agent-error-locale = Mã ngôn ngữ không xác định. Hãy cho mã một ngôn ngữ có sẵn hoặc system.
agent-error-empty-interface = Không có gì để thay đổi: hãy cho scheme, locale hoặc guides.
agent-error-empty-dictation = Không có gì để thay đổi: hãy cho ít nhất một cài đặt đọc chính tả.
agent-error-not-shown = Thẻ này không phải thẻ đang hiển thị trong cửa sổ: hãy yêu cầu lại khi người dùng hiển thị nó.
agent-error-themes-deck-mode = Bộ chọn chủ đề chỉ mở khi đang chỉnh sửa trang chiếu.
agent-error-packs-deck-mode = Bảng gói của bản trình bày chỉ mở khi đang chỉnh sửa trang chiếu.
agent-error-usage-no-deck = Mức sử dụng của thẻ này không thuộc bản trình bày nào: chỉ thẻ gói hay thẻ chủ đề được mở từ một bản trình bày (do người dùng hoặc tác nhân AI mở), hoặc gói riêng của bản trình bày, mới được tính cho bản trình bày đó.
agent-error-rewrite-text = Bản viết lại trả về mà không có văn bản.
agent-error-paragraphs = Các đoạn của bản viết lại không hợp lệ.
agent-error-draft-too-long = Bản nháp quá dài.
agent-error-draft-origin = Bản nháp bị từ chối: không rõ chế độ xem.
agent-error-draft-no-agent = Tác nhân của thẻ này chưa khởi động: bản nháp bị từ chối.
agent-error-too-many-images = Bản nháp đã có đủ hình ảnh.
agent-error-image-not-base64 = Hình ảnh đã dán không đến đầy đủ.
agent-error-image-gone = Một hình ảnh đã dán không còn khả dụng: hãy dán lại.
agent-error-unknown-agent = Tác nhân này không còn trong cấu hình.
agent-error-agent-locked = Lựa chọn tác nhân do môi trường áp đặt.
agent-error-agent-not-saved = Không thể lưu lựa chọn tác nhân.
agent-error-unknown-model = Tác nhân này không cung cấp mô hình đó.
agent-error-model-not-saved = Không thể lưu lựa chọn mô hình.
voice-error-no-dictation = Đọc chính tả không khả dụng ở đây
voice-error-busy = Đang đọc chính tả: hãy thử lại khi đã rảnh
voice-error-unknown-model = Mô hình đọc chính tả không xác định
voice-error-unavailable = Mô hình này không hỗ trợ ngôn ngữ đã chọn
voice-error-installed = Mô hình này đã được cài đặt
voice-error-not-downloadable = Mô hình này không cần tải xuống
voice-error-nothing-to-cancel = Không có lượt tải xuống nào đang chạy
voice-error-bad-silence = Khoảng lặng phải kéo dài từ 0,5 đến 10 giây
voice-error-bad-language = Ngôn ngữ đọc chính tả không xác định
voice-error-bad-url = Địa chỉ API phải bắt đầu bằng https:// (http:// chỉ cho máy tính này), không có tên người dùng, tham số hay ký tự đặc biệt
voice-error-bad-key-env = Biến khóa phải là một tên như OPENAI_API_KEY: chữ hoa, chữ số và _
voice-error-bad-api-model = Tên mô hình API chỉ được dùng chữ cái, chữ số và . _ : / -; tên bản triển khai Azure chỉ chữ cái, chữ số và . _ -
voice-error-bad-api-version = Phiên bản API Azure chỉ gồm chữ cái, chữ số, . và -, tối đa 32 ký tự, ví dụ 2025-03-01-preview
voice-error-bad-api-headers = Tiêu đề được viết Name: value; Name: $VARIABLE, tối đa 8. Authorization, api-key, Host, Content-Type, Content-Length, Transfer-Encoding, Connection và tên trùng lặp bị từ chối
voice-error-api-reserved = Chỉ người dùng mới đặt địa chỉ, biến khóa, phiên bản Azure và tiêu đề của API: hãy mở hộp bằng show_dictation và nhờ họ
voice-error-changed = Cài đặt đã thay đổi kể từ câu hỏi: hãy hỏi lại
voice-error-nothing-to-download = Không có gì để tải xuống cho tác nhân này
agent-tune-prompt = Tinh chỉnh bài trình bày đã nhập
agent-settings-unknown-selected = Thiết lập agent.selected = { $selected } không chỉ tới tác nhân AI nào trong [agent.presets] (đã biết: { $known })
agent-settings-preset-launch = Tác nhân AI [agent.presets.{ $id }] cần đúng một trong hai: command hoặc builtin
