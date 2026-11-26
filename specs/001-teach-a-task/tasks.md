---
description: "Task list for 001-teach-a-task (lõi)"
---

# Tasks: Teach a Task — lõi

**Input**: Design documents from `specs/001-teach-a-task/` — [plan.md](./plan.md), [spec.md](./spec.md),
[research.md](./research.md), [data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: Có. Constitution §Quy trình yêu cầu test tự động cho logic không tầm thường (phân tích câu trả lời
model, sinh selector/điều kiện, bậc thang, tính lịch, che bí mật). Test chạy bằng `cargo test`, không cần giao
diện thật; phần model dùng máy chủ giả trên `127.0.0.1` (máy dev không thuộc mạng công ty).

**Organization**: Theo user story của spec 001. Đường dẫn gốc mã: `crates/teach-core/`.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: chạy song song được (file khác nhau, không phụ thuộc task chưa xong)
- **[Story]**: US1…US6 theo spec 001

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Khởi tạo workspace và crate theo plan.md §Project Structure

- [ ] T001 Create workspace root `Cargo.toml` (`[workspace] members = ["crates/teach-core"]`, `resolver = "2"`, `[workspace.package] edition = "2024"`, `rust-version = "1.89"`) and `.gitignore` with `target/`
- [ ] T002 Create `crates/teach-core/Cargo.toml`: `[lib] name = "teach_core"`, `[[bin]] name = "teach" path = "src/bin/teach.rs"`; dependencies `uiautomation = "0.25"` (features `event`, `pattern`), `windows = "0.62"` (features for Win32 UI hooks/input/WindowsAndMessaging, RemoteDesktop (WTS), Graphics.Gdi, Media.Ocr, Graphics.Imaging, Storage.Streams, System.Threading), `xcap = "0.9"`, `image` (default-features off, feature `png`), `ureq = "3"`, `rustls-platform-verifier`, `serde` (derive), `serde_json`, `serde-saphyr = "2"`, `keyring = "4"`, `croner` (pinned exact version), `chrono`
- [ ] T003 [P] Add `crates/teach-core/build.rs` + `crates/teach-core/teach.exe.manifest` embedding Per-Monitor-V2 DPI awareness and `asInvoker` for bin `teach`
- [ ] T004 [P] Add `rust-toolchain.toml` (stable, components rustfmt + clippy) and `deny.toml` for `cargo deny check advisories licenses` (security checklist CHK004)
- [ ] T005 [P] Create empty module skeleton matching plan.md: `crates/teach-core/src/{lib.rs,engine.rs,skill.rs,trace.rs,recorder.rs,compiler.rs,model.rs,eval.rs,bench.rs,scheduler.rs,store.rs,secrets.rs,i18n.rs}`, `src/runner/{mod.rs,resolve.rs,check.rs}`, `src/platform/{mod.rs,uia.rs,hooks.rs,input.rs,capture.rs,ocr.rs,window.rs}`, `src/bin/teach.rs`, `tests/support/mod.rs`; `cargo build` passes

---

## Phase 2: M0 Spike (đo trước khi xây — Constitution VI)

**Purpose**: Trả lời các mục **[M0]** trong research.md. Mỗi spike là một example độc lập; kết quả ghi vào
`research.md` §Kết quả M0. Cột "Máy" ở quickstart.md §M0 quyết định chạy ở máy dev hay **máy công ty**.

- [ ] T006 [P] Spike ghi: `crates/teach-core/examples/spike_record.rs` — hook LL chuột/phím trên luồng riêng + `uiautomation` `element_from_point`/`get_focused_element`/`is_password`; in sự kiện kèm role/name/automation_id; đo quickstart 0.1, 0.2, 0.3 (ghi lại app nào `is_password` không xác định)
- [ ] T007 [P] Spike input: `crates/teach-core/examples/spike_input.rs` — SendInput `KEYEVENTF_UNICODE` gõ "Ươ ạ ẫ ỹ" với `dwExtraInfo = TEACH_MAGIC`; hook đồng thời phân biệt input không mang dấu (kể cả khi bộ gõ Việt bật); đo 0.4, 0.5
- [ ] T008 [P] Spike OCR (**máy công ty**): `crates/teach-core/examples/spike_ocr.rs` — `OcrEngine` vi-VN nếu `IsLanguageSupported` else en-US; so khớp bỏ dấu 10 nút nhãn tiếng Việt; ghi kết quả `Get-WindowsCapability -Online | ? Name -like 'Language.OCR*'`; đo 0.6 (đạt ≥ 9/10)
- [ ] T009 [P] Spike chụp: `crates/teach-core/examples/spike_capture.rs` — `xcap` chụp cửa sổ đích + cửa sổ cùng tiến trình ở 150% DPI và hai màn hình khác DPI, tô kín vùng cửa sổ tiến trình khác chồng lên; đo 0.7; nếu `xcap` sai khung → ghi quyết định dùng GDI `BitBlt`
- [ ] T010 [P] Spike phím tắt: `crates/teach-core/examples/spike_hotkeys.rs` — bắt `Ctrl+Alt+Esc`, `Ctrl+Alt+P`, `Ctrl+Alt+S` trong hook khi ứng dụng khác (kể cả toàn màn hình) đang focus, nuốt tổ hợp khỏi ứng dụng đích; đo 0.8 (20/20)
- [ ] T011 [P] Spike bí mật: `crates/teach-core/examples/spike_secret.rs` — ghi/đọc/xoá qua `keyring` và xác nhận mức lưu `CRED_PERSIST_LOCAL_MACHINE` (data-model §Secret reference); nếu `keyring` không cho chọn → ghi quyết định gọi `CredWriteW` trực tiếp
- [ ] T012 Spike model (**máy công ty**, cả DeepSeek và GLM): `crates/teach-core/examples/spike_probe.rs` — `ureq` POST `{base_url}/chat/completions` theo contracts/model-protocol.md §Giao thức; đo tương thích OpenAI, `response_format: json_schema`, vị trí phần suy nghĩ, p50/p95, giới hạn ngữ cảnh, 20 câu JSON một dòng (đạt ≥ 19/20); TLS qua kho chứng chỉ Windows + proxy hệ thống (không hard-code giá trị mạng)
- [ ] T013 Ghi toàn bộ kết quả T006–T012 và các phương án dự phòng đã chọn vào `specs/001-teach-a-task/research.md` §Kết quả M0; cập nhật plan.md nếu đổi thư viện

**Checkpoint**: Mọi mục [M0] có kết quả; quyết định dự phòng đã chốt.

---

## Phase 3: Foundational (Blocking Prerequisites)

**Purpose**: Kiểu dữ liệu, lưu trữ, nền tảng Windows và bề mặt điều khiển — mọi story đều cần

**⚠️ CRITICAL**: Chưa xong phase này thì chưa làm story nào

- [ ] T014 Implement `crates/teach-core/src/store.rs`: thư mục dữ liệu mặc định `%LOCALAPPDATA%\teach\` (FR-007) với con trỏ `location.yaml` ở vị trí mặc định; bố cục thư mục theo data-model §Bố cục; ghi YAML nguyên tử (file tạm + rename); ghi thêm JSONL kèm flush; `Settings` theo contracts/storage-files.md `settings.yaml` (không có trường API key; trường lạ → từ chối tải); đổi thư mục dữ liệu MUST từ chối ổ mạng/UNC, ổ ánh xạ mạng, thư mục OneDrive/đồng bộ đám mây đã biết và thư mục cá nhân chuyển hướng lên máy chủ, kèm lý do (FR-038)
- [ ] T015 [P] Test `crates/teach-core/tests/store.rs`: từ chối `\\server\share`, ổ ánh xạ, đường dẫn trong `%OneDrive%`; ghi nguyên tử không để file dở; settings có trường lạ bị từ chối
- [ ] T016 Implement `crates/teach-core/src/skill.rs` theo data-model §Skill/Step/Selector/Condition và contracts/skill-format.md: serde `deny_unknown_fields`, `version: 1`; `name` "`[a-z0-9-]`, 1–64 ký tự"; `intent` "một dòng, ≤ 120 ký tự"; `wait_ms` "≤ 60 000"; `xy_in_window` "0 ≤ fx, fy ≤ 1"; Input `name` "`[a-z_][a-z0-9_]*`, duy nhất trong skill; không được là `secret`"; quy tắc hợp lệ 1–6 của skill-format (một dạng selector chính, `{{secret:k}}` chỉ trong `text` với `k` theo `[a-z0-9_]+`, `expect`/`skip_if` không chứa bí mật và không dùng `xy_in_window`, không escape `{{`); hàm `validate() -> Vec<FieldError>`; hàm tính `warnings` (`coordinates_only`, `text_not_recorded`, `unverifiable` khi `expect` rỗng — FR-020); thay thế `{{input}}`
- [ ] T017 [P] Test `crates/teach-core/tests/skill_format.rs`: round-trip YAML ví dụ trong skill-format.md; mỗi quy tắc hợp lệ 1–6 có một ca sai; `unverifiable` xuất hiện khi `expect: []`
- [ ] T018 [P] Implement `crates/teach-core/src/trace.rs`: kiểu sự kiện theo contracts/trace-format.md (`click`, `scroll`, `type` có/không `secret`, `hotkey`, `window_focus`, `clipboard` chỉ có `op`, `state` với `new_elements` ≤ 50 và không có Value); đọc/ghi `events.jsonl` và `meta.yaml`
- [ ] T019 [P] Implement `crates/teach-core/src/secrets.rs`: service `teach`, user `secret:<key>` và `model-api-key`, mức lưu theo kết quả T011; API công khai chỉ `set`, `delete`, `exists`; `get` là `pub(crate)` và chỉ runner/model gọi
- [ ] T020 [P] Implement `crates/teach-core/src/i18n.rs`: enum khoá thông điệp + hai bảng `vi`/`en` (nhãn tất định `Bấm "X" (Role)`/`Click "X" (Role)`, `Gõ vào ô "X"`/`Type into "X"`, `Nhấn K`/`Press K`, lý do tạm dừng, mã lỗi); ngôn ngữ mặc định theo ngôn ngữ giao diện Windows, không khớp thì `vi` (FR-048)
- [ ] T021 Implement `crates/teach-core/src/platform/hooks.rs` (từ spike T006/T007/T010): luồng nền có message loop + cửa sổ message-only; hook LL chuột/phím đẩy sự kiện thô (vk, scancode, flags, `dwExtraInfo`, toạ độ, thời điểm) qua channel và trả ngay; cờ `AtomicBool` dừng khẩn cấp; khớp phím tắt cấu hình được và nuốt tổ hợp; nhận `WM_WTSSESSION_CHANGE` (khoá/mở khoá) và phát hiện secure desktop; `idle_for()` qua `GetLastInputInfo`
- [ ] T022 [P] Implement `crates/teach-core/src/platform/input.rs`: SendInput chuột/phím với `dwExtraInfo = TEACH_MAGIC`; gõ văn bản bằng `KEYEVENTF_UNICODE`; `release_all_held()` nhả mọi phím/nút đang giữ (FR-022)
- [ ] T023 [P] Implement `crates/teach-core/src/platform/uia.rs`: khởi tạo COM MTA cho luồng worker; `element_at(x,y)`, `focused_element()`, `is_password() -> Option<bool>` (ba trạng thái), `find_all(window, condition)`, `value()`, `set_value()`, `invoke()`; struct `ElementInfo {name, role, automation_id, class_name, path, rect, enabled, focused, selected, checked}`
- [ ] T024 [P] Implement `crates/teach-core/src/platform/window.rs`: tìm cửa sổ theo `process` (không phân biệt hoa thường) + `title_contains`; nhiều cửa sổ cùng khớp → trả về "khớp nhiều" để runner đi nhánh sửa chữa (spec Edge Cases); đưa lên trước; `launch(path,args)`; `foreground() -> WindowInfo` dùng cho kiểm tra trước mỗi lần gửi input (FR-049)
- [ ] T025 [P] Implement `crates/teach-core/src/platform/capture.rs` (theo T009): chụp vùng cửa sổ đích + cửa sổ/hộp thoại cùng tiến trình; tô kín vùng bị cửa sổ tiến trình khác chồng lên; tô kín danh sách rect (ô mật khẩu); ghi PNG; không bao giờ chụp toàn màn hình (FR-003)
- [ ] T026 [P] Implement `crates/teach-core/src/platform/ocr.rs` (theo T008): OCR vùng quanh điểm → danh sách `(text, rect)`; ngôn ngữ vi-VN nếu có, else en-US
- [ ] T027 Define trait `Desktop` in `crates/teach-core/src/platform/mod.rs` (đọc cây cửa sổ, tìm cửa sổ, foreground, gửi input, chụp, OCR, trạng thái khoá/idle) với bản thật dùng T021–T026, và bản giả `FakeDesktop` (kịch bản dựng sẵn, ghi lại mọi input đã gửi) in `crates/teach-core/tests/support/mod.rs`
- [ ] T028 Implement `crates/teach-core/src/engine.rs` theo contracts/control-surface.md: `Engine` + enum lệnh + `Receiver<Event>`; mã lỗi `invalid|busy|not_found|not_ready|model_*|io`; một worker duy nhất + khoá liên tiến trình named mutex `Local\teach-core-run` để CLI và giao diện không chạy hai skill cùng lúc (FR-025); `UserPrompt {id, kind, context, choices, deadline}`; `answer_prompt` từ chối lựa chọn ngoài `choices`; hết hạn/không trả lời được → xử lý an toàn theo FR-050 (lần chạy `failed(no_responder | approval_timeout | stuck_timeout)`, hỏi lịch → không chạy); sự kiện phát ≤ 100 ms sau thay đổi
- [ ] T029 [P] Test `crates/teach-core/tests/engine.rs`: lựa chọn ngoài `choices` → `invalid` và trạng thái không đổi; prompt hết hạn không bao giờ thành `approve`/`run_now`; lệnh thứ hai khi worker bận → `busy`; sự kiện `run_state` phát ≤ 100 ms sau khi trạng thái đổi (đo trên 100 lần chuyển trạng thái với `FakeDesktop`); không có lệnh nào trả API key/giá trị bí mật (kiểm tra bằng danh sách lệnh)
- [ ] T030 Implement CLI skeleton `crates/teach-core/src/bin/teach.rs`: phân tích tham số thủ công (không thêm crate) cho các lệnh con ở contracts/storage-files.md §Dòng lệnh; hiển thị `prompt` ở console và đọc lựa chọn từ stdin; stdin đóng/không phải console → trả lời theo FR-050 (không bao giờ đồng ý)

**Checkpoint**: Nền tảng sẵn sàng — các story bắt đầu được.

---

## Phase 4: User Story 1 - Dạy một việc bằng cách làm mẫu (Priority: P1) 🎯 MVP

**Goal**: Ghi một workflow và nhận skill nháp chạy được, không cần model (FR-001…FR-013).

**Independent Test**: `teach record "Lưu ghi chú hôm nay"` → làm 5 bước trên Notepad → nhấn `Ctrl+Alt+S` →
`skills\luu-ghi-chu-hom-nay\skill.yaml` hợp lệ, ~5 bước, mỗi bước có intent, ≥ 1 selector, `expect` hoặc cảnh
báo `unverifiable`; ký tự gõ vào ô mật khẩu không có trong `events.jsonl` (quickstart §M1, SC-001).

- [ ] T031 [P] [US1] Add fixtures `crates/teach-core/tests/fixtures/traces/notepad_save/{events.jsonl,meta.yaml}` và skill kỳ vọng `crates/teach-core/tests/fixtures/skills/notepad_save.yaml` (gồm một sự kiện `type` `secret: true`, một bước chỉ có `xy_in_window`)
- [ ] T032 [P] [US1] Test `crates/teach-core/tests/compiler.rs`: fixture trace → đúng skill kỳ vọng; thứ tự selector `automation_id` → `role+name` → `ocr_text` → `xy_in_window`; `expect` sinh từ sự kiện `state`; bước chỉ có vị trí mang `coordinates_only`; không model → nhãn tất định theo ngôn ngữ cài đặt
- [ ] T033 [US1] Implement `crates/teach-core/src/recorder.rs`: nhận sự kiện thô từ T021; bỏ di chuột thuần tuý và mọi sự kiện trên cửa sổ của chính tiến trình; gộp phím liên tiếp vào cùng phần tử thành `type`, tổ hợp có Ctrl/Alt/Win thành `hotkey`; tra UIA tại điểm bấm, OCR khi thiếu tên/mã (FR-004); ảnh trước/sau qua T025; sự kiện `state` khi giao diện ổn định (≤ 1,5 s); `is_password` = true **hoặc không xác định** → `{"kind":"type","secret":true}`, không ghi ký tự, tô kín `element.rect` (FR-006, research R-4); `clipboard` chỉ ghi `op`; tạm dừng/tiếp tục (không ghi khi tạm dừng); tự dừng ở 10 phút (FR-005); khoá phiên/secure desktop → không ghi, không chụp; phát `recording_warning` (`coordinates_only`, `text_not_recorded`)
- [ ] T034 [US1] Implement `crates/teach-core/src/compiler.rs`: tiền xử lý + tách bước theo hành động có nghĩa; sinh selector theo thứ tự ưu tiên và `window` (`process`, `title_contains`, `process_path`); sinh `expect` tất định từ `state` (FR-010); nhãn tất định qua i18n (FR-011 khi không có model); tính `warnings`; tên skill = slug từ `goal`, duy nhất; copy ảnh bước sang `skills\<name>\shots\`; `status: draft` (FR-012); 0 bước → `NoSteps`, không tạo skill
- [ ] T035 [US1] Add lệnh ghi vào `crates/teach-core/src/engine.rs`: `start_recording(goal)` (goal bắt buộc, ≤ 500 ký tự), `pause_recording`, `resume_recording`, `stop_recording -> SkillName | NoSteps`; sự kiện `recording_state`, `recording_warning`; phím tắt `record_pause`/`record_stop` từ settings gắn vào hook
- [ ] T036 [US1] Implement `teach record "<mô tả>"` in `crates/teach-core/src/bin/teach.rs`: in trạng thái và cảnh báo từng bước; mã thoát 0 có skill, 3 không có bước
- [ ] T037 [P] [US1] Test `crates/teach-core/tests/privacy.rs` (phần ghi): recorder với nguồn sự kiện giả — ô `is_password = true` và ô không xác định đều không để lại ký tự trong `events.jsonl`; `clipboard` không có nội dung; rect mật khẩu nằm trong danh sách che của ảnh

**Checkpoint**: US1 chạy được độc lập (quickstart §M1).

---

## Phase 5: User Story 2 - Chạy skill an toàn, không cần model (Priority: P2)

**Goal**: Bậc thang nấc 0/1/3/5, phê duyệt, dừng khẩn cấp, tạm dừng khi người dùng chạm (FR-018…FR-026, FR-049, FR-050).

**Independent Test**: `teach bench bench\skills --mode 1` với model tắt → 10/10 `succeeded`, 0 lần gọi model;
`Ctrl+Alt+Esc` dừng ≤ 1 s 10/10 lần (quickstart §M2, SC-002, SC-003).

- [ ] T038 [P] [US2] Implement `crates/teach-core/src/runner/check.rs`: đánh giá `window_title_contains`, `element_exists`, `element_absent`, `target_value_equals`, `focused_process` (sau khi thay `{{input}}`) cho `expect` (tất cả phải đúng) và `skip_if`
- [ ] T039 [P] [US2] Implement `crates/teach-core/src/runner/resolve.rs`: thử selector theo thứ tự, thành công chỉ khi khớp **đúng một** phần tử; `ocr_text` so khớp không dấu, không phân biệt hoa thường, theo khoảng cách chỉnh sửa (research R-5); `xy_in_window` quy đổi theo khung cửa sổ hiện tại
- [ ] T040 [US2] Implement `crates/teach-core/src/runner/mod.rs` nấc 0, 1, 3, 5: nấc 0 `skip_if` + đưa cửa sổ lên/mở ứng dụng; nấc 1 resolve; nấc 3 hành động → chờ ổn định (cây phần tử + tiêu đề cửa sổ đích không đổi trong 300 ms liên tiếp, tối đa 5 s) → `expect`; nấc 0 gặp "khớp nhiều" cửa sổ → xử lý như selector khớp nhiều; nấc 5 phát prompt `stuck` sau `max_repairs_per_step` (mặc định 2); **trước mỗi lần gửi input** kiểm tra foreground khớp `window` của bước, bước nhập bí mật kiểm tra thêm phần tử focus = phần tử đích, không khớp → không gửi (bí mật → prompt `stuck`) (FR-049); đọc bí mật ngay trước khi gõ, không lưu vào biến log; `approval: true` → prompt `approval`; input không mang `TEACH_MAGIC` → prompt `user_input`; khoá phiên/secure desktop → tạm dừng ngay (FR-022); kiểm tra cờ dừng khẩn cấp trước mỗi thao tác và trong mọi vòng chờ (≤ 50 ms), dừng thì `release_all_held()`; ngân sách `max_minutes` / `max_model_calls` → `failed(time_budget|model_budget)`; bí mật thiếu → prompt `missing_secret`; run chưa kết thúc tìm thấy khi khởi động → `interrupted`
- [ ] T041 [US2] Implement run log in `crates/teach-core/src/runner/mod.rs` + `crates/teach-core/src/store.rs` theo contracts/storage-files.md `run.jsonl` (header, `step` với `rung`, `pause`/`resume`, `model_call`, `self_patch`, `result`; `text` giữ nguyên `{{secret:k}}`); sau mỗi lần chạy giữ `keep_runs_per_skill` (mặc định 50) thư mục mới nhất trong `runs\<skill>\`, không đụng run đang chạy/tạm dừng (FR-026)
- [ ] T042 [P] [US2] Test `crates/teach-core/tests/runner_ladder.rs` với `FakeDesktop`: giao diện không đổi → 0 lần gọi model; `skip_if` thoả → bỏ qua có lý do; `approval` → chờ, `reject` → `failed(rejected)`; `expect` không bao giờ đúng → prompt `stuck` sau 2 lần; cửa sổ khác cướp focus → không input nào gửi vào cửa sổ đó; bước bí mật với focus sai → không gõ bí mật; khoá phiên → tạm dừng; vượt ngân sách thời gian → `failed(time_budget)`; dừng khẩn cấp → không còn input sau cờ, phím được nhả
- [ ] T043 [US2] Add lệnh chạy vào `crates/teach-core/src/engine.rs`: `start_run(name, inputs, trigger)` (thiếu input → `invalid` kèm danh sách và ví dụ), `stop_run`, `answer_prompt`, `list_runs`, `get_run_log`; sự kiện `run_state`, `run_log`, `prompt`, `prompt_closed`, `run_finished`
- [ ] T044 [US2] Implement `teach run <skill> [--input k=v]... [--trial] [--no-model]` in `crates/teach-core/src/bin/teach.rs`; mã thoát 0/1/2
- [ ] T045 [US2] Implement `crates/teach-core/src/bench.rs` chế độ 1 + `teach bench <dir> --mode 1` in `crates/teach-core/src/bin/teach.rs`: chạy mọi skill trong thư mục, in bảng (skill, kết quả, số lần gọi model, thời gian, p95 thời gian mỗi bước so với mục tiêu ≤ 2 s ở nấc 1), mã thoát 0 nếu 10/10 và 0 lần gọi model
- [ ] T046 [US2] Record 10 skill chuẩn bằng `teach record` vào `bench/skills/<name>/skill.yaml` (Notepad, File Explorer, Calculator, Settings, trình duyệt, ứng dụng Office hoặc tương đương — SC-002), mỗi skill có ít nhất một `expect` mỗi bước

**Checkpoint**: US1 + US2 chạy độc lập (quickstart §M2).

---

## Phase 6: User Story 3 - Hoàn thiện skill và đưa vào trạng thái sẵn sàng (Priority: P3)

**Goal**: Tham số, bí mật, phê duyệt, sửa bước, chuyển `draft → ready` (FR-014…FR-017).

**Independent Test**: Đánh dấu tên file của skill Notepad là tham số `ten_file`, bật `approval` bước Lưu,
`teach run luu-ghi-chu-hom-nay --trial --input ten_file=moi.txt` → phê duyệt → xác nhận → `status: ready`.

- [ ] T047 [US3] Add lệnh skill vào `crates/teach-core/src/engine.rs`: `list_skills`, `get_skill`, `save_skill` (gọi `validate()`, trả `[FieldError]`; bước đổi khi `ready` → về `draft`, trả `status_changed`, lịch của skill tạm ngưng kèm lý do — FR-017, FR-039), `delete_skill` (xoá thư mục skill, `runs\<skill>\` và lịch gắn với skill; giữ bí mật — FR-014), `step_image`
- [ ] T048 [US3] Implement helper `mark_parameter(step_id, start, end, Input)` in `crates/teach-core/src/skill.rs`: thay đoạn trong `text` bằng `{{name}}` và thêm Input; không có cơ chế tự gợi ý tham số (FR-015)
- [ ] T049 [US3] Add lệnh bí mật vào `crates/teach-core/src/engine.rs` (`set_secret`, `delete_secret`, `secret_exists`) và xử lý prompt `missing_secret` trong `crates/teach-core/src/runner/mod.rs` (dừng trước bước, chỉ tiếp tục khi khoá đã có)
- [ ] T050 [US3] Implement `confirm_trial(run_id, ok)` in `crates/teach-core/src/engine.rs`: chỉ với run `trial` `succeeded`; `ok` → `status: ready`, `last_confirmed_run = run_id` (FR-017)
- [ ] T051 [P] [US3] Test `crates/teach-core/tests/skill_lifecycle.rs`: `draft → ready` chỉ qua `confirm_trial`; sửa bước của `ready` → `draft`; giữ/hoàn tác tự vá không đổi `status`; `{{x}}` chưa khai báo → `save_skill` lỗi; bí mật không xuất hiện trong `skill.yaml`
- [ ] T052 [US3] Implement `teach skill list|show <skill>` và `teach secret set <key>` (đọc từ stdin không echo) in `crates/teach-core/src/bin/teach.rs`

**Checkpoint**: US1–US3 chạy độc lập.

---

## Phase 7: User Story 4 - Tự sửa chữa bằng model chỉ văn bản (Priority: P4)

**Goal**: Model client, `pick_element`, `diagnose`, `label_step`, tự vá (FR-027…FR-034, FR-020). Đây là story
đầu tiên cần model; US5 dùng lại client của story này.

**Independent Test**: `teach bench bench\skills --mode 2` và `--mode 3` (máy công ty) → mỗi chế độ ≥ 8/10, 0 lần
bấm sai lọt qua `expect` (quickstart §M3, SC-004, SC-005).

- [ ] T053 [US4] Implement HTTP client in `crates/teach-core/src/model.rs` theo contracts/model-protocol.md §Giao thức: `POST {base_url}/chat/completions`, `Authorization: Bearer` từ `secrets::get`, `temperature: 0`, `stream: false`, không gửi `tools`/ảnh/`max_tokens` mặc định; chỉ đọc `choices[0].message.content`; timeout `model.timeout_seconds` (30); mỗi lần gửi (kể cả hỏi lại) ghi một dòng `model_call` vào `run.jsonl` theo contracts/storage-files.md (`kind`, `request` = đúng chuỗi đã gửi sau bộ lọc, `response`, `parsed`, `valid`, `ms`); 401/403 → `auth_error`, mạng/timeout/5xx → `unavailable`; không tự thử lại khi lỗi mạng; TLS qua kho chứng chỉ Windows, luôn kiểm tra chứng chỉ, không có cờ tắt; từ chối `base_url` không phải `https://` trừ `127.0.0.1`/`localhost` (FR-035); không hard-code giá trị mạng; mỗi lần gửi tăng `model_calls`
- [ ] T054 [US4] Implement bộ phân tích câu trả lời in `crates/teach-core/src/model.rs`: xoá `<think>…</think>`/`<thinking>…</thinking>`, lấy đối tượng JSON cân bằng ngoặc đầu tiên (bỏ qua ngoặc trong chuỗi), kiểm tra tập cho phép theo loại; sai → gửi lại đúng một lần với `Invalid answer: <lý do>. Reply with exactly one line of JSON, for example <ví dụ>.`; vẫn sai → `none` / `ask_user` / nhãn tất định (FR-031)
- [ ] T055 [US4] Implement dựng đầu vào + bộ lọc dữ liệu in `crates/teach-core/src/model.rs`: chỉ các trường được phép (intent, hành động, tiêu đề cửa sổ, tiến trình, tên/vai trò/mã/trạng thái phần tử, tên ô đích, **độ dài** văn bản); không bao giờ ảnh, bí mật, clipboard, văn bản đã gõ, giá trị tham số, Value của ô nhập; che chuỗi trùng (không phân biệt hoa thường) với văn bản đã gõ / giá trị tham số / giá trị bí mật bằng `‹đã ẩn›`, chuỗi < 3 ký tự không che trừ bí mật (FR-033); từ chối gọi khi `data_notice_accepted = false` (FR-037)
- [ ] T056 [P] [US4] Write prompts `crates/teach-core/assets/prompts/{pick_element,diagnose,label_step}.txt`: system prompt tiếng Anh + 2–3 ví dụ mẫu có lời giải mỗi loại, đúng định dạng đầu vào ở model-protocol.md; hằng `PROMPT_VERSION` trong `model.rs`, ghi vào log mỗi lần gọi
- [ ] T057 [P] [US4] Test `crates/teach-core/tests/model_parse.rs` với máy chủ HTTP giả trên `127.0.0.1` (std `TcpListener`): `<think>` + JSON, chữ thừa quanh JSON, JSON lệch rồi đúng ở lần hỏi lại, sai cả hai lần → `none`, số ngoài danh sách, 401 → `auth_error`, timeout → `unavailable`, `base_url` `http://` không phải loopback → bị từ chối trước khi gửi
- [ ] T058 [P] [US4] Test `crates/teach-core/tests/privacy.rs` (phần model): request gửi máy chủ giả không chứa bí mật `S3cret!xyz`, giá trị tham số `KHACH-HANG-ABC`, văn bản đã gõ — kể cả khi chúng nằm trong tiêu đề cửa sổ/tên phần tử; `run.jsonl` và mọi file trong thư mục dữ liệu không chứa bí mật (SC-009)
- [ ] T059 [US4] Implement nấc 2 `pick_element` in `crates/teach-core/src/runner/resolve.rs` + `crates/teach-core/src/runner/mod.rs`: tối đa 40 ứng viên từ cửa sổ đích, lọc vai trò tương thích hành động, xếp theo độ giống tên; kết quả luôn qua `expect`; **không dùng ở bước `expect` rỗng** → chuyển thẳng nấc 5 (FR-020)
- [ ] T060 [US4] Implement nấc 4 `diagnose` in `crates/teach-core/src/runner/mod.rs`: `wait` (chờ 2 s rồi kiểm tra lại), `retry`, `dismiss` chỉ phần tử `Button` trong NEW ELEMENTS (FR-029), `ask_user`; bước `approval: true` → `retry` đổi thành `ask_user`, log ghi cả câu trả gốc (FR-028); không gọi ở bước `expect` rỗng; lỗi model → prompt `stuck` kèm nguyên nhân (FR-032)
- [ ] T061 [US4] Implement tự vá in `crates/teach-core/src/runner/mod.rs` + lệnh `pending_patches`/`keep_patch`/`undo_patch` in `crates/teach-core/src/engine.rs`: khi nấc 2 thành công và `expect` đạt, thêm selector (`automation_id` hoặc `role`+`name`) với `added_by_run`, `pending_review: true`, giữ selector cũ, ghi log `self_patch`; không đổi `status` (FR-027); `teach skill patches <skill> [--keep|--undo <step>:<idx>]` in `crates/teach-core/src/bin/teach.rs`
- [ ] T062 [US4] Implement `label_step` in `crates/teach-core/src/compiler.rs` qua `model.rs`: chỉ bước trước/hiện tại/sau, bước gõ chỉ gửi "type N characters into <ô>", tham số `LANGUAGE` theo cài đặt; kết quả một dòng 1–120 ký tự; lỗi/không có model → nhãn tất định (FR-011)
- [ ] T063 [US4] Implement bench chế độ 2/3 in `crates/teach-core/src/bench.rs` + cờ `--corrupt-primary` (xoá `automation_id`, đổi tên selector đầu) và `--inject-dialog-at <step>` (bật hộp thoại `MessageBox` từ luồng phụ trước bước đó) cho `teach run`/`teach bench --mode 2|3`
- [ ] T064 [P] [US4] Test `crates/teach-core/tests/runner_repair.rs` với `FakeDesktop` + máy chủ model giả: chọn đúng → tự vá `pending_review`; chọn sai → `expect` bắt, không coi là thành công; bước `expect` rỗng → không có request nào tới máy chủ, prompt `stuck`; `retry` ở bước `approval` → prompt, hành động không lặp; mất mạng → `stuck` với lý do `unavailable`; `dismiss` số không phải Button → coi là sai định dạng

**Checkpoint**: US4 chạy với model giả (dev) và model thật (máy công ty).

---

## Phase 8: User Story 5 - Cấu hình và kiểm tra model (Priority: P5)

**Goal**: Ba tham số, thông báo dữ liệu, Kiểm tra kết nối, Kiểm tra model (FR-035…FR-037). Phụ thuộc T053–T055.

**Independent Test**: `teach config set-model …` + `teach config set-key` + `teach config accept-notice` →
`teach eval` in tỉ lệ đúng theo loại, mã thoát 0 khi ≥ 90%; đổi model → chạy lại để so sánh.

- [ ] T065 [US5] Add lệnh cài đặt vào `crates/teach-core/src/engine.rs`: `get_settings` (không có key), `save_settings` (kiểm tra phím tắt trùng nhau, thư mục dữ liệu theo T014, `base_url` phải `https://` trừ loopback — FR-035), `set_api_key`, `api_key_set`, `data_notice` (văn bản theo ngôn ngữ, nêu thứ được gửi / không bao giờ gửi / tên phần tử có thể chứa dữ liệu nghiệp vụ), `accept_data_notice`; đổi `base_url` hoặc `name` → đặt lại `data_notice_accepted = false`
- [ ] T066 [US5] Implement `test_connection` in `crates/teach-core/src/model.rs` + `crates/teach-core/src/engine.rs`: phân biệt không kết nối được / sai khoá / không có model đó
- [ ] T067 [US5] Implement `crates/teach-core/src/eval.rs` + `crates/teach-core/assets/eval/default.jsonl` (≥ 20 ca `pick_element`, ≥ 10 ca `diagnose`, ≥ 10 ca `label_step`, toàn dữ liệu giả, định dạng contracts/storage-files.md §Câu hỏi mẫu): `pick_element`/`diagnose` chấm bằng so sánh JSON sau phân tích, `label_step` chấm theo quy tắc ở data-model §Offline eval case (phân tích được, một dòng, 1–120 ký tự, đúng `language`, chứa mọi `must_contain`, không chứa `must_not_contain`); báo cáo theo loại + tổng; ghi kết quả có dấu vân tay vào `eval\results\` (data-model §Kết quả có dấu vân tay); nạp thêm `eval\*.jsonl` trong thư mục dữ liệu; lệnh `run_eval` + sự kiện `eval_progress`/`eval_done`
- [ ] T068 [US5] Implement `teach eval [--cases <file>] [--publish]` (`--publish` sao kết quả ≥ 90% vào `crates/teach-core/assets/eval/last_result.json`) và `teach probe-model` (thay spike T012; in tương thích OpenAI, `json_schema`, phần suy nghĩ, p50/p95, 20 lần JSON một dòng; mã thoát 0 nếu ≥ 19/20) in `crates/teach-core/src/bin/teach.rs`
- [ ] T069 [US5] Implement `teach config set-model --url <u> --name <n>`, `teach config set-key` (stdin, không echo), `teach config accept-notice` (in thông báo, yêu cầu gõ `yes`) in `crates/teach-core/src/bin/teach.rs`
- [ ] T070 [P] [US5] Test `crates/teach-core/tests/eval.rs` với máy chủ giả: tỉ lệ đúng tính đúng theo cả 3 loại (`label_step` sai ngôn ngữ / hai dòng / chứa chuỗi cấm → không đạt); notice chưa chấp nhận → không có request; đổi URL → notice bị đặt lại

- [ ] T081 [US5] Implement cổng dấu vân tay (Constitution VI): hàm `fingerprint()` FNV-1a 64-bit tự viết in `crates/teach-core/src/eval.rs` trên `assets/prompts/*`, `src/model.rs`, `src/runner/resolve.rs` (chuẩn hoá LF); test `crates/teach-core/tests/eval_gate.rs` fail khi `assets/eval/last_result.json` thiếu, dấu vân tay khác dấu vân tay hiện tại, hoặc điểm tổng/từng loại < 90%; thông báo lỗi chỉ dẫn chạy `teach eval --publish` trên máy công ty (không có cờ bỏ qua)

**Checkpoint**: US4 + US5 hoàn chỉnh (quickstart §M3).

---

## Phase 9: User Story 6 - Lên lịch như lời nhắc có sẵn nút chạy (Priority: P6)

**Goal**: Quy tắc lịch luôn hỏi trước, khoá máy, bỏ lỡ, hàng đợi (FR-039, FR-041…FR-045, FR-050).

**Independent Test**: `cargo test --test scheduler` chạy toàn bộ bảng kịch bản quickstart §M5 với đồng hồ,
idle và khoá máy giả; 0 `run_started` khi chưa có `run_now` (SC-008).

- [ ] T071 [US6] Implement kiểu lịch + cron in `crates/teach-core/src/scheduler.rs`: `Schedule {id, skill, inputs, when, wait_window_minutes (mặc định 120), enabled}` theo contracts/storage-files.md `schedules.yaml`; `when` cron 5 trường giờ địa phương qua `croner`; `next_due`, `preview(n)`; lưu `schedules.yaml` qua store
- [ ] T072 [US6] Implement máy trạng thái lần kích hoạt (thuần, nhận `now`, `idle_minutes`, `locked`, `run_active`, `skill_ready`) in `crates/teach-core/src/scheduler.rs` theo data-model §Occurrence: `due`, `waiting_unlock`, `asking`, `postponed`, `queued`, `running`, `skipped`, `missed`; bất biến không có đường `due → running` không qua `run_now`; không trả lời trong `ask_timeout_minutes` (5) khi người dùng đang dùng máy → đóng prompt và hỏi lại khi rảnh `idle_minutes` (3); hết `wait_window_minutes` → `missed`; lần mới đến khi lần cũ chưa xong → lần cũ `missed`; `run_now` chỉ áp dụng cho đúng lần kích hoạt đó; khi lõi khởi động: với mỗi lịch, lần đến hạn gần nhất đã qua còn trong `wait_window_minutes` → `asking`, quá hạn → `missed`, các lần cũ hơn chỉ ghi `missed`, không chạy bù
- [ ] T073 [US6] Implement luồng tick 15 s trong `crates/teach-core/src/engine.rs`: phát `prompt(schedule_ask)`; ngay trước khi bắt đầu chạy sau `run_now`, kiểm tra lại skill còn `ready`, máy mở khoá, không có lần chạy khác; ghi `schedule_history.jsonl` mọi sự kiện; lần chạy theo lịch: prompt phê duyệt/kẹt có `deadline` = `approval_timeout_minutes` (30) → hết hạn `failed(approval_timeout|stuck_timeout)` (FR-044)
- [ ] T074 [P] [US6] Test `crates/teach-core/tests/scheduler.rs`: mọi dòng bảng quickstart §M5 (máy rảnh, đang dùng máy + hết thời hạn hỏi, hoãn 10 phút, khoá rồi mở trong thời hạn, khoá quá thời hạn → `missed`, phê duyệt hết hạn → `failed(approval_timeout)`, skill khác đang chạy → `queued`, skill về `draft` → lịch tạm ngưng, DST không kích hoạt trùng, khởi động lại sau khi ứng dụng tắt 3 ngày với lịch hằng ngày → 1 lần `asking` hoặc `missed` cho lần gần nhất, các lần cũ `missed`, 0 `run_started`); khẳng định 0 `run_started` không có `run_now`
- [ ] T075 [US6] Add lệnh lịch vào `crates/teach-core/src/engine.rs` (`list_schedules` kèm `next_due` và lý do tạm ngưng, `save_schedule` từ chối skill `draft` bằng `not_ready`, `delete_schedule`, `set_schedules_paused`, `schedule_history`, `preview_cron`) và `teach schedule list|add|rm|history|watch` in `crates/teach-core/src/bin/teach.rs` (`watch` chạy vòng lịch ở console, hỏi ở console)

**Checkpoint**: Mọi story của lõi hoàn chỉnh; sẵn sàng cho giao diện spec 002.

---

## Phase 10: Polish & Cross-Cutting Concerns

- [ ] T076 [P] Write `crates/teach-core/README.md`: cách build, các lệnh CLI, vị trí dữ liệu, giới hạn đã biết (research R-11: ứng dụng chạy quyền Administrator, màn hình khoá, ứng dụng không có UIA)
- [ ] T077 Run `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo deny check`, `cargo test` sạch trên toàn workspace
- [ ] T078 Rà soát `specs/001-teach-a-task/checklists/security.md` với code hiện tại; ghi các mục còn mở thành issue hoặc sửa spec
- [ ] T079 Chạy quickstart §M1, §M2, §M4 (phần lõi: dạy → chạy thử → phê duyệt → xác nhận chỉ bằng CLI, SC-007), §M5 trên máy dev; ghi kết quả vào `specs/001-teach-a-task/quickstart.md` (mục kết quả cuối file)
- [ ] T080 (**Máy công ty**) Chạy quickstart §M3 (`teach eval --publish` để tạo `last_result.json` cho T081, `teach bench --mode 2|3`, kiểm tra riêng tư SC-009, ngắt mạng SC-010) với DeepSeek và GLM; chọn model mặc định theo SC-004 và ghi vào `specs/001-teach-a-task/research.md`

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (P1)** → **M0 Spike (P2)** → **Foundational (P3)** → các story → **Polish (P10)**
- Spike chạy song song; T013 chờ T006–T012. Foundational dùng kết quả spike (T021 ← T006/T007/T010, T025 ← T009, T026 ← T008, T019 ← T011)

### User Story Dependencies

- **US1 (P1)**: sau Foundational; không phụ thuộc story khác
- **US2 (P2)**: sau Foundational; chạy bằng skill viết tay được, nhưng T046 cần US1 để ghi skill chuẩn
- **US3 (P3)**: sau US2 (`confirm_trial` cần run `trial`)
- **US4 (P4)**: sau US2 (nấc 2/4 cắm vào bậc thang); T062 sửa compiler của US1 — Constitution VI: US2 (M2) phải đạt trước khi bắt đầu US4 (M3)
- **US5 (P5)**: dùng model client T053–T055 của US4
- **US6 (P6)**: sau US2 + US3 (chỉ skill `ready` được lên lịch)

### Within Each Story

- Fixture/test viết trước và phải fail, rồi mới implement
- `skill.rs`/`trace.rs` trước `compiler.rs`; `check.rs`/`resolve.rs` trước `runner/mod.rs`; runner trước lệnh engine; engine trước CLI

### Parallel Opportunities

- Setup: T003, T004, T005
- Spike: T006–T012 (theo máy dev / máy công ty)
- Foundational: T015, T017, T018, T019, T020, T022–T026, T029 song song sau khi có T014/T016 tương ứng
- US1: T031, T032, T037 cùng lúc; US2: T038, T039, T042; US4: T056, T057, T058, T064; US5: T070, T081 (sau T067); US6: T074
- Sau US2: US3 và US4 có thể song song (khác file chính: `engine.rs` cần phối hợp)

---

## Parallel Example: User Story 4

```text
Task: "Write prompts crates/teach-core/assets/prompts/{pick_element,diagnose,label_step}.txt"   (T056)
Task: "Test crates/teach-core/tests/model_parse.rs với máy chủ giả 127.0.0.1"                    (T057)
Task: "Test crates/teach-core/tests/privacy.rs (phần model)"                                      (T058)
Task: "Test crates/teach-core/tests/runner_repair.rs"                                             (T064)
```

---

## Implementation Strategy

### MVP First (User Story 1 + 2)

1. Setup → M0 Spike → Foundational
2. US1: ghi + biên dịch → kiểm tra quickstart §M1
3. US2: chạy tất định → `bench --mode 1` đạt 10/10, 0 lần gọi model
4. **Dừng và đánh giá**: lõi tất định tự đứng vững (Constitution I, VI) — đây là MVP

### Incremental Delivery

1. US3 → skill `ready`
2. US4 + US5 → tự sửa chữa với model (máy công ty), chọn model mặc định
3. US6 → lịch
4. Spec 002 (giao diện Tauri): chỉ phần không phụ thuộc kết quả M3/M5 (khung app, màn hình danh sách/ghi/chạy dựa trên `engine.rs`) được bắt đầu song song sau US2; nghiệm thu M4 của 002 chỉ sau khi M3 đạt (Constitution §Quy trình: thứ tự mốc)

---

## Notes

- [P] = khác file, không phụ thuộc task chưa xong
- Task ghi **máy công ty** không được đánh dấu xong bằng kết quả chạy trên máy dev
- Mọi task chạm tới ghi, chạy, lịch, log hoặc giao tiếp model: review MUST kiểm tra riêng Nguyên tắc IV và V (Constitution §Quy trình)
- Commit sau mỗi task hoặc nhóm task logic; dừng ở mỗi checkpoint để kiểm tra story độc lập
