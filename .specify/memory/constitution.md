<!--
Sync Impact Report (2026-11-27)
- Version change: 1.1.0 → 1.2.0 (MINOR: mở rộng hướng dẫn ở VI và §Quy trình, không bỏ nguyên tắc nào)
- Modified principles:
  V. Riêng tư — vùng mật khẩu MUST bị che, hoặc ảnh bị bỏ khi không che chắc chắn được
  VI. Đo được mới tính — chế độ "hộp thoại bất ngờ" được chạy trên bộ 10 skill của ứng dụng thử có kiểm soát
       (hộp thoại phải thuộc chính ứng dụng đích mới đo đúng nấc chẩn đoán); lần đạt phải có log đúng nấc được đo
       và được chấm bằng phép kiểm tra độc lập với điều kiện kết quả của skill
- Expanded sections: "Quy trình…" — log lần chạy được phép bỏ ảnh khi ảnh có thể lộ bí mật (Nguyên tắc V
  thắng), log MUST ghi rõ ảnh nào bị bỏ và vì sao
- Removed sections: none
- Dependent artifacts: specs/001-teach-a-task/spec.md (FR-006, FR-016, SC-004), plan.md (Constitution Check),
  tasks.md (T063, T091), contracts/storage-files.md (`shot_omitted`)
- Deferred TODOs: none

Sync Impact Report (2026-11-23)
- Version change: 1.0.0 → 1.1.0 (MINOR: mở rộng hướng dẫn, không bỏ/định nghĩa lại nguyên tắc)
- Modified principles:
  VII. Đơn giản, đúng phạm vi — bỏ cấm "stack web" chung chung; ghi Tauri là công nghệ giao diện đã chốt,
       cấm stack web khác, frontend không tải tài nguyên mạng
- Expanded sections: "Ràng buộc nền tảng và giao tiếp model" — kiến trúc hai lớp lõi/giao diện; ranh giới
  lõi ⇄ giao diện là ranh giới tin cậy (IV, V thực thi trong lõi; webview không có quyền file/shell/mạng)
- Removed sections: none
- Dependent artifacts: specs/002-tauri-gui/spec.md (Assumptions "Constitution" đã được giải quyết);
  plan 002 dùng Nguyên tắc VII bản mới, không cần Complexity Tracking cho Tauri
- Deferred TODOs: none

Sync Impact Report (2026-11-22)
- Version change: (template, chưa có) → 1.0.0
- Modified principles: thay toàn bộ placeholder bằng 7 nguyên tắc:
  I. Tất định trước, model chỉ sửa chữa
  II. Model là người trả lời trắc nghiệm, chỉ văn bản
  III. Kiểm tra bằng code, không bằng model
  IV. Con người luôn nắm quyền (KHÔNG THƯƠNG LƯỢNG)
  V. Riêng tư và bí mật ở lại trên máy (KHÔNG THƯƠNG LƯỢNG)
  VI. Đo được mới tính
  VII. Đơn giản, đúng phạm vi
- Added sections: "Ràng buộc nền tảng và giao tiếp model", "Quy trình phát triển và cổng chất lượng"
- Removed sections: none
- Templates: không sửa (template đọc constitution lúc chạy)
- Deferred TODOs: none
-->

# Teach a Task Constitution

## Core Principles

### I. Tất định trước, model chỉ sửa chữa

- Mọi việc làm được bằng code tất định (tìm phần tử, thao tác, kiểm tra kết quả, sinh điều kiện,
  biên dịch bản ghi) MUST do code làm.
- Model MUST chỉ được gọi khi phần tất định thất bại: bộ chọn không khớp/khớp nhiều, hoặc điều kiện
  kết quả sai; và khi gắn nhãn ý định lúc biên dịch.
- Khi giao diện không đổi, một lần chạy MUST hoàn tất với 0 lần gọi model.
- Mọi tính năng MUST hoạt động (có thể suy giảm) khi không có model: thiếu model thì rơi về nhãn
  tất định hoặc hỏi người dùng, không bao giờ chặn.

Lý do: model của công ty có thể yếu, chậm, hoặc không truy cập được; độ tin cậy phải nằm ở code.

### II. Model là người trả lời trắc nghiệm, chỉ văn bản

- Mỗi lần gọi model MUST là một câu hỏi đóng: chọn số thứ tự trong danh sách ứng viên hoặc một giá
  trị trong enum cố định. Ngoại lệ duy nhất là câu ý định ngắn lúc biên dịch.
- Model MUST NOT lập kế hoạch, sinh toạ độ, sinh hành động mới hay viết văn bản tự do lúc chạy.
- Đầu vào gửi model MUST chỉ là văn bản; ngữ cảnh mỗi lần gọi MUST giới hạn ở bước hiện tại và danh
  sách ứng viên (khoảng ≤ 2.000 token), không gửi lịch sử hay toàn bộ skill.
- Harness MUST tự phân tích và kiểm tra câu trả lời (bỏ phần suy nghĩ/chữ thừa, kiểm tra tập cho
  phép, hỏi lại tối đa một lần, sau đó rơi về `none`/`ask_user`). Code MUST NOT phụ thuộc vào
  structured output, tool calling hay đầu vào ảnh; được phép bật thêm nếu máy chủ hỗ trợ.
- Mỗi loại câu hỏi MUST có prompt cố định, có ví dụ mẫu, và có mặt trong bộ kiểm tra ngoại tuyến.

Lý do: đây là mẫu số chung thấp nhất để model cỡ nhỏ, chỉ văn bản (DeepSeek, GLM) vẫn dùng được.

### III. Kiểm tra bằng code, không bằng model

- Việc một bước "xong" MUST chỉ được quyết định bởi điều kiện kết quả kiểm tra tất định (cửa sổ
  focus, phần tử tồn tại, giá trị bằng X).
- Mọi lựa chọn của model MUST được xác minh bằng điều kiện kết quả trước khi lần chạy đi tiếp.
- Tự vá (thêm bộ chọn mới vào skill) MUST chỉ xảy ra khi điều kiện kết quả đạt, và MUST ghi log.

Lý do: model nhỏ có thể sai mà vẫn tự tin; code là trọng tài duy nhất.

### IV. Con người luôn nắm quyền (KHÔNG THƯƠNG LƯỢNG)

- Ứng dụng thao tác trên máy thật, không có máy ảo cách ly, nên các lớp bảo vệ sau MUST luôn có:
  dừng khẩn cấp có hiệu lực trong ≤ 1 giây; chuột/phím của người dùng làm tạm dừng; bước
  `approval` luôn chờ người bấm Đồng ý; giới hạn tổng thời gian và tổng số lần gọi model mỗi lần chạy.
- MUST NOT có chế độ tự phê duyệt hay tự chạy theo lịch: mỗi lần chạy theo lịch MUST có một lần
  người dùng đồng ý.
- Khi kẹt, hệ thống MUST tạm dừng và trao quyền cho người dùng, không đoán tiếp.
- MUST NOT chạy hai skill cùng lúc.

Lý do: sai một cú bấm trên máy thật có thể gây mất dữ liệu; an toàn đứng trên tiện lợi.

### V. Riêng tư và bí mật ở lại trên máy (KHÔNG THƯƠNG LƯỢNG)

- Bản ghi, ảnh chụp, log MUST chỉ lưu trên máy người dùng.
- Ký tự gõ vào ô mật khẩu MUST NOT được ghi; vùng đó MUST bị che trong ảnh, hoặc ảnh MUST bị bỏ khi không
  che chắc chắn được (không biết vùng, hoặc giá trị bí mật có thể hiện lại ở chỗ khác).
- Ảnh chụp, giá trị bí mật, nội dung clipboard MUST NOT được gửi tới máy chủ model.
- API key và bí mật MUST chỉ nằm trong kho bí mật của hệ điều hành (Windows Credential Manager);
  MUST NOT xuất hiện trong file cấu hình, skill, log, prompt hay thông báo lỗi.
- Người dùng MUST được thông báo rõ thứ gì được gửi tới máy chủ model trước lần gọi đầu tiên.
- Không ghi âm.

Lý do: bản ghi màn hình chứa dữ liệu nghiệp vụ; rò rỉ là không thể thu hồi.

### VI. Đo được mới tính

- Mọi khẳng định về độ tin cậy (đặc biệt "model yếu vẫn làm được") MUST được chứng minh bằng bộ đo:
  10 skill chuẩn × 3 chế độ (giao diện không đổi + tắt model; bộ chọn bị làm hỏng; hộp thoại bất
  ngờ) và bộ kiểm tra ngoại tuyến có đáp án. Chế độ hộp thoại bất ngờ được phép dùng bộ 10 skill trên
  một ứng dụng thử có kiểm soát, để hộp thoại thuộc chính ứng dụng đích. Một lần chỉ được tính đạt khi
  log cho thấy đúng nấc đang đo đã được dùng và phép kiểm tra kết quả độc lập với điều kiện kết quả của
  skill xác nhận.
- Ẩn số về hạ tầng (giao thức máy chủ, OCR tiếng Việt, khả năng build thư viện) MUST được đo ở bước
  thử nghiệm kỹ thuật trước khi xây phụ thuộc vào chúng.
- Phần tất định MUST đạt nghiệm thu (10/10 skill chế độ 1, không gọi model) trước khi thêm phần model.
- Thay đổi prompt, bộ lọc ứng viên hay bộ phân tích câu trả lời MUST chạy lại bộ kiểm tra ngoại
  tuyến và không làm giảm tỉ lệ đúng dưới ngưỡng (≥ 90%).

Lý do: không đo thì không biết harness có chặt thật hay không.

### VII. Đơn giản, đúng phạm vi

- Một binary phát hành, dữ liệu là file văn bản người đọc được trong thư mục dữ liệu người dùng; MUST
  NOT thêm cơ sở dữ liệu hay dịch vụ nền riêng khi chưa có nhu cầu thật.
- Giao diện người dùng dùng **Tauri** (đã chốt, spec 002): frontend web nhúng chạy trong cùng binary, chỉ
  nói chuyện với lõi Rust qua bề mặt điều khiển. Ngoài giao diện Tauri, MUST NOT thêm stack web khác (máy
  chủ web, giao diện từ xa, framework web thứ hai). Frontend MUST giữ gọn: không tải tài nguyên từ mạng,
  phụ thuộc frontend phải có lý do trong plan như phụ thuộc Rust.
- Một model tại một thời điểm; không định tuyến giữa các model.
- Những thứ ngoài phạm vi (macOS/Linux, đầu vào ảnh, model nhúng, nhánh rẽ/vòng lặp phức tạp, kích
  hoạt theo sự kiện, chia sẻ skill, ghi video/âm) MUST NOT được xây "để sẵn"; thêm khi có nhu cầu
  thật và qua sửa spec.
- Phụ thuộc mới MUST có lý do rõ trong plan; ưu tiên thư viện chuẩn và tính năng có sẵn của Windows.
- Tham số hoá skill do người dùng quyết; MUST NOT để model đoán.

Lý do: mỗi lớp phức tạp thêm là một chỗ có thể hỏng trên máy người dùng.

## Ràng buộc nền tảng và giao tiếp model

- Nền tảng: Windows 10/11; ngôn ngữ triển khai Rust; nhận diện phần tử qua UI Automation, OCR có
  sẵn của Windows làm dự phòng.
- Kiến trúc hai lớp: **lõi** (thư viện Rust + CLI, spec 001) chứa mọi quy tắc, an toàn, riêng tư;
  **giao diện Tauri** (spec 002) chỉ hiển thị và chuyển lựa chọn của người dùng. Mọi quy tắc của Nguyên tắc
  IV và V MUST được thực thi trong lõi, không dựa vào giao diện; lõi MUST kiểm chứng được không cần giao diện.
- Ranh giới lõi ⇄ giao diện là ranh giới tin cậy: lõi MUST coi mọi lệnh từ giao diện là đầu vào cần kiểm tra;
  không có lệnh nào trả ra API key, giá trị bí mật hay nội dung đã gõ; webview MUST NOT có quyền truy cập file,
  shell hay mạng ngoài các lệnh lõi đã khai báo.
- Model: chỉ cấu hình bằng ba tham số (base URL, API key, tên model); giao thức giả định là chat
  completions tương thích OpenAI. Toàn bộ giao tiếp HTTP với model MUST nằm sau một hàm/mô-đun duy
  nhất để thay giao thức chỉ phải sửa một chỗ.
- Lời gọi model dùng nhiệt độ 0 (hoặc chế độ tất định nhất có thể); lỗi mạng, hết giờ, lỗi xác thực
  MUST được xử lý như `ask_user` với nguyên nhân hiển thị rõ.
- Mất mạng công ty không được làm hỏng skill có giao diện không đổi.
- Lịch chỉ chạy khi phiên Windows đang mở khoá; GUI MUST nói rõ giới hạn này.

## Quy trình phát triển và cổng chất lượng

- Quy trình Spec Kit: spec → (clarify) → plan → tasks → implement. Plan MUST có mục "Constitution
  Check" đối chiếu từng nguyên tắc I–VII và ghi rõ mọi vi phạm có chủ đích kèm lý do.
- Thứ tự mốc theo spec: M0 thử nghiệm kỹ thuật → M1 ghi + biên dịch → M2 chạy tất định → M3 model +
  tự vá → M4 GUI đầy đủ → M5 lịch + khay. Không bắt đầu mốc sau khi mốc trước chưa đạt nghiệm thu,
  trừ việc song song không phụ thuộc.
- Kiểm thử: logic không tầm thường (bộ phân tích câu trả lời model, sinh bộ chọn/điều kiện, bậc
  thang phân giải, tính lịch, che bí mật) MUST có test tự động chạy được không cần giao diện thật.
  Hành vi trên giao diện thật được nghiệm thu bằng bộ đo ở Nguyên tắc VI.
- Mỗi lần chạy skill MUST để lại log đủ để tái hiện: bước, nấc đã dùng, prompt, câu trả lời, ảnh. Ảnh
  có thể lộ bí mật được bỏ theo Nguyên tắc V; khi đó log MUST ghi rõ ảnh nào bị bỏ và vì sao.
- Review MUST kiểm tra riêng các nguyên tắc KHÔNG THƯƠNG LƯỢNG (IV, V) cho mọi thay đổi chạm tới
  ghi, chạy, lịch, log hoặc giao tiếp model.

## Governance

- Constitution này đứng trên mọi tài liệu và thói quen khác của dự án. Khi spec/plan mâu thuẫn với
  nó, constitution thắng cho tới khi được sửa.
- Sửa đổi: đề xuất bằng văn bản (thay đổi gì, vì sao, ảnh hưởng tới spec/plan nào), được chủ dự án
  chấp thuận, cập nhật Sync Impact Report và các tài liệu phụ thuộc trong cùng thay đổi.
- Phiên bản theo semver: MAJOR khi bỏ hoặc định nghĩa lại nguyên tắc; MINOR khi thêm nguyên tắc/mục
  hoặc mở rộng đáng kể; PATCH khi làm rõ câu chữ.
- Nguyên tắc IV và V chỉ được nới bằng thay đổi MAJOR kèm lý do an toàn/riêng tư được ghi rõ.
- Tuân thủ: mỗi plan và mỗi review MUST xác nhận tuân thủ; độ phức tạp vượt Nguyên tắc VII MUST được
  biện minh trong bảng Complexity Tracking của plan.

**Version**: 1.2.0 | **Ratified**: 2026-11-22 | **Last Amended**: 2026-11-27
