# Contract: bề mặt điều khiển (lõi ⇄ giao diện 002 / CLI)

FR-047. Giao diện Tauri (spec 002) và CLI `teach` chỉ nói chuyện với lõi qua bề mặt này: **lệnh** (gọi đồng
bộ, trả kết quả hoặc lỗi) và **sự kiện** (lõi phát, bên hiển thị theo dõi). Đây là hợp đồng ngữ nghĩa; dạng
Rust cụ thể (`Engine` + `Receiver<Event>`) và cách ánh xạ sang lệnh/sự kiện Tauri chốt ở plan 002.

## Bất biến

1. Không có lệnh nào cho phép: tự phê duyệt, chạy lịch không qua `answer_prompt(.., run_now)`, đọc API key,
   đọc giá trị bí mật, đọc nội dung đã gõ của ô mật khẩu, gọi model tuỳ ý.
2. Lệnh có tác dụng phụ trả lỗi có mã (`invalid`, `busy`, `not_found`, `not_ready`, `model_*`, `io`) kèm
   thông điệp theo ngôn ngữ cài đặt; không panic xuyên ranh giới.
3. Mọi thay đổi trạng thái chạy/ghi/lịch đều phát sự kiện ≤ 100 ms sau khi xảy ra.
4. Dừng khẩn cấp không đi qua bề mặt này (hook trong lõi); `stop_run` chỉ là thêm một đường.

## Lệnh

| Nhóm | Lệnh | Ghi chú |
|---|---|---|
| Ghi | `start_recording(goal)` · `pause_recording()` · `resume_recording()` · `stop_recording() -> SkillName \| NoSteps` | `busy` nếu đang chạy skill (FR-025) |
| Skill | `list_skills() -> [SkillSummary]` · `get_skill(name) -> Skill` · `save_skill(name, Skill) -> Ok \| [FieldError]` · `delete_skill(name)` · `step_image(name, step, before\|after) -> path` | `save_skill` kiểm tra hợp lệ (FR-014), tự đổi `ready → draft` nếu bước đổi (FR-017) và trả `status_changed` |
| Tự vá | `pending_patches(name) -> [Patch]` · `keep_patch(name, step, idx)` · `undo_patch(name, step, idx)` | không đổi `status` (FR-027) |
| Chạy | `start_run(name, inputs, trigger) -> RunId` (trả `busy` khi còn lần chạy ở `running`/`paused`/`stopping` — gồm lần chạy vừa huỷ mà lời gọi đang dở chưa trả về, tối đa ~30 s) · `stop_run()` · `answer_prompt(prompt_id, choice)` · `confirm_trial(run_id, ok)` · `list_runs(name) -> [RunSummary]` · `get_run_log(name, run_id) -> [RunLogLine]` | `start_run` thiếu input → `invalid` kèm danh sách thiếu |
| Bí mật | `set_secret(key, value)` · `delete_secret(key)` · `secret_exists(key) -> bool` | chỉ ghi; giá trị không bao giờ đi ra |
| Cài đặt | `get_settings() -> Settings` (không có key) · `save_settings(Settings) -> Ok \| [FieldError]` · `set_api_key(value)` · `api_key_set() -> bool` · `data_notice() -> Text` · `accept_data_notice()` | lõi từ chối gọi model khi notice chưa chấp nhận (FR-037) |
| Kiểm tra | `test_connection() -> Ok \| ModelError` · `run_eval() -> EvalReport` | chạy trên luồng worker; tiến độ qua sự kiện |
| Lịch | `list_schedules() -> [ScheduleSummary]` (kèm `next_due`, lý do tạm ngưng) · `save_schedule(Schedule) -> Ok \| [FieldError]` · `delete_schedule(id)` · `set_schedules_paused(bool)` · `schedule_history(id?) -> [HistoryLine]` · `preview_cron(expr, n) -> [DateTime]` | `save_schedule` từ chối skill `draft` (`not_ready`) |

## Sự kiện

| Sự kiện | Nội dung |
|---|---|
| `recording_state` | `recording \| paused \| stopped`, thời gian đã ghi |
| `recording_warning` | `coordinates_only` / `text_not_recorded` cho bước vừa ghi (FR-008) |
| `run_state` | run_id, skill, trạng thái (data-model §Run, gồm `stopping`), bước hiện tại, nấc, model_calls/ngân sách, thời gian máy tự làm/ngân sách; khi kết thúc kèm lý do: `failed(time_budget \| model_budget \| approval_timeout \| stuck_timeout \| rejected \| launch_failed \| no_responder \| disk_full \| input_guard_lost)`, `cancelled(user \| emergency_stop)` |
| `run_log` | một dòng log vừa ghi (đã lọc bí mật) |
| `prompt` | **yêu cầu người dùng** (dưới) |
| `prompt_closed` | prompt_id, lý do (`answered`, `expired`, `superseded`) |
| `run_finished` | run_id, kết quả, số tự vá chờ duyệt, `trial` (để hỏi xác nhận) |
| `eval_progress` / `eval_done` | tiến độ, báo cáo |
| `schedule_changed` | lịch thêm/sửa/xoá/tạm ngưng, `next_due` mới |

### Yêu cầu người dùng (`prompt`)

```text
{ id, kind, context, choices, deadline? }
kind ∈ approval        choices: approve | reject
       stuck           choices: continue | cancel          context: lý do (`repair_budget`, `step_timeout` — bước vượt
                                                       3 phút, `target_unresponsive`, `focus_mismatch_secret`,
                                                       lỗi model `auth_error`/`unavailable`), ảnh lúc ghi, ảnh hiện tại
       user_input      choices: continue | stop
       missing_secret  choices: continue | cancel          context: tên khoá (gọi set_secret trước khi continue)
       schedule_ask    choices: run_now | postpone(minutes) | skip     context: lịch, skill, inputs
```

- Tại một thời điểm có tối đa một `prompt` của lần chạy đang chạy; `schedule_ask` không phát khi đang có lần
  chạy (lõi xếp hàng — data-model §Occurrence).
- `answer_prompt` với `choice` không thuộc `choices` → `invalid`, lõi không đổi trạng thái.
- Hết `deadline` → lõi tự đóng prompt (`prompt_closed: expired`) và áp quy tắc của spec (thất bại / hỏi lại
  khi rảnh / bỏ lỡ) — không bao giờ coi hết hạn là đồng ý.

## CLI `teach`

CLI dùng đúng các lệnh trên; prompt hiện ở console (đọc lựa chọn từ stdin). Danh sách lệnh con:
[storage-files.md §Dòng lệnh](./storage-files.md#dòng-lệnh-của-teach).
