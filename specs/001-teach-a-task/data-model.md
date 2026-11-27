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
│   ├── label_log.jsonl           # log lời gọi model `label_step` (ngoài lần chạy; contracts/storage-files.md)
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
| started_at / ended_at | thời điểm ISO 8601 | thời gian ghi (không tính tạm dừng) = ended_at − started_at − paused_ms ≤ 10 phút (FR-005) |
| paused_ms | số | tổng thời gian tạm dừng ghi, ghi vào `meta.yaml`; không tính vào giới hạn 10 phút |
| stop_reason | enum | `user` \| `time_limit` |
| screen | {width, height, scale} | để quy đổi toạ độ |
| events | danh sách Event | xem contracts/trace-format.md; không chứa ký tự gõ trong ô mật khẩu (FR-006) |

Trace là bất biến sau khi ghi. Giữ (FR-054): mặc định tới khi skill có `source_trace` trỏ tới nó bị xoá; ngay
sau biên dịch lõi phát prompt `trace_cleanup` hỏi xoá luôn; người dùng xoá được bất kỳ lúc nào (`delete_trace`),
skill vẫn dùng được vì ảnh cần cho editor đã được copy sang thư mục skill. Trace 0 bước bị xoá ngay. Ảnh lưu PNG,
ghi xuống đĩa ngay khi chụp (FR-003).

### Skill

| Trường | Kiểu | Ràng buộc |
|---|---|---|
| name | chuỗi | duy nhất; `[a-z0-9-]`, 1–64 ký tự; là tên thư mục |
| description | chuỗi | lấy từ `goal` của trace, sửa được |
| status | enum (**tính ra, không lưu trong file**) | `ready` khi `confirmed.fingerprint` = `exec_fingerprint()` hiện tại, ngược lại `draft` (FR-017) |
| source_trace | chuỗi \| null | id trace gốc |
| inputs | danh sách Input | tên duy nhất trong skill |
| steps | danh sách Step | ≥ 1 bước; `id` duy nhất, không bắt buộc liên tục |
| confirmed | {run, fingerprint} \| null | lần chạy thử được xác nhận gần nhất và dấu vân tay thực thi lúc lần chạy đó bắt đầu |

**Dấu vân tay thực thi** `exec_fingerprint()` (FR-017): FNV-1a 64-bit (cùng hàm với dấu vân tay eval) trên JSON
chuẩn hoá (khoá theo thứ tự cố định) của: `inputs[].name`; với từng bước theo thứ tự: `id`, `action`, `window`,
`target` **bỏ selector có `patch`**, `text`, `keys`, `scroll`, `wait_ms`, `launch`, `expect`, `approval`, `repeat_safe`, `skip_if`.
Không gồm: `name`, `description`, `intent`, `warnings`, `source_trace`, `confirmed`, selector tự vá.

**Chuyển trạng thái** (FR-012, FR-017, FR-027):

```text
            biên dịch xong (confirmed = null)
                 │
                 ▼
  ┌──────────► draft ── confirm_trial(run): run trial succeeded
  │                     và run.skill_fingerprint = exec_fingerprint() ──► ready (confirmed = {run, fp})
  │                                                                       │
  └──── exec_fingerprint() đổi (sửa nội dung thực thi qua app hoặc tay) ◄─┘
        (sửa intent/description, tự vá, Giữ/Hoàn tác tự vá KHÔNG đổi dấu vân tay)
```

`status` không bao giờ đọc từ file: mỗi lần tải, lõi tính lại. File có trường `status` → trường lạ → lỗi (FR-047).
Chỉ skill `ready` được gắn vào lịch (FR-039); scheduler tính lại ngay trước khi chạy (FR-041).

**Ghi đồng thời** (FR-013): mọi thao tác ghi `skill.yaml` đi qua `store::update_skill(name, |skill| ...)` — giữ khoá
**liên tiến trình** theo skill (named mutex `Local\teach-skill-<name>`; GUI 002 và CLI có thể chạy cùng lúc), đọc
lại file mới nhất, áp đúng thay đổi của mình, ghi nguyên tử. Skill không còn tồn tại → `not_found`, không tạo lại
(gắn nhãn nền gặp lỗi này thì dừng). Bản chỉnh sửa của người dùng (`save_skill`) mang `revision` = FNV-1a của nội
dung file lúc `get_skill`; khác `revision` hiện tại → `conflict`, không ghi. Gắn nhãn nền: với mỗi bước, chỉ đặt
`intent` khi bước cùng `id` còn tồn tại và `intent` hiện tại vẫn bằng nhãn tất định đã sinh lúc biên dịch;
ngược lại bỏ qua bước đó. Gắn nhãn nền không đổi `status`. Skill về `draft` thì các lịch của nó bị tạm ngưng (không
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
| target | danh sách Selector | bắt buộc với `click`, `type`, `scroll`; không có với `hotkey`, `wait`, `launch`; thứ tự trong file = thứ tự ưu tiên của selector gốc |
| text | chuỗi | chỉ với `type`; có thể chứa `{{input}}` và `{{secret:key}}` |
| keys | chuỗi | chỉ với `hotkey`, ví dụ `Ctrl+S` |
| scroll | {dx, dy} | chỉ với `scroll` |
| wait_ms | số | chỉ với `wait`, ≤ 60 000 |
| launch | {path, args} | chỉ với `launch` |
| expect | danh sách Condition | tất cả phải đúng; rỗng = bước chỉ kết thúc được ở `executed_unverified`/`confirmed_by_user`, bị cấm nấc 2/4 + tự vá (FR-020) |
| approval | bool | mặc định false |
| repeat_safe | bool | mặc định false; chỉ với `click`, `hotkey`, `type`; người dùng tự bật khi lặp lại bước không gây hậu quả (FR-028); compiler/tự vá không bao giờ bật; vô hiệu khi `approval: true` |
| skip_if | Condition \| null | |
| warnings | danh sách chuỗi | `coordinates_only` (FR-008), `text_not_recorded` (không xác định được ô có phải mật khẩu, research R-4), `unverifiable` (`expect` rỗng — FR-020: không sửa chữa bằng model, không tự vá), `clipboard_external` (bước dán không có bước sao chép/cắt trước đó — FR-002), `suggest_approval` (gợi ý bật `approval` — FR-021); tính lại mỗi lần lưu skill, chỉ để hiển thị |

**Selector** — đúng một trong các dạng sau, kèm siêu dữ liệu tự vá:

| Dạng | Trường |
|---|---|
| theo mã | `automation_id` (+ tuỳ chọn `role`) |
| theo vai trò + tên | `role`, `name` |
| theo chữ nhận dạng | `ocr_text` |
| theo vị trí | `xy_in_window: [fx, fy]`, 0 ≤ fx, fy ≤ 1; tuỳ chọn `at: {role, name?}` — vai trò/tên phần tử lúc ghi; có `at` thì chỉ khớp khi phần tử tại điểm có cùng `role` và (nếu có) cùng `name` sau chuẩn hoá bỏ dấu, không phân biệt hoa thường; không có `at` (lúc ghi không có thông tin phần tử) thì khớp theo vị trí |

Siêu dữ liệu tự vá (FR-027): selector do tự vá thêm có `patch: {id, run, pending_review}` — `id` dạng
`p-<run_id>-<step>` duy nhất trong skill, không đổi khi danh sách selector đổi. Danh sách "đã tự vá" = selector có
`patch.pending_review: true`. "Giữ" (theo `id`) → `pending_review: false`; "Hoàn tác" (theo `id`) → xoá selector
đó; `id` không còn → `not_found`. Selector có `patch` nằm ngoài dấu vân tay thực thi nên không đổi `status`.

**Thứ tự thử selector lúc chạy** (FR-009, FR-027): selector gốc không phải `xy_in_window` theo thứ tự trong file →
selector có `patch` (cũ trước) → `xy_in_window`. Selector có `patch` chỉ được thử khi bước có thể kết thúc ở
`verified` (lần kiểm `expect` thứ nhất — sau nấc 0, trước nấc 1 — có ít nhất một điều kiện sai hoặc chưa biết; điều kiện cần phần tử đích khi chưa xác định đích là "chưa biết"); ngược lại bỏ qua.

**Kết quả bước** (FR-020, ghi trong `run.jsonl`): `verified` | `executed_unverified` | `confirmed_by_user` |
`skipped` (`skip_if` thoả). **Lần gửi** (FR-028, FR-049): ngay trước khi gửi, kiểm cửa sổ focus và phần tử
sẽ trúng (toạ độ → phần tử tại điểm là đích hoặc con của đích; gõ phím → phần tử focus là đích); không khớp →
`dispatch: not_sent` (kèm lý do `focus_mismatch` | `occluded` | `wrong_focus`; với `Invoke`/`SetValue`: cửa sổ đích đang có hộp thoại modal của chính nó → `occluded`), ngược lại `sent`. Được "thử lại" khi
`not_sent`, hoặc `action ∈ {scroll, wait}`, hoặc `type` qua `set_value`, hoặc `repeat_safe: true` (không có
`approval`).

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
| skill_fingerprint | chuỗi | `exec_fingerprint()` lúc bắt đầu; `confirm_trial` chỉ nhận khi bằng giá trị hiện tại |
| inputs | map tên → giá trị | giá trị tham số; bí mật không bao giờ có mặt |
| status | enum | xem dưới |
| budget | {max_minutes, max_model_calls, max_repairs_per_step} | chụp từ cài đặt lúc bắt đầu |
| model_calls | số | ≤ max_model_calls |

**Trạng thái lần chạy**:

```text
running ⇄ paused(reason)        reason ∈ {user_input, approval, stuck, missing_secret}
running → succeeded | failed(reason) | cancelled(reason)        reason ∈ {user, emergency_stop}
(dừng khẩn cấp: running → stopping → cancelled(emergency_stop); `stopping` kéo dài tới khi lời gọi đang dở
 (model ≤ 30 s, ứng dụng đích ≤ 5 s + 2 s + 5 s) trả về; kết quả của nó bị bỏ; sự kiện `cancelled` phát ≤ 2 s,
 không chờ `stopping` kết thúc)
(ứng dụng mở lại thấy run chưa kết thúc) → interrupted
```

`failed.reason` ∈ {`time_budget`, `model_budget`, `approval_timeout`, `stuck_timeout`,
`rejected`, `launch_failed`, `no_responder`, `disk_full`, `input_guard_lost`} (`no_responder`: yêu cầu người dùng không thể được trả lời — FR-050). Lần chạy `trial` thành công mở hộp "Xác nhận skill này chạy đúng?"; hộp hiển thị số bước theo từng kết quả (`verified` / `executed_unverified` / `confirmed_by_user`) để người dùng biết bao nhiêu bước không được máy kiểm chứng.

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
 │   (đang có lần chạy: due/waiting_unlock → queued, CHƯA hỏi;
 │    lần chạy hiện tại kết thúc → queued → asking)
 └──────────────────────────────────────────► asking
                                                │
       ┌───────────── Chạy ngay ────────────────┼── Hoãn N phút ──► postponed ──(hết N phút)──► asking
       ▼                                        ├── Bỏ qua ──► skipped
   running → succeeded | failed                 ├── không trả lời, người dùng đang dùng máy
                                                │      → hỏi lại khi máy rảnh (vẫn asking)
                                                └── hết wait_window → missed
```

Bất biến: không có đường nào từ `due` tới `running` mà không qua lựa chọn "Chạy ngay" của người dùng
(FR-041). **Một chủ sở hữu vòng lịch** mỗi phiên: tiến trình giữ named mutex `Local\teach-scheduler` mới chạy tick
và phát `schedule_ask`; tiến trình khác (GUI 002 hoặc `teach schedule watch` đến sau) báo "vòng lịch đang chạy ở
tiến trình khác" và không tick. Mỗi `(schedule, due)` có tối đa một prompt mở và tối đa một `run_started`, kiểm tra
bằng `schedule_history.jsonl` dưới cùng mutex (kể cả sau khi khởi động lại: đã có `run_started`/`skipped`/`missed`
cho `(schedule, due)` → không hỏi lại). `run_now` cho prompt đã đóng (hết hạn, bị thay thế) → `invalid`, không chạy. Khi đang có một lần chạy, lần kích hoạt mới ở `queued` và CHƯA hỏi; chỉ chuyển `asking` sau khi lần
chạy hiện tại kết thúc. Lần kích hoạt mới của cùng lịch đến khi lần cũ chưa xong: nếu lần cũ ở
`asking`/`queued`/`postponed`/`waiting_unlock` → lần cũ được ghi `missed` và thay bằng lần mới (không chồng hộp
hỏi); nếu lần cũ đang `running` → lần mới `queued`.

Lựa chọn Chạy ngay gắn với đúng một Occurrence. Trước khi `asking → running`, kiểm lại (FR-041): skill `ready`,
không khoá, không có run/ghi khác, chưa quá `wait_window`; không đạt → `skipped` (skill không sẵn sàng),
`waiting_unlock`, `queued` hoặc `missed`, kèm lý do trong lịch sử. Máy ngủ lúc đến giờ = máy khoá, không đánh thức
máy. Đồng hồ bị chỉnh/đổi múi giờ: tính lại `next_due` từ giờ hiện tại; mỗi lịch chỉ xét lần đến hạn gần nhất đã
qua; không tạo hai Occurrence cho cùng `(schedule, due)` (FR-043).

### Settings (cài đặt)

| Trường | Mặc định |
|---|---|
| model.base_url, model.name | rỗng (chưa cấu hình → không gọi model) |
| model.api_key | **không có trong file**; Credential Manager service `teach`, user `model-api-key` |
| model.timeout_seconds | 30 |
| model.data_notice_accepted | false (FR-037) |
| language | `vi` hoặc `en`; mặc định theo ngôn ngữ Windows, không khớp thì `vi` |
| hotkeys.emergency_stop / record_pause / record_stop | `Ctrl+Alt+Esc` / `Ctrl+Alt+P` / `Ctrl+Alt+S` |
| budget.max_minutes / max_model_calls / max_repairs_per_step / max_step_minutes | 30 / 20 / 2 / 3 — phút chỉ đếm thời gian máy tự làm, không tính tạm dừng chờ người (FR-023) |
| budget.manual_prompt_timeout_minutes | 60 — hạn chót yêu cầu người dùng ở lần chạy TAY (FR-024); hết hạn → `failed(approval_timeout \| stuck_timeout)` theo FR-050 |
| schedule.idle_minutes / ask_timeout_minutes / approval_timeout_minutes | 3 / 5 / 30 (`approval_timeout_minutes` là hạn chót cho lần chạy theo lịch, FR-044) |
| keep_runs_per_skill | 50 |
| start_with_windows | false (lõi chỉ lưu giá trị; việc đăng ký khởi động cùng Windows thuộc spec 002) |

### Offline eval case (câu hỏi mẫu)

| Trường | Kiểu |
|---|---|
| id | chuỗi |
| kind | `pick_element` \| `diagnose` \| `label_step` |
| (nguồn) | bộ mặc định nhúng trong binary hoặc file người dùng thêm — chỉ bộ mặc định được dùng cho phát hành |
| input | đúng nội dung phần "user" sẽ gửi model (contracts/model-protocol.md) |
| expected | câu trả lời đúng, ví dụ `{"choice": 3}`; với `label_step` là quy tắc chấm `{"language": "vi", "must_contain": ["File name:"], "must_not_contain": ["BaoCao-2026"]}` (luôn kèm: phân tích được, một dòng, 1–120 ký tự) |

Bộ mặc định nhúng trong binary; người dùng/nhóm phát triển có thể thêm file `eval\*.jsonl` trong thư mục
dữ liệu. Kết quả: tỉ lệ đúng theo từng `kind` và tổng (SC-004 ≥ 90%).

**Kết quả có dấu vân tay** (`eval\results\<model>-<thời điểm>.json` trong thư mục dữ liệu — tên file theo model nên
chạy hai model không ghi đè nhau; bản dùng cho phát hành sao vào `crates/teach-core/assets/eval/last_result.json`):
`{fingerprint, model_name, prompt_version, release_set: bool, scores: {pick_element, diagnose, label_step, total},
cases, at}`. `release_set = true` chỉ khi lần chạy dùng đúng bộ mặc định nhúng trong binary, không có `--cases` và
không nạp `eval\*.jsonl` của người dùng. `fingerprint` = FNV-1a 64-bit (tự viết, không thêm crate) trên nội dung
(chuẩn hoá xuống dòng LF) của: `assets/prompts/*`, `src/model/prompt.rs` (dựng câu hỏi, cắt ứng viên cho đủ ≤
2.000, `PROMPT_VERSION`), `src/model/parse.rs`, `src/model/redact.rs` (bộ lọc dữ liệu gửi model, FR-033),
`src/runner/candidates.rs`, `assets/eval/default.jsonl` (bộ câu hỏi mẫu) và `src/eval.rs` (trình chấm) — prompt,
bộ phân tích, bộ lọc ứng viên (Constitution VI) cộng bộ lọc dữ liệu, bộ câu hỏi và trình chấm; sửa phần khác (HTTP,
timeout, so khớp selector nấc 1) không đổi dấu vân tay. Kết quả chỉ chứa tên model, điểm và id ca — không chứa câu
hỏi, câu trả lời hay địa chỉ máy chủ.
