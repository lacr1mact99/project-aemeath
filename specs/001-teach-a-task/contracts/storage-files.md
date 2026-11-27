# Contract: file cài đặt, lịch, log lần chạy, câu hỏi mẫu, dòng lệnh

## `settings.yaml`

```yaml
version: 1
language: vi
model: {base_url: "https://llm.congty.local/v1", name: "deepseek-chat", timeout_seconds: 30,
        data_notice_accepted: true}
hotkeys: {emergency_stop: "Ctrl+Alt+Esc", record_pause: "Ctrl+Alt+P", record_stop: "Ctrl+Alt+S"}
budget: {max_minutes: 30, max_model_calls: 20, max_repairs_per_step: 2, max_step_minutes: 3,
         manual_prompt_timeout_minutes: 60}
schedule: {idle_minutes: 3, ask_timeout_minutes: 5, approval_timeout_minutes: 30}
keep_runs_per_skill: 50
start_with_windows: false
```

Không có trường API key. Khi tải, nếu phát hiện chuỗi giống khoá trong file (trường lạ) → từ chối tải và
báo lỗi, không ghi chuỗi đó vào log.

## `schedules.yaml`

```yaml
version: 1
schedules:
  - id: s-7f3a
    skill: xuat-bao-cao-tuan
    inputs: {tuan: "2026-W48"}
    when: "0 8 * * MON"        # cron 5 trường, giờ địa phương
    wait_window_minutes: 120
    enabled: true
```

## `schedule_history.jsonl`

```json
{"schedule":"s-7f3a","due":"2026-11-30T08:00:00+07:00","event":"waiting_unlock","at":"..."}
{"schedule":"s-7f3a","due":"2026-11-30T08:00:00+07:00","event":"asking","at":"..."}
{"schedule":"s-7f3a","due":"2026-11-30T08:00:00+07:00","event":"postponed","minutes":15,"at":"..."}
{"schedule":"s-7f3a","due":"2026-11-30T08:00:00+07:00","event":"run_started","run":"20261130-081503","at":"..."}
{"schedule":"s-7f3a","due":"2026-11-30T08:00:00+07:00","event":"run_finished","status":"succeeded","at":"..."}
```

`event` ∈ `asking` · `waiting_unlock` · `postponed` · `skipped` · `missed` · `queued` · `run_started`
· `run_finished`. `skipped` có thêm `reason` (`user` | `skill_not_ready`, FR-041). Lịch sử lịch không bị dọn cùng log lần chạy.

## `runs\<skill>\<run_id>\run.jsonl`

Dòng đầu là header, sau đó mỗi sự kiện một dòng, dòng cuối là kết quả:

```json
{"type":"header","run":"20261122-093012","skill":"xuat-bao-cao-tuan","trigger":"trial",
 "inputs":{"tuan":"2026-W47"},"budget":{"max_minutes":30,"max_model_calls":20,"max_repairs_per_step":2,"max_step_minutes":3},
 "app_version":"0.1.0","prompt_version":1}
{"type":"step","step":3,"rung":1,"selector_index":0,"result":"ok","ms":420,"shot":"s03.png"}
{"type":"model_call","step":4,"kind":"pick_element","request":"<phần user đã gửi>","response":"<content>",
 "parsed":{"choice":2},"valid":true,"ms":1830}
{"type":"self_patch","step":4,"selector":{"role":"Edit","name":"Tên tệp:"}}
{"type":"pause","step":5,"reason":"approval"} 
{"type":"resume","step":5,"by":"user","decision":"approve"}
{"type":"result","status":"succeeded","model_calls":1,"ms":15400}
```

`rung` ∈ 0…5 theo FR-018. Bước người dùng làm tay rồi chọn Tiếp tục ở prompt `stuck` mà `expect` rỗng ghi `"result":"confirmed_by_user"` (FR-024). `model_call.request` là đúng chuỗi đã gửi (đã qua bộ lọc dữ liệu) để tái hiện
được; `request` và `response` không bao giờ chứa bí mật (SC-009).

## Log lời gọi model ngoài lần chạy

- `label_step` (lúc biên dịch): ghi vào `skills\<name>\label_log.jsonl`, mỗi lời gọi một dòng `model_call` cùng định dạng như trong `run.jsonl`.
- `eval`, `probe-model`: chỉ ghi vào file kết quả (`eval\results\`), không ghi log nội dung.
- `test_connection`: không ghi log nội dung.
- Các lời gọi này không tính vào `model_calls` của lần chạy nào.

## Câu hỏi mẫu `eval\*.jsonl`

```json
{"id":"pick-001","kind":"pick_element","input":"STEP: ...\nCANDIDATES:\n1. ...","expected":{"choice":3}}
{"id":"diag-004","kind":"diagnose","input":"STEP: ...","expected":{"action":"dismiss","choice":4}}
{"id":"label-002","kind":"label_step","input":"THIS: type 19 characters into Edit \"File name:\" in \"Save As\"\nLANGUAGE: vi","expected":{"language":"vi","must_contain":["File name:"],"must_not_contain":[]}}
```

Chấm: `pick_element`/`diagnose` — câu trả lời sau bước phân tích bằng `expected` (so sánh JSON) → đúng;
`label_step` — đạt mọi quy tắc trong `expected` cộng các quy tắc luôn áp dụng (data-model §Offline eval case).
`teach eval` ghi kết quả có dấu vân tay (data-model) vào `eval\results\`; `--publish` sao kết quả ≥ 90% vào
`crates/teach-core/assets/eval/last_result.json`.

## Dòng lệnh của `teach`

CLI của lõi, dùng [bề mặt điều khiển](./control-surface.md); không mở giao diện (giao diện và các cờ khởi
động như `--tray` thuộc spec 002).

| Lệnh | Việc | Mã thoát |
|---|---|---|
| `teach record "<mô tả>"` | ghi tới khi nhấn phím tắt Dừng (hoặc 10 phút), in cảnh báo từng bước, biên dịch thành skill nháp, rồi hỏi có xoá bản ghi gốc không (stdin đóng → giữ; FR-054) | 0 có skill, 3 không có bước |
| `teach run <skill> [--input k=v]... [--trial] [--no-model] [--corrupt-primary] [--inject-dialog-at <step>]` | chạy skill; yêu cầu người dùng hỏi ở console; `--trial` hỏi xác nhận sẵn sàng khi xong; các cờ còn lại tạo 3 chế độ của bộ đo | 0 thành công, 1 thất bại, 2 huỷ |
| `teach skill list\|show <skill>\|patches <skill> [--keep\|--undo <step>:<idx>]` | xem skill, duyệt tự vá | 0 |
| `teach config set-model --url <u> --name <n>` · `teach config set-key` (đọc từ stdin, không echo) · `teach config accept-notice` · `teach secret set <key>` | cài đặt, bí mật | 0 |
| `teach eval [--cases <file>] [--publish]` | chạy bộ câu hỏi mẫu với model trong cài đặt, in tỉ lệ đúng theo loại | 0 nếu ≥ 90% |
| `teach bench <thư mục skill chuẩn> --mode 1\|2\|3` | chạy cả bộ 10 skill chuẩn ở một chế độ, in bảng kết quả | 0 nếu đạt ngưỡng |
| `teach probe-model` | phép đo M0: giao thức, `json_schema`, phần suy nghĩ, độ trễ, 20 lần JSON một dòng | 0 nếu ≥ 19/20 |
| `teach schedule list\|add\|rm\|history` · `teach schedule watch` | quản lý lịch; `watch` chạy vòng lịch ở console, hỏi ở console khi đến giờ | 0 |

Mọi lệnh chạy vẫn tuân thủ dừng khẩn cấp, phê duyệt và ngân sách.
