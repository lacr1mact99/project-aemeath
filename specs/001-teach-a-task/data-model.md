# Data Model: Teach a Task

Toàn bộ dữ liệu là file trong thư mục dữ liệu (mặc định `%LOCALAPPDATA%\teach\`). Không có cơ sở dữ liệu.
Quy ước: **YAML** cho file người dùng đọc/sửa được (skill, cài đặt, lịch); **JSONL** cho dữ liệu chỉ ghi
thêm (bản ghi sự kiện, log lần chạy, lịch sử lịch). Định dạng chi tiết ở [contracts/](./contracts/).

## Bố cục thư mục

```text
%LOCALAPPDATA%\teach\
├── settings.yaml                 # Cài đặt (không chứa API key)
├── schedules.yaml                # Danh sách lịch
├── schedule_history.jsonl        # Mỗi lần lịch kích hoạt một dòng
├── traces\<trace_id>\
│   ├── meta.yaml                 # mô tả kết quả, thời điểm, máy, kích thước màn hình
│   ├── events.jsonl              # contracts/trace-format.md
│   └── shots\NNNN{a|b}.png       # ảnh trước/sau (vùng mật khẩu đã che)
├── skills\<skill_name>\
│   ├── skill.yaml                # contracts/skill-format.md
│   └── shots\                    # ảnh trước/sau của từng bước (copy từ trace khi biên dịch)
└── runs\<skill_name>\<run_id>\
    ├── run.jsonl                 # contracts/storage-files.md
    └── shots\
```

`<run_id>` = thời điểm bắt đầu `YYYYMMDD-HHMMSS` (giờ địa phương), thêm hậu tố `-2`, `-3` nếu trùng.
Thư mục dữ liệu được đổi trong Cài đặt; con trỏ tới thư mục đó luôn nằm ở vị trí mặc định
(`%LOCALAPPDATA%\teach\location.yaml`) để ứng dụng tìm được khi khởi động.

## Thực thể

### Trace (bản ghi)

| Trường | Kiểu | Ràng buộc |
|---|---|---|
| id | chuỗi | `YYYYMMDD-HHMMSS`, duy nhất |
| goal | chuỗi | mô tả kết quả người dùng nhập; bắt buộc, ≤ 500 ký tự |
| started_at / ended_at | thời điểm ISO 8601 | ended_at − started_at ≤ 10 phút (FR-005) |
| stop_reason | enum | `user` \| `time_limit` |
| screen | {width, height, scale} | để quy đổi toạ độ |
| events | danh sách Event | xem contracts/trace-format.md; không chứa ký tự gõ trong ô mật khẩu (FR-006) |

Trace là bất biến sau khi ghi. Có thể xoá sau khi đã biên dịch thành skill (ảnh cần cho editor đã được
copy sang thư mục skill).

### Skill

| Trường | Kiểu | Ràng buộc |
|---|---|---|
| name | chuỗi | duy nhất; `[a-z0-9-]`, 1–64 ký tự; là tên thư mục |
| description | chuỗi | lấy từ `goal` của trace, sửa được |
| status | enum | `draft` \| `ready` |
| source_trace | chuỗi \| null | id trace gốc |
| inputs | danh sách Input | tên duy nhất trong skill |
| steps | danh sách Step | ≥ 1 bước; `id` duy nhất, không bắt buộc liên tục |
| last_confirmed_run | chuỗi \| null | run_id lần chạy thử được xác nhận gần nhất |

**Chuyển trạng thái** (FR-012, FR-017, FR-027):

```text
            biên dịch xong
                 │
                 ▼
  ┌──────────► draft ── chạy thử thành công + người dùng bấm Xác nhận ──► ready
  │                                                                       │
  └──────── người dùng sửa bước (thêm/xoá/sắp xếp/sửa trường) ◄───────────┘
            (tự vá và hoàn tác tự vá KHÔNG đổi trạng thái)
```

Chỉ skill `ready` được gắn vào lịch (FR-039). Skill về `draft` thì các lịch của nó bị tạm ngưng (không
kích hoạt) kèm lý do (giao diện 002 hiển thị); chuyển lại `ready` thì lịch tự hoạt động lại.

### Input (tham số đầu vào)

| Trường | Kiểu | Ràng buộc |
|---|---|---|
| name | chuỗi | `[a-z_][a-z0-9_]*`, duy nhất trong skill; không được là `secret` |
| description | chuỗi | |
| example | chuỗi | gợi ý khi nhập giá trị |

Mọi `{{name}}` trong `text`/`expect` của bước MUST trỏ tới một Input đã khai báo (kiểm tra khi lưu).

### Step (bước)

| Trường | Kiểu | Ràng buộc |
|---|---|---|
| id | số nguyên | duy nhất trong skill |
| intent | chuỗi | một dòng, ≤ 120 ký tự |
| action | enum | `click` \| `type` \| `hotkey` \| `scroll` \| `wait` \| `launch` |
| window | WindowMatch | bắt buộc trừ `wait` |
| target | danh sách Selector | bắt buộc với `click`, `type`, `scroll`; thứ tự = thứ tự ưu tiên |
| text | chuỗi | chỉ với `type`; có thể chứa `{{input}}` và `{{secret:key}}` |
| keys | chuỗi | chỉ với `hotkey`, ví dụ `Ctrl+S` |
| scroll | {dx, dy} | chỉ với `scroll` |
| wait_ms | số | chỉ với `wait`, ≤ 60 000 |
| launch | {path, args} | chỉ với `launch` |
| expect | danh sách Condition | tất cả phải đúng; rỗng = chỉ cần hành động không lỗi, và bước bị cấm nấc 2/4 + tự vá (FR-020) |
| approval | bool | mặc định false |
| skip_if | Condition \| null | |
| warnings | danh sách chuỗi | `coordinates_only` (FR-008), `text_not_recorded` (không xác định được ô có phải mật khẩu, research R-4), `unverifiable` (`expect` rỗng — FR-020: không sửa chữa bằng model, không tự vá); tính lại mỗi lần lưu skill, chỉ để hiển thị |

**Selector** — đúng một trong các dạng sau, kèm siêu dữ liệu tự vá:

| Dạng | Trường |
|---|---|
| theo mã | `automation_id` (+ tuỳ chọn `role`) |
| theo vai trò + tên | `role`, `name` |
| theo chữ nhận dạng | `ocr_text` |
| theo vị trí | `xy_in_window: [fx, fy]`, 0 ≤ fx, fy ≤ 1 |

Siêu dữ liệu: `added_by_run: <run_id>` và `pending_review: true` khi selector do tự vá thêm vào
(FR-027). Danh sách "đã tự vá" cuối lần chạy = các selector có `pending_review: true`. Người dùng bấm
"Giữ" → bỏ `pending_review`; bấm "Hoàn tác" → xoá selector đó. Cả hai không đổi `status`.

**WindowMatch**: `process` (tên exe, so khớp không phân biệt hoa thường), `title_contains` (tuỳ chọn),
`process_path` (tuỳ chọn, dùng để mở ứng dụng nếu chưa chạy ở nấc 0).

**Condition** — đúng một trong:
`window_title_contains: <chuỗi>` · `element_exists: <Selector không có xy>` ·
`element_absent: <Selector không có xy>` · `target_value_equals: <chuỗi>` ·
`focused_process: <tên exe>`. Chuỗi có thể chứa `{{input}}`, không được chứa `{{secret:...}}`.

### Secret reference (tham chiếu bí mật)

Chỉ là tên khoá trong `{{secret:<key>}}`; giá trị nằm trong Windows Credential Manager dưới service
`teach` và user `secret:<key>`. Không có file nào chứa giá trị. Thiếu khoá khi chạy → dừng trước bước
đó, yêu cầu người dùng nhập (lưu vào Credential Manager), không gửi gì ra ngoài.
Bí mật và API key MUST được lưu với mức lưu trữ chỉ trên máy (`CRED_PERSIST_LOCAL_MACHINE`), không dùng mức
`ENTERPRISE` (mức này đi theo roaming profile) — cùng lý do với FR-007. **[M0]** xác nhận `keyring` cho chọn mức
này; nếu không, gọi `CredWriteW` trực tiếp trong `secrets.rs`.

### Run (lần chạy)

| Trường | Kiểu | Ràng buộc |
|---|---|---|
| id | chuỗi | xem bố cục |
| skill | chuỗi | tên skill |
| trigger | enum | `manual` \| `schedule:<schedule_id>` \| `trial` |
| inputs | map tên → giá trị | giá trị tham số; bí mật không bao giờ có mặt |
| status | enum | xem dưới |
| budget | {max_minutes, max_model_calls, max_repairs_per_step} | chụp từ cài đặt lúc bắt đầu |
| model_calls | số | ≤ max_model_calls |

**Trạng thái lần chạy**:

```text
running ⇄ paused(reason)        reason ∈ {user_input, approval, stuck, missing_secret}
running → succeeded | failed(reason) | cancelled
(ứng dụng mở lại thấy run chưa kết thúc) → interrupted
```

`failed.reason` ∈ {`time_budget`, `model_budget`, `approval_timeout`, `stuck_timeout`,
`rejected`, `launch_failed`, `no_responder`} (`no_responder`: yêu cầu người dùng không thể được trả lời — FR-050). Lần chạy `trial` thành công mở hộp "Xác nhận skill này chạy đúng?".

**Giữ log** (FR-026): sau mỗi lần chạy kết thúc, giữ `keep_runs_per_skill` (mặc định 50) thư mục mới
nhất trong `runs\<skill>\`, xoá phần còn lại; không đụng run đang `running`/`paused`.

### Schedule (lịch)

| Trường | Kiểu | Ràng buộc |
|---|---|---|
| id | chuỗi | duy nhất, sinh tự động |
| skill | chuỗi | skill phải tồn tại; chỉ kích hoạt khi skill `ready` |
| inputs | map | đủ giá trị cho mọi Input của skill |
| when | biểu thức cron (5 trường, giờ địa phương) | giao diện 002 sinh từ mẫu hằng ngày/hằng tuần/mỗi N phút |
| wait_window_minutes | số | mặc định 120 (thời hạn chờ) |
| enabled | bool | |

**Một lần kích hoạt (Occurrence)** — ghi vào `schedule_history.jsonl`:

```text
due ──(máy khoá)──► waiting_unlock ──(mở khoá)──┐
 │                                              ▼
 └──────────────────────────────────────────► asking
                                                │
       ┌───────────── Chạy ngay ────────────────┼── Hoãn N phút ──► postponed ──(hết N phút)──► asking
       ▼                                        ├── Bỏ qua ──► skipped
   (đang có skill khác chạy? → queued → asking) ├── không trả lời, người dùng đang dùng máy
       ▼                                        │      → hỏi lại khi máy rảnh (vẫn asking)
   running → succeeded | failed                 └── hết wait_window → missed
```

Bất biến: không có đường nào từ `due` tới `running` mà không qua lựa chọn "Chạy ngay" của người dùng
(FR-041). Lần kích hoạt mới của cùng lịch đến khi lần cũ chưa xong → lần cũ được ghi `missed` và thay bằng
lần mới (không chồng hộp hỏi).

### Settings (cài đặt)

| Trường | Mặc định |
|---|---|
| model.base_url, model.name | rỗng (chưa cấu hình → không gọi model) |
| model.api_key | **không có trong file**; Credential Manager service `teach`, user `model-api-key` |
| model.timeout_seconds | 30 |
| model.data_notice_accepted | false (FR-037) |
| language | `vi` hoặc `en`; mặc định theo ngôn ngữ Windows, không khớp thì `vi` |
| hotkeys.emergency_stop / record_pause / record_stop | `Ctrl+Alt+Esc` / `Ctrl+Alt+P` / `Ctrl+Alt+S` |
| budget.max_minutes / max_model_calls / max_repairs_per_step | 30 / 20 / 2 |
| schedule.idle_minutes / ask_timeout_minutes / approval_timeout_minutes | 3 / 5 / 30 |
| keep_runs_per_skill | 50 |
| start_with_windows | false (lõi chỉ lưu giá trị; việc đăng ký khởi động cùng Windows thuộc spec 002) |

### Offline eval case (câu hỏi mẫu)

| Trường | Kiểu |
|---|---|
| id | chuỗi |
| kind | `pick_element` \| `diagnose` \| `label_step` |
| input | đúng nội dung phần "user" sẽ gửi model (contracts/model-protocol.md) |
| expected | câu trả lời đúng, ví dụ `{"choice": 3}`; với `label_step` là quy tắc chấm `{"language": "vi", "must_contain": ["File name:"], "must_not_contain": ["BaoCao-2026"]}` (luôn kèm: phân tích được, một dòng, 1–120 ký tự) |

Bộ mặc định nhúng trong binary; người dùng/nhóm phát triển có thể thêm file `eval\*.jsonl` trong thư mục
dữ liệu. Kết quả: tỉ lệ đúng theo từng `kind` và tổng (SC-004 ≥ 90%).

**Kết quả có dấu vân tay** (`eval\results\<thời điểm>.json` trong thư mục dữ liệu; bản dùng cho phát hành sao vào
`crates/teach-core/assets/eval/last_result.json`): `{fingerprint, model_name, prompt_version, scores: {pick_element,
diagnose, label_step, total}, cases, at}`. `fingerprint` = FNV-1a 64-bit (tự viết, không thêm crate) trên nội dung
`assets/prompts/*`, `src/model.rs` và `src/runner/resolve.rs` (chuẩn hoá xuống dòng LF). Kết quả chỉ chứa tên
model, điểm và id ca — không chứa câu hỏi, câu trả lời hay địa chỉ máy chủ.
