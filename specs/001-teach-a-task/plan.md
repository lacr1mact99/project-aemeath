# Implementation Plan: Teach a Task — lõi (dạy máy làm việc bằng cách làm mẫu)

**Branch**: `001-teach-a-task` | **Date**: 2026-11-22 (cập nhật 2026-11-23: tách giao diện sang 002) | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/001-teach-a-task/spec.md` (+ bản nháp gốc [draft-v0.3.md](./draft-v0.3.md))

## Summary

Lõi Windows cho phép làm mẫu một workflow trên bất kỳ ứng dụng desktop nào, biên dịch thành skill YAML có bộ
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
khiển (lệnh → sự kiện). Hành vi trên giao diện ứng dụng đích: `teach bench` (10 skill chuẩn × 3 chế độ) và
`teach eval` (bộ câu hỏi mẫu) — Nguyên tắc VI.

**Target Platform**: Windows 10 22H2+ / Windows 11, x64, phiên người dùng đang mở khoá; tiến trình
Per-Monitor-V2 DPI aware.

**Project Type**: thư viện lõi + CLI (giao diện desktop ở spec 002)

**Performance Goals**: dừng khẩn cấp: không gửi thao tác sau ≤ 1 s, báo "đã dừng" ≤ 2 s (SC-003, FR-022); mỗi lần hỏi ứng dụng đích ≤ 5 s + 1 lần thử lại (FR-051); mỗi bước ≤ 3 phút thời gian máy tự làm (FR-023); gắn nhãn nền ≤ 2 phút/skill (FR-011); dung lượng trống tối thiểu 500 MB (FR-052); hook bị ngắt phát hiện ≤ 1 s (FR-053); một bước ở nấc 1 ≤ 2 s (p95, ứng dụng đích phản hồi bình thường; chỉ là thông tin — `teach bench` báo vượt nhưng mã thoát không phụ thuộc) gồm chờ giao diện ổn định (một định nghĩa "ổn định" cho cả ghi và chạy: cây phần tử + tiêu đề cửa sổ đích không đổi trong 300 ms liên tiếp; chờ tối đa 1,5 s khi GHI để không làm chậm người dùng, tối đa 5 s khi CHẠY rồi vẫn kiểm tra `expect`);
ghi không làm trễ thao tác người dùng (hook trả ≤ 1 ms); mỗi lần gọi model ≤ 30 s (timeout); sự kiện trạng
thái phát ra ≤ 100 ms sau thay đổi (để giao diện 002 đạt ≤ 1 s).

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

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Nguyên tắc | Cách thiết kế đáp ứng | Trước P0 | Sau P1 |
|---|---|---|---|
| I. Tất định trước | Bậc thang nấc 0→5, nấc 1 không gọi model; compiler sinh selector + `expect` bằng code; nhãn tất định khi thiếu model; `run --no-model` + bench chế độ 1 kiểm chứng 0 lần gọi | ✅ | ✅ |
| II. Trắc nghiệm, chỉ văn bản | 3 loại câu hỏi cố định ([model-protocol](./contracts/model-protocol.md)), cả 3 có trong bộ câu hỏi mẫu (`label_step` chấm theo quy tắc); ngữ cảnh mỗi lần gọi chỉ bước hiện tại + ứng viên (`label_step`: mục tiêu skill + bước hiện tại, không gửi bước trước/sau); bộ phân tích tự viết, hỏi lại 1 lần; không `tools`/ảnh; `json_schema` chỉ là tuỳ chọn bật thêm | ✅ | ✅ |
| III. Kiểm tra bằng code | `expect` là danh sách Condition tất định; lựa chọn của model luôn qua `expect`; tự vá chỉ khi `expect` đạt | ✅ | ✅ |
| IV. Con người nắm quyền | Cờ dừng khẩn cấp trong luồng hook của lõi (không phụ thuộc giao diện, R-8); input không mang `TEACH_MAGIC` → tạm dừng (R-2); `approval`; ngân sách; máy trạng thái lịch không có cạnh `due → running` không qua lựa chọn Chạy ngay (data-model); bề mặt điều khiển không có lệnh "tự đồng ý" ([control-surface](./contracts/control-surface.md)) | ✅ | ✅ |
| V. Riêng tư & bí mật | `is_password` + mặc định coi là bí mật khi không xác định (R-4); che ảnh; bí mật chỉ đọc từ Credential Manager ngay trước khi gõ; bộ lọc dữ liệu gửi model có test (SC-009); API key và bí mật chỉ ghi, không có lệnh đọc ra qua bề mặt điều khiển | ✅ | ✅ |
| VI. Đo được mới tính | M0 spike ([quickstart §M0](./quickstart.md#m0)) trước khi xây; `bench`/`eval`/`probe-model` là lệnh CLI có mã thoát theo ngưỡng; thứ tự M2 trước M3; cổng dấu vân tay: test target riêng `eval_gate` (không chạy trong `cargo test` mặc định, bắt buộc ở bước kiểm tra phát hành `cargo test --test eval_gate`) fail khi prompt / `model/prompt.rs` / `model/parse.rs` / `runner/candidates.rs` đổi mà chưa có kết quả `eval` ≥ 90% trùng dấu vân tay (kết quả tạo trên máy công ty) | ✅ | ✅ |
| VII. Đơn giản | Một crate lõi (thư viện + CLI), không tokio, không DB; bỏ `terminator` (dep nặng), `enigo`, `global-hotkey` (phím tắt qua hook có sẵn); i18n tự viết 2 bảng | ✅ | ✅ |
| Ràng buộc: một mô-đun model | Toàn bộ HTTP + prompt + phân tích ở mô-đun `crates/teach-core/src/model/` (`mod.rs`, `prompt.rs`, `parse.rs`) | ✅ | ✅ |

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
├── draft-v0.3.md        # bản nháp gốc của người dùng
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
├── checklists/requirements.md
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
├── examples/                    # spike M0 (spike_record, spike_input, spike_ocr, spike_capture, spike_hotkeys, spike_secret, spike_probe)
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
│   │   ├── mod.rs               # HTTP, vòng hỏi lại, bộ lọc dữ liệu (FR-033)
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
│       ├── input.rs             # SendInput (TEACH_MAGIC), Unicode
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
    ├── eval.rs                  # chấm bộ câu hỏi mẫu (3 loại), thông báo dữ liệu
    ├── eval_gate.rs             # [[test]] test = false: chỉ chạy bằng `cargo test --test eval_gate` ở bước kiểm tra phát hành
    ├── support/mod.rs           # FakeDesktop (bản giả của trait Desktop)
    └── fixtures/
bench/skills/                    # 10 skill chuẩn (Notepad, Explorer, Calculator, Settings, trình duyệt, Office…)
```

**Structure Decision**: workspace với một crate lõi (thư viện + CLI). Thư viện là thứ giao diện Tauri (002)
nhúng vào cùng tiến trình; CLI `teach` phục vụ nghiệm thu lõi và bộ đo trước khi có giao diện. Bản phát hành
cuối là một exe do plan 002 quyết định (có thể nhúng các lệnh CLI). Ranh giới kiểm thử: mọi thứ ngoài
`platform/` là logic thuần, test được qua trait nhỏ `Desktop` (đọc cây, thao tác, chụp) mà runner nhận vào —
bản thật ở `platform/`, bản giả trong `tests/`. Đây là trait duy nhất có hai implementation, cần cho test.

## Thứ tự triển khai (khớp mốc của spec)

| Mốc | Phạm vi | Cổng ra |
|---|---|---|
| M0 | Spike lõi: hook + UIA + is_password, SendInput tiếng Việt, OCR, xcap DPI, `spike_probe` (T012) với DeepSeek + GLM (máy công ty); CLI `probe-model` ở T068 | Mọi mục **[M0]** của lõi trong research.md có kết quả ghi vào `research.md` §Kết quả M0; quyết định dự phòng được chốt |
| M1 | `trace`, `recorder`, `compiler`, `skill`, `store`, CLI `record` | quickstart §M1 |
| M2 | `runner` nấc 0/1/3/5, `platform/*`, `engine` (lệnh/sự kiện chạy), CLI `run --no-model`, `bench --mode 1`, dừng khẩn cấp | quickstart §M2 (10/10, 0 lần gọi model, ≤ 1 s) |
| M3 | `model`, `eval`, nấc 2/4, tự vá + duyệt tự vá, `bench --mode 2/3` | quickstart §M3 (≥ 90%, ≥ 8/10, 0 bấm sai lọt) → chọn model mặc định |
| M4 | → spec 002 (giao diện đầy đủ, dùng `engine`) | quickstart của 002 |
| M5 | `scheduler` (quy tắc + lịch sử + yêu cầu "hỏi lịch"), CLI `schedule` | quickstart §M5 (mô phỏng); kịch bản trên máy thật với khay/hộp hỏi ở 002 |

## Complexity Tracking

Không có vi phạm constitution cần biện minh.
