# Implementation Plan: Teach a Task — lõi (dạy máy làm việc bằng cách làm mẫu)

**Branch**: `001-teach-a-task` | **Date**: 2026-11-22 (cập nhật 2026-11-23: tách giao diện sang 002) | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/001-teach-a-task/spec.md` (+ bản nháp gốc v0.3, xem lịch sử git: commit `d6e355c`)

## Summary

Lõi Windows cho phép làm mẫu một workflow trên ứng dụng desktop có UIA (phạm vi nghiệm thu MVP: spec §Assumptions "Ứng dụng đích"), biên dịch thành skill YAML có bộ
chọn phần tử UIA và điều kiện kết quả, rồi chạy lại theo bậc thang tất định (nấc 0–5). Model công ty (chỉ văn
bản, có thể yếu) chỉ được hỏi câu trắc nghiệm khi bộ chọn hỏng hoặc kết quả sai. Lịch là lời nhắc luôn hỏi
trước. Lõi là một thư viện Rust kèm CLI `teach` để nghiệm thu không cần giao diện; giao diện Tauri (spec
[002](../002-tauri-gui/spec.md)) nhúng thư viện này qua [bề mặt điều khiển](./contracts/control-surface.md).
Kỹ thuật: Rust thuần, không async runtime trong lõi, UIA qua `uiautomation`, input qua hook/SendInput tự viết
trên crate `windows`, model qua `ureq`.

## Technical Context

**Language/Version**: Rust stable ≥ 1.89 (MSRV cao nhất trong các dep lõi: `serde-saphyr`), edition 2024,
target `x86_64-pc-windows-msvc`

**Primary Dependencies**: `uiautomation` 0.25, `windows` 0.62 (hook LL, SendInput, OCR WinRT, WTS, cửa sổ
message-only), `xcap` 0.9, `image` (png), `ureq` 3 + `rustls-platform-verifier`, `serde`, `serde_json`,
`serde-saphyr` 2, `keyring` 4, `croner`, `chrono`. Lý do: [research.md](./research.md). Thư viện giao diện,
khay, thông báo, một-tiến-trình, khởi động cùng Windows thuộc plan 002.

**Storage**: file trong `%LOCALAPPDATA%\teach\` — YAML (skill, cài đặt, lịch), JSONL (bản ghi, log, lịch sử
lịch), PNG (ảnh). Bí mật trong Windows Credential Manager. Xem [data-model.md](./data-model.md).

**Testing**: `cargo test` cho logic thuần (không cần giao diện thật): phân tích câu trả lời model, bộ lọc dữ
liệu gửi model, dựng ứng viên, compiler (trace → skill), kiểm tra hợp lệ skill, thay thế tham số, máy trạng
thái runner với lớp UIA giả, máy trạng thái lịch với đồng hồ và trạng thái khoá máy giả, giữ log, bề mặt điều
khiển (lệnh → sự kiện). Hành vi trên giao diện ứng dụng đích: `teach bench` (10 skill chuẩn × chế độ 1/2, 10 skill trên `bench_target` × chế độ 3 — Constitution VI v1.2.0) và
`teach eval` (bộ câu hỏi mẫu) — Nguyên tắc VI.

**Target Platform**: Windows 10 22H2+ / Windows 11, x64, phiên người dùng đang mở khoá; tiến trình
Per-Monitor-V2 DPI aware.

**Project Type**: thư viện lõi + CLI (giao diện desktop ở spec 002)

**Performance Goals**: dừng khẩn cấp: không gửi thao tác sau ≤ 1 s, báo "đã dừng" ≤ 2 s (SC-003, FR-022); mỗi lần hỏi ứng dụng đích ≤ 5 s + 1 lần thử lại (FR-051); mỗi bước ≤ 3 phút thời gian máy tự làm (FR-023); gắn nhãn nền ≤ 2 phút/skill (FR-011); dung lượng trống tối thiểu 500 MB (FR-052); hook bị ngắt phát hiện ≤ 1 s (FR-053); một bước ở nấc 1 ≤ 2 s (p95, ứng dụng đích phản hồi bình thường; chỉ là thông tin — `teach bench` báo vượt nhưng mã thoát không phụ thuộc) gồm chờ giao diện ổn định (một định nghĩa "ổn định" cho cả ghi và chạy: cây phần tử + tiêu đề cửa sổ đích không đổi trong 300 ms liên tiếp; chờ tối đa 1,5 s khi GHI để không làm chậm người dùng, tối đa 5 s khi CHẠY rồi vẫn kiểm tra `expect`);
ghi không làm trễ thao tác người dùng (hook trả ≤ 1 ms); mỗi lần gọi model ≤ 30 s (timeout); sự kiện trạng
thái phát ra ≤ 100 ms sau thay đổi (để giao diện 002 đạt ≤ 1 s); chờ cửa sổ sau khi mở ứng dụng ≤ 30 s
(FR-018); mỗi lần OCR ≤ 5 s (FR-004); biên dịch ≤ 10 s cho bản ghi 10 phút (FR-011); hỏi lịch ≤ 30 s sau giờ hẹn
(FR-041, tick 15 s); thân câu trả lời model ≤ 256 KB (FR-032). Ảnh: PNG vùng cửa sổ đích, ghi xuống đĩa ngay;
vùng có cạnh dài > 1920 px thu nhỏ còn 1920 (ảnh chỉ để xem; OCR dùng ảnh gốc trong bộ nhớ rồi bỏ). M0 (T009) đo
MB/phút ghi và MB/lần chạy để ghi vào research.md; sàn bảo vệ là 500 MB trống (FR-052) + giữ 50 lần chạy (FR-026).
Lõi chạy nền chờ lịch: chỉ luồng tick 15 s và luồng hook, không đánh thức máy (FR-043).

**Cách đo** (mỗi mục tiêu): dừng khẩn cấp, `launch_failed`, OCR/UIA quá hạn, trần bước, ngân sách → test
`runner_ladder`/`engine_run` với `FakeDesktop` + đồng hồ giả, và trên máy thật theo **bảng số lần thử dừng khẩn cấp**
ở quickstart §Bảng dừng khẩn cấp (M2: 20 lần gồm app treo và UIA đang `Invoke`; M3: 6 lần khi chờ model treo); sự kiện ≤ 100 ms → `engine_run` (100 lần chuyển trạng thái); nấc 1 p95
và thời gian mỗi skill → `teach bench --mode 1` ghi vào báo cáo bench; hook ≤ 1 ms, MB/phút → spike M0
(`spike_record`), ghi research.md §Kết quả M0; biên dịch ≤ 10 s → test `compiler` với trace mẫu 10 phút; hỏi lịch
≤ 30 s, máy ngủ, chỉnh đồng hồ → `tests/scheduler.rs` (quickstart §M5); độ trễ model, OCR trên ảnh Windows công ty
→ `probe-model`/spike M0 **trên máy công ty**.

**Constraints**: chỉ văn bản tới model, tổng độ dài messages ≤ 2.000 (số ký tự ÷ 4) mỗi lần gọi (FR-030); không phụ thuộc structured output/tool
calling; không gửi ảnh, bí mật, clipboard, văn bản đã gõ; chạy được hoàn toàn khi không có model/mạng;
không chạy hai skill cùng lúc; mọi lần chạy theo lịch cần người dùng chọn Chạy ngay; lõi không phụ thuộc
giao diện (dừng khẩn cấp, phím tắt ghi chạy trong lõi).

**Dev vs target environment**: máy phát triển **không** thuộc mạng nội bộ công ty. Không dùng mạng, proxy,
chứng chỉ hay cấu hình của máy này làm căn cứ triển khai; không hard-code giá trị mạng. Phần model được test
bằng máy chủ giả trên `127.0.0.1`. Các mục M0 về mạng/model và về ảnh Windows của công ty (gói OCR, bộ gõ,
Office) phải đo trên máy công ty (research.md, đầu file).

**Scale/Scope**: một người dùng/máy; hàng chục skill, mỗi skill ≤ ~100 bước; bản ghi ≤ 10 phút;
log giữ 50 lần chạy/skill; thông điệp lõi 2 ngôn ngữ.

## Quyết định kỹ thuật phải chốt ở M0 (trước khi xây runner phát thao tác và tự vá)

**D-1. Tách quyết định dừng khỏi lời gọi có thể treo** (FR-022, SC-003). Worker gọi UIA và HTTP đồng bộ nên có
thể kẹt; quyết định dừng vì vậy không được nằm trên worker:

- **Một cổng phát thao tác duy nhất** `platform/input.rs::Gate`: mọi `SendInput`, `Invoke`, `SetValue`,
  `Toggle`… đi qua `Gate::dispatch(op)`, giữ một mutex ngắn: kiểm cờ dừng → ghi `sent` → gọi. Cờ dừng được bật
  bởi luồng hook (hoặc đường dự phòng D-2) **dưới cùng mutex**, nên sau khi cờ bật không lần gửi mới nào bắt đầu.
  `Gate` tự theo dõi phím/nút đang giữ; **luồng hook** (không phải worker) gọi `Gate::release_all_held()` ngay khi
  dừng.
- **Engine** (không phải worker) chuyển run sang `stopping`, phát `cancelled(emergency_stop)` ≤ 2 s; worker cũ khi
  thoát khỏi lời gọi treo thấy cờ, bỏ kết quả, không gửi gì thêm, rồi nhả `busy` (`start_run` trả `busy` tới lúc đó).
- **Lời gọi có tác dụng phụ đã bắt đầu** (`Invoke`/`SetValue` vào app treo) không thu hồi được và có thể có tác dụng
  muộn. Mốc "không thao tác nào được gửi sau 1 s" tính theo lúc **bắt đầu** lời gọi qua `Gate`; log ghi
  `late_effect_possible` cho lời gọi còn dở lúc dừng. Lời gọi chỉ đọc đặt `IUIAutomation2::ConnectionTimeout` /
  `TransactionTimeout` (cùng ngân sách 5 s của FR-051).
- **[M0] spike `spike_stop`** (T087): đo trên app thử treo có kiểm soát (a) đọc UIA treo, (b) `Invoke` treo, (c)
  `SetValue` treo, (d) SendInput liên tục — mỗi ca: thời điểm cờ bật → lần gửi cuối, tác dụng muộn có xảy ra không và
  sau bao lâu, `TransactionTimeout` có cắt được lời gọi có tác dụng phụ không. **Quyết định dự phòng**: nếu tác dụng
  muộn > 1 s sau khi dừng xảy ra ở (b)/(c), mặc định đổi sang SendInput cho `click`/`type` và chỉ dùng
  `Invoke`/`SetValue` khi SendInput không áp dụng được (phần tử ngoài màn hình), ghi giới hạn vào README.
- **Đã chốt sau M0 (2026-11-27, research.md §Kết quả M0 0.13)**: `Gate` 20/20 không gửi sau cờ dừng; mọi cách gửi
  (`Invoke`, `SetValue`, SendInput) đều có tác dụng muộn ~6,9 s khi app đích treo. Mặc định **SendInput** cho
  `click`/`type` (không chặn worker); `SetValue` chỉ kèm `TransactionTimeout = 5 s` qua `CUIAutomation8` (timeout
  huỷ được lời gọi, không còn tác dụng muộn); `Invoke` chỉ khi SendInput không áp dụng được (phần tử ngoài màn hình).
  Mốc 1 s của SC-003 là "không lần gửi **mới** nào"; tác dụng muộn của lời gọi/input đã gửi trước lúc dừng không
  thu hồi được — lõi ghi `late_effect_possible` và README nêu giới hạn này.

**D-2. Hook bị gỡ và gán dữ liệu ghi cho đúng hành động** (FR-053, FR-003). Windows gỡ hook LL quá
`LowLevelHooksTimeout` mà không báo; heuristic "thời điểm input lệch" hay cài lại định kỳ chưa chứng minh được
1 s. Thiết kế:

- **Dừng khẩn cấp có hai đường độc lập**: hook LL **và** `RegisterHotKey` (WM_HOTKEY, không bị gỡ theo
  `LowLevelHooksTimeout`) trên cửa sổ message-only ở **luồng riêng**, không chung luồng hook (M0: chung luồng thì
  `WM_HOTKEY` kẹt theo hook). Hook LL chạy trước `RegisterHotKey`, nên khi `RegisterHotKey` thành công hook **không
  nuốt** tổ hợp dừng khẩn cấp mà chỉ quan sát (hệ thống tự giữ phím tắt đã đăng ký khỏi ứng dụng đích); cả hai đường
  gọi `Gate::stop()` (idempotent). `RegisterHotKey` thất bại (tổ hợp bị tiến trình khác giữ, kể cả tiến trình lõi thứ
  hai) → cảnh báo khi khởi động, hook nuốt tổ hợp như các phím tắt khác. Đã đo (`spike_hotkeys --self-test 10`):
  hook 10/10 + `WM_HOTKEY` 10/10 khi đăng ký được; hook 10/10 khi không đăng ký.
- Giám sát hook: cài lại hook mỗi 1 s khi đang ghi/chạy (rẻ, không cần đoán đã mất hay chưa) **và** đối chiếu
  `GetLastInputInfo` với sự kiện cuối từ hook; lệch > 1 s khi đang chạy → coi là đã mất trong khoảng đó → tạm dừng
  run (prompt `user_input`, vì không biết người dùng có chạm không); không cài lại được → `input_guard_lost`.
- Gán dữ liệu: recorder đo `lag_ms` cho mỗi sự kiện; vượt `RECORD_LAG_LIMIT_MS` hoặc focus đã đổi → đánh dấu
  `uncertain` (contracts/trace-format.md); compiler không sinh selector/`expect` từ dữ liệu đó.
- **[M0] spike `spike_hookloss`** (T088): ép mất hook (luồng hook ngủ > `LowLevelHooksTimeout`) khi máy rảnh, khi có
  input liên tục, khi chuyển focus giữa lúc gõ, khi bấm liên tiếp 20 lần/giây; đo thời gian phát hiện và số sự kiện
  mất; với recorder thật + UIA chậm, đo phân bố `lag_ms` để chốt `RECORD_LAG_LIMIT_MS`. **Không đạt 1 s thì
  không được tự nới FR-053** (MUST, Constitution IV không thương lượng): phải sửa spec — và constitution nếu chạm
  IV — trước khi bắt đầu M2; `RegisterHotKey` là đường dừng thứ hai bắt buộc theo FR-053, không thay cho việc phát
  hiện thao tác của người dùng.

**D-3. OCR cùng với kiểu đóng gói CLI** (FR-004, R-5). `Windows.Media.Ocr` có thể đòi *package identity* khi gọi
từ ứng dụng desktop; CLI `teach.exe` hiện unpackaged. T008 có **hai cổng riêng**: (1) OCR chạy được trong đúng kiểu
phân phối (exe unpackaged, và kiểu phân phối plan 002 dự kiến); (2) `ocr_text` tìm đúng ≥ 9/10 nhãn tiếng Việt.
Cổng (1) trượt → chọn trước khi xây phần dựa vào OCR: (a) sparse package cấp identity cho exe, (b) Tesseract
`vie` (thêm dep C, Nguyên tắc VII), hoặc (c) bỏ `ocr_text` khỏi MVP — app không UIA chỉ còn `xy_in_window`
(FR-008). Recorder/runner không phụ thuộc OCR cho tới khi cổng (1) đạt.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Nguyên tắc | Cách thiết kế đáp ứng | Trước P0 | Sau P1 |
|---|---|---|---|
| I. Tất định trước | Bậc thang nấc 0→5, nấc 1 không gọi model; compiler sinh selector + `expect` bằng code; nhãn tất định khi thiếu model; `run --no-model` + bench chế độ 1 kiểm chứng 0 lần gọi | ✅ | ✅ |
| II. Trắc nghiệm, chỉ văn bản | 3 loại câu hỏi cố định ([model-protocol](./contracts/model-protocol.md)), cả 3 có trong bộ câu hỏi mẫu (`label_step` chấm theo quy tắc); ngữ cảnh mỗi lần gọi chỉ bước hiện tại + ứng viên (`label_step`: chỉ bước hiện tại, không gửi mục tiêu skill hay bước trước/sau); bộ phân tích tự viết, hỏi lại 1 lần; không `tools`/ảnh; `json_schema` chỉ là tuỳ chọn bật thêm | ✅ | ✅ |
| III. Kiểm tra bằng code | `expect` là danh sách Condition tất định, kiểm cả trước và sau khi gửi; lựa chọn của model chỉ được chấp nhận và tự vá khi bước `verified` (điều kiện sai → đúng, FR-020); hành động đã gửi không tự lặp (FR-028); `ready` gắn với dấu vân tay thực thi của lần chạy thử (FR-017) | ✅ | ✅ |
| IV. Con người nắm quyền | Cờ dừng khẩn cấp trong luồng hook của lõi (không phụ thuộc giao diện, R-8); input không mang `TEACH_MAGIC` → tạm dừng (R-2); `approval`; ngân sách; máy trạng thái lịch không có cạnh `due → running` không qua lựa chọn Chạy ngay (data-model); bề mặt điều khiển không có lệnh "tự đồng ý" ([control-surface](./contracts/control-surface.md)) | ✅ | ✅ |
| V. Riêng tư & bí mật | `is_password` + mặc định coi là bí mật khi không xác định (R-4); che ảnh, không xác định được vùng thì bỏ ảnh, không ghi ảnh từ bước gõ bí mật tới hết lần chạy (FR-006, FR-016) — log ghi `shot_omitted` theo §Quy trình Constitution v1.2.0; bộ lọc áp cho mọi request kể cả hỏi lại và cho câu trả lời trước khi ghi log (FR-033); bí mật chỉ đọc từ Credential Manager ngay trước khi gõ; bộ lọc dữ liệu gửi model có test (SC-009); API key và bí mật chỉ ghi, không có lệnh đọc ra qua bề mặt điều khiển | ✅ | ✅ |
| VI. Đo được mới tính | M0 spike ([quickstart §M0](./quickstart.md#m0)) trước khi xây; `bench`/`eval`/`probe-model` là lệnh CLI có mã thoát theo ngưỡng; thứ tự M2 trước M3; cổng dấu vân tay: test target riêng `eval_gate` (không chạy trong `cargo test` mặc định, bắt buộc ở bước kiểm tra phát hành `cargo test --test eval_gate`) fail khi prompt / `model/prompt.rs` / `model/parse.rs` / `model/redact.rs` / `runner/candidates.rs` / bộ câu hỏi mặc định / trình chấm đổi mà chưa có kết quả `eval` ≥ 90% `release_set` trùng dấu vân tay (kết quả tạo trên máy công ty); bench chế độ 2/3 khẳng định đúng nấc được đo và chấm bằng phép kiểm tra độc lập với `expect`; chế độ 3 chạy trên 10 skill của `bench_target` (hộp thoại thuộc chính app đích) — được phép từ Constitution v1.2.0; **M0 hai cổng**: M1/M2 bắt đầu sau cổng A khi cổng B (OCR, giao thức model — máy công ty) chưa đạt, theo ngoại lệ "việc song song không phụ thuộc" của Constitution §Quy trình — M1/M2 không dùng model, và OCR có đường rơi tất định theo FR-004 (coi như không đọc được chữ); phần dựa vào hai ẩn số đó (T026, nhánh OCR, US4) vẫn chờ cổng B | ✅ | ✅ |
| VII. Đơn giản | Một crate lõi (thư viện + CLI), không tokio, không DB; bỏ `terminator` (dep nặng), `enigo`, `global-hotkey` (phím tắt qua hook có sẵn); i18n tự viết 2 bảng | ✅ | ✅ |
| Ràng buộc: một mô-đun model | Toàn bộ HTTP + prompt + phân tích + bộ lọc ở mô-đun `crates/teach-core/src/model/` (`mod.rs`, `prompt.rs`, `parse.rs`, `redact.rs`) | ✅ | ✅ |

Không có vi phạm → bảng Complexity Tracking để trống. (Lựa chọn Tauri cho giao diện và quan hệ với Nguyên tắc
VII được xử lý trong plan 002.)

**Lệch so với bản nháp (đã ghi trong research.md, không vi phạm constitution)**: `uiautomation` thay
`terminator-rs`/`terminator-workflow-recorder`; hook LL + SendInput thay `rdev`/`enigo`; `ureq` thay
`reqwest`+`tokio`; `serde-saphyr` thay `serde_yaml`; `croner` thay `cron`; OCR tiếng Việt chuyển sang so
khớp bỏ dấu và tiêu chí M0 OCR được diễn giải lại (R-5); giao diện chuyển từ egui sang Tauri ở spec 002.

## Project Structure

### Documentation (this feature)

```text
specs/001-teach-a-task/
├── spec.md              # đặc tả lõi (đã clarify, đã tách giao diện)
├── plan.md              # file này
├── research.md          # Phase 0
├── data-model.md        # Phase 1
├── quickstart.md        # Phase 1 — kịch bản nghiệm thu M0–M5 (M4 phần lõi)
├── contracts/
│   ├── control-surface.md # lệnh + sự kiện giữa lõi và giao diện 002 / CLI
│   ├── skill-format.md
│   ├── trace-format.md
│   ├── model-protocol.md
│   └── storage-files.md   # settings, schedules, run log, eval, dòng lệnh
├── checklists/            # requirements.md, performance.md, security.md
└── tasks.md             # Phase 2 (/speckit-tasks — đã tạo)
```

### Source Code (repository root)

```text
Cargo.toml                       # workspace; plan 002 thêm thành viên app Tauri
rust-toolchain.toml              # stable + rustfmt + clippy
deny.toml                        # cargo deny: advisories + licenses
crates/teach-core/
├── Cargo.toml                   # lib "teach_core" + bin "teach" (CLI)
├── build.rs                     # nhúng manifest cho bin CLI
├── teach.exe.manifest           # PerMonitorV2, asInvoker
├── README.md                    # build, lệnh CLI, vị trí dữ liệu, giới hạn đã biết
├── examples/                    # spike M0 (spike_record, spike_input, spike_ocr, spike_capture, spike_hotkeys, spike_secret, spike_probe, spike_stop, spike_hookloss, spike_session); bench_target (app thử có kiểm soát: treo theo lệnh, tự bật hộp thoại của chính nó)
├── assets/
│   ├── prompts/                 # system prompt + ví dụ mẫu: pick_element, diagnose, label_step
│   └── eval/
│       ├── default.jsonl        # bộ câu hỏi mẫu nhúng trong binary
│       └── last_result.json     # kết quả eval ≥ 90% có dấu vân tay, tạo bằng `teach eval --publish` (máy công ty)
├── src/
│   ├── lib.rs
│   ├── engine.rs                # bề mặt điều khiển: Engine (lệnh) + luồng Event (contracts/control-surface.md)
│   ├── bin/teach.rs             # CLI: record | run | eval | bench | probe-model | skill | schedule | config
│   ├── skill.rs                 # kiểu Skill/Step/Selector/Condition, kiểm tra hợp lệ, thay thế {{...}}
│   ├── trace.rs                 # kiểu sự kiện bản ghi
│   ├── recorder.rs              # sự kiện thô → events.jsonl (gộp phím, tra UIA, che mật khẩu, state)
│   ├── compiler.rs              # trace → skill (tách bước, selector, expect, nhãn)
│   ├── runner/
│   │   ├── mod.rs               # bậc thang nấc 0–5, ngân sách, tạm dừng/dừng, tự vá, yêu cầu người dùng
│   │   ├── resolve.rs           # khớp selector nấc 1, so khớp bỏ dấu
│   │   ├── candidates.rs        # bộ lọc ứng viên cho pick_element/diagnose (≤ 40, lọc vai trò, xếp độ giống tên) — nằm trong dấu vân tay eval
│   │   └── check.rs             # đánh giá Condition (expect, skip_if)
│   ├── model/                   # MỘT mô-đun model
│   │   ├── mod.rs               # HTTP, vòng hỏi lại
│   │   ├── redact.rs            # bộ lọc dữ liệu gửi model + lọc câu trả lời trước khi ghi (FR-033) — nằm trong dấu vân tay eval
│   │   ├── prompt.rs            # dựng câu hỏi theo loại, cắt ứng viên ≤ 2.000 (FR-030), PROMPT_VERSION — nằm trong dấu vân tay eval
│   │   └── parse.rs             # bộ phân tích câu trả lời — nằm trong dấu vân tay eval
│   ├── eval.rs                  # chấm bộ câu hỏi mẫu; probe-model
│   ├── bench.rs                 # 3 chế độ bộ đo (corrupt selector, chèn hộp thoại)
│   ├── scheduler.rs             # máy trạng thái lần kích hoạt (thuần) + luồng tick; tính lần kế tiếp
│   ├── store.rs                 # đường dẫn, đọc/ghi YAML/JSONL, giữ log 50 lần
│   ├── secrets.rs               # Credential Manager (chỉ ghi/xoá ra ngoài lõi; đọc chỉ khi gõ/gọi model)
│   ├── i18n.rs                  # thông điệp lõi vi/en (nhãn tất định, lý do tạm dừng, lỗi)
│   └── platform/                # mọi lời gọi Windows nằm ở đây
│       ├── uia.rs               # bọc uiautomation: đọc cây, phần tử tại điểm, is_password, value, invoke
│       ├── hooks.rs             # hook LL, phím tắt (dừng khẩn cấp, tạm dừng/dừng ghi), khoá phiên, máy rảnh
│       ├── input.rs             # Gate: cổng phát thao tác duy nhất (SendInput TEACH_MAGIC, Unicode, Invoke/SetValue), cờ dừng, nhả phím (D-1)
│       ├── capture.rs           # chụp ảnh, che vùng
│       ├── ocr.rs               # Windows.Media.Ocr
│       └── window.rs            # tìm/đưa cửa sổ lên trước, mở ứng dụng
└── tests/
    ├── model_parse.rs           # <think>, chữ thừa, JSON lệch, ngoài tập cho phép, hỏi lại (máy chủ giả 127.0.0.1)
    ├── privacy.rs               # bộ lọc dữ liệu gửi model, log không chứa bí mật (SC-009)
    ├── compiler.rs              # trace mẫu → skill mẫu (fixtures/)
    ├── skill_format.rs          # quy tắc hợp lệ, round-trip YAML
    ├── runner_ladder.rs         # bậc thang với UIA giả + model giả
    ├── scheduler.rs             # máy trạng thái lịch, khoá máy, bỏ lỡ, DST, không chạy khi chưa đồng ý
    ├── engine.rs                # lựa chọn ngoài choices → invalid, busy, không có lệnh trả key (không cần runner)
    ├── engine_run.rs            # start_run → run_state ≤ 100 ms, prompt hết hạn (lần tay 60 phút / theo lịch 30 phút), disk_full
    ├── store.rs                 # từ chối thư mục mạng/đồng bộ, ghi nguyên tử
    ├── skill_lifecycle.rs       # draft ⇄ ready, tự vá không đổi trạng thái
    ├── runner_repair.rs         # nấc 2/4, tự vá, cấm sửa chữa khi expect rỗng
    ├── eval.rs                  # chấm bộ câu hỏi mẫu (3 loại), notice chưa chấp nhận → không có request
    ├── eval_gate.rs             # [[test]] test = false: chỉ chạy bằng `cargo test --test eval_gate` ở bước kiểm tra phát hành
    ├── support/mod.rs           # FakeDesktop (bản giả của trait Desktop)
    └── fixtures/
bench/skills/                    # 10 skill chuẩn (Notepad, Explorer, Calculator, Settings, trình duyệt, Office…); mỗi skill kèm reset.ps1 (đặt lại trạng thái app) + check.ps1 (kiểm tra kết quả độc lập với expect)
bench/dialog-skills/             # 10 skill trên bench_target cho chế độ 3 (hộp thoại của chính app đích), cũng có reset/check
```

**Structure Decision**: workspace với một crate lõi (thư viện + CLI). Thư viện là thứ giao diện Tauri (002)
nhúng vào cùng tiến trình; CLI `teach` phục vụ nghiệm thu lõi và bộ đo trước khi có giao diện. Bản phát hành
cuối là một exe do plan 002 quyết định (có thể nhúng các lệnh CLI). Ranh giới kiểm thử: mọi thứ ngoài
`platform/` là logic thuần, test được qua trait nhỏ `Desktop` (đọc cây, thao tác, chụp) mà runner nhận vào —
bản thật ở `platform/`, bản giả trong `tests/`. Đây là trait duy nhất có hai implementation, cần cho test.

## Thứ tự triển khai (khớp mốc của spec)

| Mốc | Phạm vi | Cổng ra |
|---|---|---|
| M0 | Spike lõi: hook + UIA + is_password, SendInput tiếng Việt, OCR **theo kiểu phân phối** (D-3), xcap DPI, dừng khi lời gọi treo (D-1), mất hook + gán dữ liệu ghi (D-2), khoá phiên/RDP, `spike_probe` (T012) với DeepSeek + GLM (máy công ty); CLI `probe-model` ở T068 | **Cổng A (máy dev)**: spike input/UIA/chụp/phím tắt/dừng/mất hook đạt, D-1, D-2 chốt → mở nền tảng Windows và runner phát thao tác. **Cổng B (máy công ty)**: OCR theo kiểu phân phối (D-3) và giao thức model (0.10) → mở phần OCR và US4/M3. Đo lại các mục Dev trên máy công ty là điều kiện của cổng ra M2. So prompt Anh–Việt đo trong M3 (quickstart §M3). Kết quả ghi `research.md` §Kết quả M0 |
| M1 | `trace`, `recorder`, `compiler`, `skill`, `store`, CLI `record` | quickstart §M1 |
| M2 | `runner` nấc 0/1/3/5, `platform/*`, `engine` (lệnh/sự kiện chạy), CLI `run --no-model`, `bench --mode 1`, dừng khẩn cấp | quickstart §M2 (10/10, 0 lần gọi model, ≤ 1 s) **và** đo lại các mục M0 cổng A trên máy công ty đạt |
| M3 | `model`, `eval`, nấc 2/4, tự vá + duyệt tự vá, `bench --mode 2/3` | quickstart §M3 (≥ 90%, ≥ 8/10, 0 bấm sai lọt) → chọn model mặc định |
| M4 | → spec 002 (giao diện đầy đủ, dùng `engine`) | quickstart của 002 |
| M5 | `scheduler` (quy tắc + lịch sử + yêu cầu "hỏi lịch"), CLI `schedule` | quickstart §M5 (mô phỏng); kịch bản trên máy thật với khay/hộp hỏi ở 002 |

## Complexity Tracking

Không có vi phạm constitution cần biện minh.
