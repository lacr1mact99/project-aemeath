# Research: Teach a Task

Phase 0 của `/speckit-plan`. Số phiên bản và ngày tra từ crates.io / GitHub / docs.rs / learn.microsoft.com
ngày 2026-11-22. Mục ghi **[M0]** là ẩn số chỉ trả lời được bằng spike trên máy thật; tiêu chí xem
[quickstart.md](./quickstart.md#m0).

Thay đổi so với `draft-v0.3.md` §5 được đánh dấu **(đổi)**.

> **Môi trường phát triển ≠ môi trường đích.** Máy dùng để phát triển không thuộc mạng nội bộ công ty.
> Không quyết định nào dưới đây dựa trên mạng, proxy, chứng chỉ, DNS hay cấu hình của máy phát triển, và
> code MUST NOT ngầm giả định chúng (không hard-code proxy, CA, tên miền, địa chỉ máy chủ model). Các phép
> đo phụ thuộc môi trường công ty — mọi mục **[M0]** về mạng/model (R-6, R-7) và về ảnh Windows của công ty
> (gói OCR R-5, bộ gõ, Office, chính sách nhóm) — MUST chạy trên **máy công ty, trong mạng công ty**; kết quả
> đo trên máy phát triển không được dùng làm căn cứ cho các mục đó.

---

## R-1. Nhận diện và thao tác phần tử UIA — **(đổi)** `uiautomation` thay `terminator-rs`

- **Decision**: `uiautomation` 0.25.x (leexgone, Apache-2.0) làm lớp UIA duy nhất; thiếu gì gọi thẳng
  `IUIAutomation` qua crate `windows`.
- **Rationale**: bảo trì đều (0.25.3 ngày 2026-11-21), lớp mỏng, đã có đủ API cần: `element_from_point`,
  `get_focused_element`, `find_all(TreeScope, condition)`, `is_password`, `ValuePattern::get_value`,
  `InvokePattern`, sự kiện focus/structure-changed (feature `event`). Chính `terminator` cũng dựa trên nó.
- **Alternatives**: `terminator-rs` 0.23.35 — bản crates.io đứng từ 2025-12-03 trong khi repo đã 0.24.x,
  kéo theo tokio, reqwest, tungstenite, uni-ocr, xcap, dep macOS/Linux; API đổi nhanh → trái Nguyên tắc VII.
  Giữ làm tài liệu tham khảo thiết kế.

## R-2. Ghi input — **(đổi)** hook low-level tự viết thay `terminator-workflow-recorder`/`rdev`

- **Decision**: `SetWindowsHookExW(WH_MOUSE_LL / WH_KEYBOARD_LL)` qua crate `windows`, chạy trên một luồng
  riêng có message loop. Hook chỉ đẩy sự kiện thô (vk, scancode, flags, `dwExtraInfo`, toạ độ, thời điểm)
  vào channel và trả ngay; mọi việc nặng (tra UIA, chụp ảnh, ghép ký tự) làm ở luồng khác.
- **Ghép ký tự**: lấy văn bản bằng `ToUnicodeEx` ở luồng xử lý (không gọi trong hook). Bộ gõ tiếng Việt
  (Unikey/EVKey) tự phát lại ký tự đã dựng bằng SendInput → các ký tự đó đến hook với cờ injected và được
  ghi như người dùng gõ. **[M0]** xác nhận văn bản ghi ra khớp văn bản thực tế trong ô.
- **Phân biệt input của chính ứng dụng**: mọi SendInput của `teach.exe` gắn `dwExtraInfo = TEACH_MAGIC`.
  Khi chạy skill, input nào **không** mang dấu đó (kể cả từ bộ gõ) = người dùng chạm vào → tạm dừng (FR-022).
  Không dựa vào cờ `LLMHF_INJECTED` vì bộ gõ cũng đặt cờ này.
- **Alternatives**: `terminator-workflow-recorder` — không có logic IsPassword (phải tự làm), dep nặng (R-1).
  `rdev` 0.5.3 — release cuối 2023, không lộ `dwExtraInfo` (issue #178), làm hỏng dead key (issue #77)
  → rủi ro tiếng Việt.

## R-3. Giả lập chuột/phím — **(đổi)** SendInput trực tiếp thay `enigo`

- **Decision**: `SendInput` qua crate `windows`; gõ văn bản bằng `KEYEVENTF_UNICODE` từng UTF-16 unit;
  ưu tiên `ValuePattern::set_value` / `InvokePattern::invoke` khi phần tử hỗ trợ và điều kiện kết quả kiểm
  tra được, rơi về SendInput khi không.
- **Rationale**: cần đặt `dwExtraInfo` (R-2) và kiểm soát DPI; enigo 0.6.1 (2025-08) chưa có bản sửa DPI
  đang nằm trên nhánh main. Phần dùng tới chỉ vài chục dòng.
- **DPI**: tiến trình khai báo Per-Monitor-V2 trong manifest; mọi toạ độ là pixel vật lý; `xy_in_window`
  quy đổi theo khung cửa sổ hiện tại.
- **[M0]**: gõ Unicode tiếng Việt có dấu vào Notepad, Office, một app Electron, một trình duyệt khi bộ gõ
  Việt đang bật/tắt.

## R-4. Ô mật khẩu (FR-006, Nguyên tắc V)

- **Decision**: trước khi ghi bất kỳ phím nào, đọc `is_password` của phần tử đang focus. Ba trường hợp:
  `true` → `secret: true`, không ghi ký tự, che `rect` trong ảnh; `false` → ghi bình thường;
  **không xác định** (không lấy được phần tử focus hoặc thuộc tính lỗi) → coi như bí mật (không ghi ký tự,
  bước được gắn cảnh báo `text_not_recorded`, người dùng nhập lại văn bản trong editor nếu không phải bí mật).
- **Rationale**: Nguyên tắc V là không thương lượng; sai theo hướng "không ghi" chỉ tốn một lần sửa tay.
- **[M0]**: tỉ lệ `is_password` đúng trên ô mật khẩu của Win32, WPF, trình duyệt, Electron.

## R-5. OCR — `Windows.Media.Ocr`, nhưng **không trông vào nhận dạng tiếng Việt có dấu**

- **Phát hiện**: OCR của Windows chỉ nhận ngôn ngữ có gói `Language.OCR~~~<tag>`; bảng FOD Windows 10 không
  có gói OCR `vi-VN` (vi-VN chỉ có Basic, TextToSpeech). Chưa xác minh trên Windows 11 hiện tại.
- **Decision**: dùng `OcrEngine` với ngôn ngữ `vi-VN` nếu `IsLanguageSupported`, nếu không thì `en-US`.
  Selector `ocr_text` được so khớp **không dấu, không phân biệt hoa thường, theo khoảng cách chỉnh sửa**
  (bỏ dấu cả chuỗi đã lưu và chuỗi OCR, ví dụ "Lưu" ↔ "Luu"). Danh sách chữ OCR gửi model (khi app không có
  UIA) cũng ở dạng đã đọc được, không suy dấu.
- **Rationale**: OCR chỉ là đường dự phòng thứ ba (sau automation_id và role+name); so khớp bỏ dấu là tất
  định, không thêm phụ thuộc. Tesseract (`vie`) kéo theo thư viện C + dữ liệu huấn luyện → chỉ thêm nếu đo
  thấy cần (Nguyên tắc VII).
- **Ảnh hưởng nghiệm thu**: tiêu chí M0 "OCR đọc được nhãn nút tiếng Việt" được hiểu là: **selector
  `ocr_text` tìm đúng ≥ 9/10 nút có nhãn tiếng Việt** trên ứng dụng thử, không đòi chuỗi OCR khớp từng dấu.
  Nếu không đạt → mở quyết định thêm Tesseract.
- **[M0]**: `Get-WindowsCapability -Online | ? Name -like 'Language.OCR*'` trên Win10/Win11; gọi
  `OcrEngine` từ exe unpackaged.

## R-6. Giao tiếp model — `ureq` thay `reqwest`

- **Decision**: `ureq` 3.x (đồng bộ), gọi trên luồng worker của runner; `serde_json` để dựng/đọc JSON.
  TLS dùng kho chứng chỉ Windows của máy đang chạy (`rustls-platform-verifier`; dự phòng feature
  `native-tls`/SChannel) để nhận CA nội bộ do IT công ty cài; proxy lấy từ cấu hình hệ thống/biến môi trường
  của máy đang chạy. Không có giá trị mạng nào nằm trong code; trên máy phát triển, phần model được kiểm thử
  bằng máy chủ giả cục bộ (`tests/` dựng HTTP server trên `127.0.0.1` trả câu trả lời mẫu, kể cả `<think>`,
  JSON lệch, 401, timeout).
- **Rationale**: vài lời gọi mỗi lần chạy, không cần async runtime; bỏ tokio khỏi toàn bộ dự án.
- **Alternatives**: `reqwest` (blocking vẫn kéo tokio).
- **[M0]** (`teach.exe probe-model`, chạy **trên máy công ty, trong mạng công ty**, với cả DeepSeek và GLM):
  HTTPS + CA + proxy công ty; endpoint có
  đúng `/chat/completions` kiểu OpenAI; có nhận `response_format: json_schema`; phần suy nghĩ trả trong
  `<think>` hay trường riêng; độ trễ p50/p95; giới hạn ngữ cảnh; ≥ 19/20 câu trả lời JSON một dòng hợp lệ.
  Nếu giao thức khác OpenAI: chỉ sửa mô-đun `src/model/` (Constitution §Ràng buộc).

## R-7. Ngôn ngữ prompt gửi model

- **Decision**: system prompt và ví dụ mẫu bằng tiếng Anh; tên phần tử, tiêu đề cửa sổ, ý định giữ nguyên
  ngôn ngữ gốc; `label_step` nhận tham số `LANGUAGE: vi|en` để trả câu ý định theo ngôn ngữ cài đặt (FR-048).
- **Rationale**: model nhỏ thường theo chỉ dẫn định dạng tiếng Anh ổn định hơn; định dạng câu trả lời là
  JSON nên không phụ thuộc ngôn ngữ người dùng.
- **[M0/M3]**: chạy bộ câu hỏi mẫu với prompt Anh và Việt cho cả hai model; giữ bản tiếng Anh trừ khi bản
  Việt cao hơn ≥ 5 điểm phần trăm.

## R-8. Phím tắt, dừng khẩn cấp, thông điệp (giao diện → spec 002)

> Cập nhật 2026-11-23: GUI, khay, thông báo Windows, một-tiến-trình, khởi động cùng Windows chuyển sang spec
> 002 (Tauri) và được nghiên cứu ở plan 002. Phương án egui/`tray-icon`/`tauri-winrt-notification` trước đây
> bị bỏ. Lõi chỉ giữ các mục dưới.

| Nhu cầu | Decision | Ghi chú / rủi ro |
|---|---|---|
| Dừng khẩn cấp | **không** dựa vào giao diện: hook bàn phím (R-2) nhận tổ hợp dừng và bật cờ `AtomicBool` mà runner kiểm tra trước mỗi thao tác và trong mọi vòng chờ (≤ 50 ms) | ≤ 1 s kể cả khi giao diện ẩn/treo/chưa mở (SC-003) |
| Phím tắt ghi (tạm dừng/dừng) | **(đổi)** cũng bắt trong hook LL của lõi, không dùng `global-hotkey` | một cơ chế cho mọi phím tắt; tổ hợp bị nuốt khỏi ứng dụng đích khi khớp; trùng nhau → `save_settings` báo lỗi |
| Thông điệp lõi (FR-048) | hai bảng chuỗi `vi`/`en` trong `i18n.rs`, khoá là enum: nhãn tất định, lý do tạm dừng, lỗi | không thêm crate; chuỗi giao diện thuộc 002 |

## R-9. Lưu trữ, bí mật, lịch

| Nhu cầu | Decision | Lý do |
|---|---|---|
| YAML | **(đổi)** `serde-saphyr` 2.x | `serde_yaml` deprecated; `serde_yml` có RUSTSEC-2025-0068 (unsound); `serde_norway` 11 tháng không cập nhật |
| JSONL | `serde_json`, ghi thêm từng dòng + flush | |
| Bí mật | `keyring` 4.x (store mặc định = Windows Credential Manager) | service `teach` |
| Cron | **(đổi)** `croner` (5 trường, xử lý DST theo giờ địa phương qua `chrono`) | `cron` 0.17 bắt buộc 7 trường. croner 5.0 rất mới → ghim phiên bản, có test DST |
| Thời gian | `chrono` | |
| Máy rảnh | `GetLastInputInfo` | so với `idle_minutes` |
| Khoá phiên | `WTSRegisterSessionNotification` trên cửa sổ message-only ở luồng nền (cùng luồng với hook); dự phòng poll `OpenInputDesktop` mỗi 5 s | **[M0]** |
| Chụp màn hình | `xcap` 0.9 | issue Windows mở về DPI/vị trí cửa sổ (#143) → **[M0]**; dự phòng GDI `BitBlt` qua `windows` |
| PNG | `image` (chỉ codec png) | che vùng mật khẩu + ghi file |

## R-10. Mô hình luồng (không dùng async runtime)

- **Luồng của bên gọi**: giao diện 002 hoặc CLI giữ `Engine` và nhận `Event` qua channel; lõi không phụ thuộc event loop của giao diện.
- **Luồng input**: hook LL + cửa sổ message-only nhận thông báo khoá phiên; đẩy sự kiện qua channel; giữ cờ
  dừng khẩn cấp và nhận mọi phím tắt của lõi.
- **Luồng worker**: recorder-processor *hoặc* runner (không bao giờ cả hai, FR-025); gọi UIA (COM khởi tạo
  MTA trên luồng này), chụp ảnh, gọi model đồng bộ.
- **Scheduler**: luồng tick riêng của lõi, kiểm tra mỗi 15 s; logic là máy trạng thái
  thuần (data-model §Occurrence) để test không cần đồng hồ thật.
- Giao tiếp bên gọi ⇄ lõi ⇄ worker: `std::sync::mpsc` (lệnh xuống, sự kiện lên) — không cần crossbeam.

## R-11. Giới hạn đã biết (lõi trả lý do rõ; giao diện 002 / tài liệu hiển thị)

- Ứng dụng chạy quyền Administrator: Windows (UIPI) chặn hook/SendInput/UIA từ tiến trình quyền thường →
  bước trên app đó thất bại ở nấc 1 với lý do rõ ràng; không tự nâng quyền.
- Màn hình khoá / UAC secure desktop: không thao tác được; lịch chờ mở khoá (FR-043).
- App không có UIA và OCR đọc kém: chỉ còn `xy_in_window` (cảnh báo FR-008).
