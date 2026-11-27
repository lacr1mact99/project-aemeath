# Quickstart: chạy và nghiệm thu Teach a Task (lõi)

Phần giao diện (M4, khay/hộp hỏi/thông báo của M5) nghiệm thu ở quickstart của spec 002.

Hướng dẫn kiểm chứng end-to-end theo từng mốc. Định dạng file: [contracts/](./contracts/); thực thể:
[data-model.md](./data-model.md); lý do kỹ thuật: [research.md](./research.md).

## Chuẩn bị

- Windows 10 22H2+ hoặc 11 x64, phiên đang mở khoá, tài khoản người dùng thường (không Administrator).
- Rust stable ≥ 1.89 (`rustup default stable`), Visual Studio Build Tools (MSVC).
- Các bước có model thật (M0 0.10, M3) chạy trên **máy công ty, trong mạng công ty**, với ba tham số
  base URL, API key, tên model cho **cả** DeepSeek và GLM. Trên máy phát triển (ngoài mạng công ty), phần
  model chỉ được kiểm thử bằng máy chủ giả cục bộ trong `cargo test`.
- Ứng dụng thử: Notepad, File Explorer, Calculator, Settings, Edge/Chrome, một ứng dụng Office (hoặc
  LibreOffice), một ứng dụng Electron bất kỳ.

```powershell
cargo build --release
cargo test                      # logic thuần, không cần giao diện thật
.\target\release\teach.exe --help   # CLI của lõi (contracts/storage-files.md §Dòng lệnh)
```

## M0

Spike kỹ thuật. Mỗi mục ghi kết quả (đạt/không, số đo, quyết định dự phòng) vào `research.md` §Kết quả M0.

**Nơi đo**: máy phát triển không thuộc mạng công ty. Cột "Máy" cho biết mục nào được phép đo ở đâu:
**Dev** = đo trên máy phát triển được; **Cty** = bắt buộc đo trên máy công ty, trong mạng công ty (kết quả
trên máy phát triển không có giá trị). M0 có **hai cổng**:

- **Cổng A (M0-Dev)** — chặn nền tảng Windows (T021–T025, T027) và runner phát thao tác (T040, T083–T085): các dòng
  0.1, 0.4, 0.5, 0.7, 0.8, 0.9, 0.13, 0.14 và phần máy dev của 0.2, 0.3, 0.11; plan D-1, D-2 và `RECORD_LAG_LIMIT_MS`
  đã chốt.
- **Cổng B (M0-Cty)** — chặn phần dựa vào OCR (T026, nhánh OCR của T033/T039; trước cổng này `ocr_text` coi như
  không khớp, recorder rơi về `xy_in_window` như FR-004 khi OCR quá hạn) và việc bắt đầu US4/M3 (T053 trở đi):
  0.6a, 0.6b (plan D-3), 0.10.
- **Đo lại trên máy công ty** các dòng Dev và phần Cty của 0.2, 0.3, 0.11: không chặn task nào, nhưng là điều kiện
  bắt buộc của checkpoint M2 (ảnh Windows công ty có thể khác; lệch → sửa `recorder`/`input` trước khi M2 đạt).
- 0.12 (so prompt Anh–Việt) cần prompt và bộ câu hỏi mẫu nên đo trong M3 (§M3, T080), không thuộc cổng nào.

| # | Máy | Cổng |
|---|---|---|
| 0.1, 0.4, 0.5, 0.7, 0.8, 0.9, 0.13, 0.14 | Dev (đo lại trên Cty trước checkpoint M2) | A |
| 0.2, 0.3, 0.11 | Dev (cổng A) + Cty (bộ gõ, trình duyệt, app nội bộ, chính sách khoá máy/RDP — trước checkpoint M2) | A |
| 0.6a, 0.6b | **Cty** (gói OCR và kiểu phân phối phụ thuộc ảnh Windows/chính sách của công ty) | B |
| 0.10 | **Cty** (mạng, proxy, CA, máy chủ model nội bộ) | B |

| # | Kiểm tra | Đạt khi |
|---|---|---|
| 0.1 | Ghi 10 cú bấm trên Notepad và Edge | 10/10 sự kiện có `element` với `role` + (`name` hoặc `automation_id`) |
| 0.2 | Gõ "Tiếng Việt có dấu: Ươ ạ ẫ ỹ" bằng bộ gõ Việt khi đang ghi | `text` trong events.jsonl khớp nội dung ô |
| 0.3 | Gõ vào ô mật khẩu (trang đăng nhập trình duyệt, hộp thoại Win32, app Electron) | không ký tự nào trong events.jsonl; ảnh bị che; ghi lại app nào `is_password` không xác định |
| 0.4 | SendInput Unicode "Ươ ạ ẫ ỹ" vào Notepad, Word, Electron, trình duyệt (bộ gõ Việt bật và tắt) | văn bản trong ô khớp chính xác |
| 0.5 | Chạm chuột trong lúc app đang SendInput | được nhận là "người dùng" (không có `TEACH_MAGIC`), kể cả khi bộ gõ Việt bật |
| 0.6a | Gọi `OcrEngine` từ `teach.exe` **unpackaged** và từ kiểu phân phối plan 002 dự kiến (plan D-3) | OCR trả kết quả trong đúng kiểu phân phối; trượt → ghi phương án đã chọn (sparse package / Tesseract / bỏ `ocr_text` khỏi MVP) |
| 0.6b | `Get-WindowsCapability -Online \| ? Name -like 'Language.OCR*'`; OCR 10 nút nhãn tiếng Việt (chỉ đo khi 0.6a đạt hoặc với phương án thay thế) | selector `ocr_text` (so khớp bỏ dấu) tìm đúng ≥ 9/10 nút (R-5) |
| 0.7 | Chụp cửa sổ trên màn hình 150% và hai màn hình khác DPI | ảnh đúng khung cửa sổ; `xy_in_window` bấm đúng điểm |
| 0.8 | Phím tắt dừng khẩn cấp và tạm dừng/dừng ghi qua hook khi cửa sổ khác đang focus (kể cả ứng dụng toàn màn hình) | nhận được 20/20 lần; tổ hợp không lọt vào ứng dụng đích |
| 0.9 | `spike_secret` (T011): ghi/đọc/xoá bí mật và API key qua `keyring`, kiểm mức lưu bằng `CredReadW` | ghi/đọc/xoá đúng; `Persist = CRED_PERSIST_LOCAL_MACHINE`; không đạt → chốt gọi `CredWriteW` trực tiếp |
| 0.10 | `cargo run --example spike_probe` (T012) với DeepSeek, rồi với GLM — từ M3 dùng `teach probe-model` (T068) | in ra: tương thích OpenAI?, `json_schema`?, phần suy nghĩ ở đâu, p50/p95 độ trễ, giới hạn ngữ cảnh; **≥ 19/20** câu JSON một dòng hợp lệ (mã thoát 0) |
| 0.11 | `spike_session` (T090): khoá/mở khoá máy, UAC, ngắt/nối lại RDP, thu nhỏ cửa sổ RDP ở máy khách — mỗi ca 5 lần, cả lúc ghi và lúc chạy | 100% ca khoá/UAC/RDP ngắt được phát hiện và tạm dừng trước thao tác kế tiếp; ca RDP thu nhỏ ghi lại hành vi (giới hạn đã biết) |
| 0.13 | `spike_stop` (T088) trên `bench_target` treo theo lệnh: đọc UIA treo, `Invoke` treo, `SetValue` treo, SendInput liên tục (plan D-1) — mỗi ca 5 lần | 20/20 không lần gửi mới nào qua `Gate` sau ≤ 1 s; ghi có/không tác dụng muộn và độ trễ; chốt dùng `Invoke`/`SetValue` hay SendInput mặc định |
| 0.14 | `spike_hookloss` (T089): ép mất hook khi máy rảnh, input liên tục, đổi focus giữa lúc gõ, 20 lần bấm/giây; đo `lag_ms` của recorder với UIA chậm (plan D-2) | ghi thời gian phát hiện/cài lại; `RegisterHotKey` dừng được 20/20 trong lúc hook mất; chốt `RECORD_LAG_LIMIT_MS` |

## Bảng dừng khẩn cấp

Một bảng duy nhất cho SC-003; plan, tasks và các checkpoint tham chiếu bảng này. Mỗi lần thử đạt khi: không lần gửi
mới nào qua `Gate` sau ≤ 1 s kể từ lúc nhấn **và** `cancelled(emergency_stop)` ≤ 2 s. Mốc đạt: 100% số lần thử.

| Mốc | Tình huống | Số lần | Cách tạo |
|---|---|---|---|
| M0 | 4 ca lời gọi treo của `spike_stop` (0.13) | 20 | `bench_target` treo theo lệnh |
| M2 | skill dài, ứng dụng phản hồi bình thường | 10 | `teach run` + nhấn `Ctrl+Alt+Esc` |
| M2 | ứng dụng đích treo (đọc UIA treo) | 5 | skill trên `bench_target`, lệnh treo |
| M2 | đang `Invoke`/`SetValue` vào ứng dụng treo | 5 | như trên |
| M3 | đang chờ model (máy chủ giả trả chậm 30 s) | 6 | máy chủ giả `127.0.0.1` |

## M1

Trạng thái đầu cố định: Notepad đang mở với tài liệu trống và đang focus; chưa có `ghichu-test.txt`.

1. `teach record "Lưu ghi chú hôm nay"`.
2. Gõ `Xin chào`, `Ctrl+S`, gõ tên `ghichu-test.txt`, bấm **Save**. Nhấn phím tắt Dừng (`Ctrl+Alt+S`).
3. Kỳ vọng: `skills\luu-ghi-chu-hom-nay\skill.yaml` hợp lệ theo [skill-format](./contracts/skill-format.md),
   đúng 4 bước `type`, `hotkey`, `type`, `click` (không có `launch`); mỗi bước có `intent`; bước `type`/`click` có
   ≥ 1 selector; mỗi bước có ≥ 1 `expect` hoặc cảnh báo `unverifiable`; không `expect` nào đúng sẵn ở trạng thái
   trước hành động trong bản ghi; trên bộ workflow chuẩn ≥ 80% số bước có `expect` tự sinh; trạng thái `draft`
   (SC-001).
4. Ghi lại một lần có gõ mật khẩu vào trang đăng nhập bất kỳ → `events.jsonl` không có ký tự đó; ảnh bị che,
   hoặc không có ảnh khi phần tử không có toạ độ.
5. Tắt model (để trống base URL) và lặp bước 1–3 → câu ý định dạng tất định (`Bấm "Save" (Button)`).
6. Để ghi chạy quá 10 phút → tự dừng, skill vẫn được sinh.

## M2

```powershell
teach bench bench\skills --mode 1      # model tắt, giao diện không đổi
```

- Kỳ vọng: 10/10 skill `succeeded` **và** 10/10 `check.ps1` đạt, tổng `model_calls` = 0, mã thoát 0 (SC-002).
- Dừng khẩn cấp: các dòng M2 của §Bảng dừng khẩn cấp (20/20).
- Chạm chuột giữa lần chạy (`teach run`) → trạng thái `paused(user_input)`, console hỏi; chọn `continue` → chạy tiếp.
- Bước `approval: true` → dừng tới khi chọn `approve`; `reject` → `failed(rejected)`.
- Sửa `expect` của một bước thành điều không thể đúng → sau 2 lần sửa, `paused(stuck)` hiện ảnh và lý do.
- Sửa `expect` của một bước thành điều đã đúng trước khi thao tác → bước ghi `executed_unverified`, không có nấc
  2/4; bước `click` có `expect` không đạt → không có lần gửi thứ hai (`dispatch: sent` chỉ một lần); bật
  `teach skill set <skill> <step> --repeat-safe on` rồi chạy lại → được bấm lại trong ngân sách sửa chữa.
- Mở một hộp thoại của chính app che nút đích ngay trước bước bấm → log `dispatch: not_sent` (`occluded`), không
  cú bấm nào lọt vào hộp thoại.

## M3

**Máy công ty.** So prompt Anh–Việt (R-7, trước đây là dòng M0 0.12): chạy `teach eval` với prompt Anh và Việt cho
cả hai model; ghi điểm 4 tổ hợp vào research.md; chọn tiếng Việt chỉ khi cao hơn ≥ 5 điểm %. Làm trước khi chạy
bench chế độ 2/3 để bench dùng đúng prompt đã chọn.

```powershell
teach eval                                    # bộ mặc định, KHÔNG --publish; kết quả eval\results\<model>-*.json
teach bench bench\skills --mode 2             # mọi selector gốc của bước đích bị làm hỏng → bắt buộc nấc 2
teach bench bench\dialog-skills --mode 3      # bench_target bật hộp thoại của chính nó → bắt buộc nấc 4
```

Bench luôn chạy trên **bản sao** skill trong thư mục tạm, chạy `reset.ps1` trước mỗi lần và chấm bằng `check.ps1`
(độc lập với `expect`). Một lần chỉ được tính đạt khi `succeeded`, `check.ps1` đạt **và** log có nấc cần đo (mode 2:
`rung: 2` ở bước bị làm hỏng; mode 3: `rung: 4`). Sau mode 2, bench chạy **lượt 2** trên bản sao đã tự vá: mọi lần
đạt ở lượt 1 phải `succeeded` ở nấc 1 với `selector: patch:<id>`, 0 lần gọi model.

Chạy `eval` + hai lệnh bench với DeepSeek rồi với GLM; chọn model mặc định. Model được chọn phải đạt: `eval` ≥ 90%;
mode 2 ≥ 8/10; mode 3 ≥ 8/10; 0 lần `succeeded` mà `check.ps1` trượt (SC-004, SC-005). Sau đó đặt model đã chọn
trong cài đặt và chạy `teach eval --publish` (chỉ nhận lần chạy `release_set`). Tự vá: `teach skill patches <skill>`
liệt kê theo `patch_id`; thêm một selector gốc rồi `--undo <patch_id>` vẫn xoá đúng selector đó; `status` không đổi.

Kiểm tra riêng tư (SC-009, theo từng nơi dữ liệu đi qua): đặt bí mật `{{secret:demo}}` = `S3cret!xyz` và tham số
`tuan` = `KHACH-HANG-ABC`; chạy mode 2 cho skill đó.

```powershell
$d = "$env:LOCALAPPDATA\teach"
# File văn bản (yaml/jsonl/json/tạm): 0 kết quả cho bí mật và API key
Get-ChildItem $d -Recurse -File -Exclude *.png | Select-String -SimpleMatch 'S3cret!xyz'
Get-ChildItem $d -Recurse -File -Exclude *.png | Select-String -SimpleMatch '<api key>'
# Request gửi model (log): 0 kết quả cho tham số và bí mật
Get-ChildItem "$d\runs" -Recurse -Filter run.jsonl | Select-String -SimpleMatch 'KHACH-HANG-ABC','S3cret!xyz'
# Ảnh: không có ảnh nào của lần chạy từ bước gõ bí mật trở đi (so số dòng `step` có `shot` với số file)
```

Phần thân/header request thật và sự kiện được kiểm tự động trong `tests/privacy.rs` (máy chủ giả ghi lại toàn bộ
request); trên máy công ty, đối chiếu `model_call.request` trong `run.jsonl` như trên.

Dừng khẩn cấp khi đang chờ model: dòng M3 của §Bảng dừng khẩn cấp (6/6); `start_run` ngay sau đó trả `busy` tới
khi lời gọi cũ trả về.

Ngắt mạng → skill chế độ 1 vẫn chạy xong; skill chế độ 2 chuyển `paused(stuck)` với lý do "không kết nối
được máy chủ model" (SC-010).

## M4

→ Spec 002 (giao diện). Điều kiện từ lõi (SC-007 phần lõi), chỉ bằng CLI, **không sửa YAML tay**:

```powershell
teach record "Lưu ghi chú hôm nay"
teach skill param luu-ghi-chu-hom-nay 3 --from 0 --to 15 --name ten_file --example ghichu-test.txt
teach skill set luu-ghi-chu-hom-nay 4 --approval on
teach run luu-ghi-chu-hom-nay --trial --input ten_file=moi.txt   # approve ở console, rồi xác nhận → ready
teach skill show luu-ghi-chu-hom-nay                              # status: ready
teach skill set luu-ghi-chu-hom-nay 4 --approval off              # → draft (dấu vân tay đổi)
teach skill confirm luu-ghi-chu-hom-nay <run_id cũ>               # bị từ chối: not_ready
```

## M5

Nghiệm thu lõi bằng mô phỏng: `cargo test --test scheduler` dùng đồng hồ, trạng thái máy rảnh và khoá máy giả.
Kịch bản thật trên máy với khay, hộp hỏi, thông báo Windows: quickstart của spec 002. Có thể thử tay bằng
`teach schedule watch` (hỏi ở console).

| Kịch bản (mô phỏng) | Kỳ vọng |
|---|---|
| Lịch mỗi 5 phút, máy rảnh | phát `prompt(schedule_ask)`; không có `run_started` khi chưa trả lời `run_now` |
| Đang có input lúc đến giờ, không trả lời 5 phút | `prompt_closed(expired)`, phát lại khi máy rảnh 3 phút |
| Hoãn 10 phút | phát lại sau 10 phút; lịch sử có `postponed` |
| Khoá máy qua giờ hẹn, mở khoá trong 2 giờ | `waiting_unlock` rồi `asking` ngay sau khi mở khoá |
| Khoá máy quá `wait_window_minutes` | lịch sử ghi `missed` |
| Lần chạy theo lịch gặp bước phê duyệt, không trả lời 30 phút | `failed(approval_timeout)` |
| Skill A đang chạy khi lịch của skill B đến giờ | B `queued`, chỉ hỏi sau khi A xong |
| Sửa bước của skill đang có lịch | skill về `draft`, lịch tạm ngưng có lý do; xác nhận lại → lịch hoạt động lại |
| Đổi giờ mùa hè/đông (DST) giữa hai lần | lần kế tiếp đúng theo giờ địa phương, không kích hoạt trùng |
| Máy ngủ qua giờ hẹn, thức trong / quá `wait_window_minutes` | `asking` / `missed`; không đánh thức máy; lần cũ hơn chỉ `missed` |
| Chỉnh đồng hồ lùi 1 giờ rồi tiến 3 giờ | không có hai Occurrence cùng `due`; không chạy bù hàng loạt |
| Trả lời `run_now` khi skill vừa về `draft` | không `run_started`; lịch sử `skipped` (`skill_not_ready`) |
| Đến giờ, máy rảnh, mở khoá | `prompt(schedule_ask)` phát ≤ 30 s sau `due` (đồng hồ giả) |
| Hai tiến trình cùng theo dõi lịch (GUI giả + `schedule watch`) | chỉ tiến trình giữ `Local\teach-scheduler` tick; đúng 1 prompt cho mỗi `(schedule, due)`; tiến trình sau thoát mã 4 |
| `run_now` đến sau khi prompt đã `expired`/`superseded` | `invalid`; 0 `run_started` |
| Khởi động lại sau khi lần kích hoạt đã `run_started` | không hỏi lại `(schedule, due)` đó |
| Lịch mỗi 5 phút, người dùng hoãn 10 phút | lần kế tiếp đến trong lúc hoãn → lần cũ `missed`, chỉ một prompt cho lần mới |

Trên mọi kịch bản (SC-008): 100% lần đủ điều kiện hỏi có `prompt(schedule_ask)`; 100% lần còn lại có kết quả lịch
sử đúng; 0 `run_started` khi chưa có lựa chọn `run_now` cho lần đó; 0 `(schedule, due)` có hai prompt hoặc hai
`run_started`.
