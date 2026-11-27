# Research: Teach a Task

Phase 0 của `/speckit-plan`. Số phiên bản và ngày tra từ crates.io / GitHub / docs.rs / learn.microsoft.com
ngày 2026-11-22. Mục ghi **[M0]** là ẩn số chỉ trả lời được bằng spike trên máy thật; tiêu chí xem
[quickstart.md](./quickstart.md#m0).

Thay đổi so với bản nháp gốc v0.3 §5 (`git show d6e355c:specs/001-teach-a-task/draft-v0.3.md`) được đánh dấu **(đổi)**.

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
- **Đóng gói**: `Windows.Media.Ocr` có thể chỉ được hỗ trợ cho ứng dụng desktop có *package identity*; CLI
  `teach.exe` hiện unpackaged. Chưa xác minh — đây là cổng riêng của M0 (plan D-3), độc lập với độ chính xác.
- **[M0]** hai cổng: (1) gọi `OcrEngine` từ `teach.exe` **unpackaged** và từ kiểu phân phối plan 002 dự kiến —
  chạy được hay không; (2) `Get-WindowsCapability -Online | ? Name -like 'Language.OCR*'` trên Win10/Win11 và
  ≥ 9/10 nhãn tiếng Việt. Cổng (1) trượt → chọn một: sparse package cấp identity, Tesseract `vie`, hoặc bỏ
  `ocr_text` khỏi MVP (plan D-3) — chốt trước khi xây recorder/runner phần OCR.

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
- **[M3]** (quickstart §M3, máy công ty, T080): chạy bộ câu hỏi mẫu với prompt Anh và Việt cho cả hai model; giữ
  bản tiếng Anh trừ khi bản Việt cao hơn ≥ 5 điểm phần trăm. Cần prompt và bộ câu hỏi mẫu nên không đo được ở M0.

## R-8. Phím tắt, dừng khẩn cấp, thông điệp (giao diện → spec 002)

> Cập nhật 2026-11-23: GUI, khay, thông báo Windows, một-tiến-trình, khởi động cùng Windows chuyển sang spec
> 002 (Tauri) và được nghiên cứu ở plan 002. Phương án egui/`tray-icon`/`tauri-winrt-notification` trước đây
> bị bỏ. Lõi chỉ giữ các mục dưới.

| Nhu cầu | Decision | Ghi chú / rủi ro |
|---|---|---|
| Dừng khẩn cấp | **không** dựa vào giao diện: hook bàn phím (R-2) **và** `RegisterHotKey` (đường thứ hai, không bị Windows gỡ theo `LowLevelHooksTimeout`) bật cờ dừng dưới mutex của `Gate` — cổng phát thao tác duy nhất (plan D-1); runner còn kiểm cờ trong mọi vòng chờ (≤ 50 ms) | ≤ 1 s kể cả khi giao diện ẩn/treo/chưa mở hoặc worker kẹt trong lời gọi UIA/HTTP (SC-003); **[M0]** `spike_stop`, `spike_hookloss` |
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
| Khoá phiên | `WTSRegisterSessionNotification` trên cửa sổ message-only ở luồng nền (cùng luồng với hook); dự phòng poll `OpenInputDesktop` mỗi 5 s | **[M0]** `spike_session` (quickstart 0.11): khoá/mở khoá, UAC, RDP ngắt/nối lại, cửa sổ RDP thu nhỏ — đo thời gian từ sự kiện tới lúc ghi/chạy tạm dừng |
| Chụp màn hình | `xcap` 0.9 | issue Windows mở về DPI/vị trí cửa sổ (#143) → **[M0]**; dự phòng GDI `BitBlt` qua `windows` |
| PNG | `image` (chỉ codec png) | che vùng mật khẩu + ghi file |

## R-10. Mô hình luồng (không dùng async runtime)

- **Luồng của bên gọi**: giao diện 002 hoặc CLI giữ `Engine` và nhận `Event` qua channel; lõi không phụ thuộc event loop của giao diện.
- **Luồng input**: hook LL + cửa sổ message-only nhận thông báo khoá phiên; đẩy sự kiện qua channel; giữ cờ
  dừng khẩn cấp và nhận mọi phím tắt của lõi.
- **Luồng worker**: recorder-processor *hoặc* runner (không bao giờ cả hai, FR-025); gọi UIA (COM khởi tạo
  MTA trên luồng này), chụp ảnh, gọi model đồng bộ. Worker có thể kẹt trong lời gọi; nó **không** quyết định dừng:
  mọi thao tác đi qua `Gate` (plan D-1), cờ dừng do luồng input bật, engine phát `cancelled`. Lời gọi UIA chỉ đọc
  dùng `ConnectionTimeout`/`TransactionTimeout`; lời gọi có tác dụng phụ còn dở lúc dừng được log
  `late_effect_possible`.
- **Scheduler**: luồng tick riêng của lõi, kiểm tra mỗi 15 s; logic là máy trạng thái
  thuần (data-model §Occurrence) để test không cần đồng hồ thật.
- Giao tiếp bên gọi ⇄ lõi ⇄ worker: `std::sync::mpsc` (lệnh xuống, sự kiện lên) — không cần crossbeam.

## R-11. Giới hạn đã biết (lõi trả lý do rõ; giao diện 002 / tài liệu hiển thị)

- Ứng dụng chạy quyền Administrator: Windows (UIPI) chặn hook/SendInput/UIA từ tiến trình quyền thường →
  bước trên app đó thất bại ở nấc 1 với lý do rõ ràng; không tự nâng quyền.
- Màn hình khoá / UAC secure desktop: không thao tác được; lịch chờ mở khoá (FR-043).
- App không có UIA và OCR đọc kém: chỉ còn `xy_in_window` (cảnh báo FR-008).

---

## Kết quả M0

### Cổng A — máy dev, lượt 1 (2026-11-27, chạy tự động; **chưa đóng cổng**)

Chỉ các phép đo chạy được không cần người thao tác. Mọi mục dưới đây phải đo lại trên máy công ty trước checkpoint M2.

| # | Kết quả | Trạng thái |
|---|---|---|
| 0.9 | `keyring` 4 ghi `Persist = 3` (ENTERPRISE, đi theo roaming profile), target `LegacyGeneric:target=<user>.<service>`, không chọn được mức lưu. `CredWriteW` trực tiếp với `CRED_PERSIST_LOCAL_MACHINE`: ghi/đọc/xoá đúng, đọc lại `Persist = 2`. **Quyết định**: `secrets.rs` gọi `CredWriteW`/`CredReadW`/`CredDeleteW`, target `teach:<user>`; `keyring` chỉ còn là dev-dependency của `spike_secret`. | Đạt |
| 0.13 | `spike_stop` trên `bench_target` treo 8 s, dừng bằng bộ hẹn giờ sau 1 s, 4 ca × 5 lần: **20/20 không lần gửi nào qua `Gate` bắt đầu sau cờ dừng** (cờ bật đúng lúc nhấn). Tác dụng muộn ~6,9 s sau dừng: `Invoke` 5/5, `SetValue` 5/5 (khi không đặt timeout), SendInput 5/5 (33 cú bấm đã xếp hàng trong cửa sổ treo, xử lý hết khi hết treo). `IUIAutomation2::TransactionTimeout = 5 s` (cần tạo `CUIAutomation8`, `uiautomation` mặc định tạo `CUIAutomation` không có `IUIAutomation2`): cắt được đọc UIA và **cắt được `SetValue` (không có tác dụng muộn)**, **không cắt được `Invoke`** (chặn tới hết treo rồi vẫn có tác dụng). Chưa thử đường dừng bằng phím thật. | Đạt phần `Gate`; xem D-1 |
| 0.14 | `LowLevelHooksTimeout` không có trong HKCU (mặc định). Cho hook ngủ 800 ms: phát hiện lệch `GetLastInputInfo` sau 377–675 ms, hồi lại 125–424 ms, 0 sự kiện mất — **chưa chứng minh Windows đã gỡ hook** (có thể chỉ là trễ). Cửa sổ `RegisterHotKey` chung luồng với hook ⇒ `WM_HOTKEY` bị chặn khi hook kẹt (3 lần đến dồn sau khi hook hết ngủ). Phân bố `lag_ms` với UIA giả 200 ms ở 20 sự kiện/s: hàng đợi tăng tuyến tính (p99 5,9 s) — tải này không dùng để chốt ngưỡng. | Chưa đạt — cần đo tiếp |
| 0.7 | Notepad, 100 % DPI, một màn hình: `xcap` báo khung đúng DWM nhưng ảnh rộng 1901 thay vì 1906 px, chụp 1,4 s; GDI `BitBlt` đúng 1906×1018, 113 ms. Chưa đo 150 % và hai màn hình khác DPI. | Chưa đủ |
| 0.8 | Hook cài được; `RegisterHotKey(Ctrl+Alt+Esc)` đăng ký được khi chạy riêng (lần thất bại 0x80070581 trước đó là do các spike khác chạy song song cùng đăng ký tổ hợp này). Hook LL nuốt tổ hợp trước khi tới `WM_HOTKEY`, nên hai đường không độc lập với cùng một tổ hợp. | Chưa đo 20/20 |
| 0.6a/0.6b (tham khảo, **không** thay máy công ty) | Máy dev chỉ có gói OCR `en-US` (vi-VN không hỗ trợ, đúng R-5). `OcrEngine` gọi được từ exe unpackaged; `spike_ocr` 9/10 nút tiếng Việt khớp sau bỏ dấu ("Huỷ" không đọc được; "Gửi" khớp nhờ "Gti" d=1 — ngưỡng lỏng với nhãn ngắn), 25 ms. | Thông tin |
| 0.1–0.5, 0.11 | Spike đã viết, chạy khởi động/thoát sạch; cần người thao tác. | Chưa đo |

**Hệ quả thiết kế (D-1, D-2 đã duyệt 2026-11-27):**

- **D-1 (đã chốt)**: `Gate` đúng thiết kế. Không phương án nào loại được tác dụng muộn khi app đích treo. Chốt: SendInput mặc định cho `click`/`type` (không chặn worker); `SetValue` chỉ kèm `TransactionTimeout = 5 s` qua `CUIAutomation8`; `Invoke` chỉ khi SendInput không áp dụng được (phần tử ngoài màn hình), ghi giới hạn vào README. Ngưỡng "tác dụng muộn > 1 s" của plan D-1 bị vượt ở mọi cách gửi khi app treo → cần sửa câu chữ plan/spec về `late_effect_possible` thay vì coi là trượt.
- **D-2 (đã chốt, plan D-2)**: cửa sổ `RegisterHotKey` ở **luồng riêng**; khi đăng ký được, hook chỉ quan sát tổ hợp dừng khẩn cấp (không nuốt). Đo lại: `spike_hotkeys --self-test 10` → hook 10/10, `WM_HOTKEY` 10/10 (`--no-hotkey`: hook 10/10, nuốt 10/10); `spike_hookloss --mode force` → `WM_HOTKEY` đến từng lần trong lúc hook kẹt (~415 ms sau khi phát hiện, khi Windows bỏ qua hook quá hạn) thay vì dồn tới khi hook hết kẹt. Còn mở: chứng minh Windows thực sự gỡ hook và thời gian phát hiện ≤ 1 s (FR-053), kiểm Esc không lọt vào ứng dụng đích (0.8, người đo). Tổ hợp mặc định `Ctrl+Alt+Esc` giữ nguyên (không bị chiếm trên máy dev); kiểm lại trên máy công ty. Hệ quả: hai tiến trình lõi không cùng đăng ký được — tiến trình đến sau chỉ còn đường hook, cảnh báo như plan D-2.
- **`RECORD_LAG_LIMIT_MS`**: chưa chốt — cần đo với recorder thật + UIA thật ở tốc độ thao tác người.
