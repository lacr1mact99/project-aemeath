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
trên máy phát triển không có giá trị). Với mục Dev, nên đo lại trên máy công ty trước khi chốt M0 vì ảnh
Windows của công ty có thể khác.

| # | Máy |
|---|---|
| 0.1, 0.4, 0.5, 0.7, 0.8 | Dev (đo lại trên Cty) |
| 0.2, 0.3 | Dev + Cty (bộ gõ, trình duyệt, app nội bộ của công ty) |
| 0.6 | **Cty** (gói OCR phụ thuộc ảnh Windows/chính sách của công ty) |
| 0.10 | **Cty** (mạng, proxy, CA, máy chủ model nội bộ) |

| # | Kiểm tra | Đạt khi |
|---|---|---|
| 0.1 | Ghi 10 cú bấm trên Notepad và Edge | 10/10 sự kiện có `element` với `role` + (`name` hoặc `automation_id`) |
| 0.2 | Gõ "Tiếng Việt có dấu: Ươ ạ ẫ ỹ" bằng bộ gõ Việt khi đang ghi | `text` trong events.jsonl khớp nội dung ô |
| 0.3 | Gõ vào ô mật khẩu (trang đăng nhập trình duyệt, hộp thoại Win32, app Electron) | không ký tự nào trong events.jsonl; ảnh bị che; ghi lại app nào `is_password` không xác định |
| 0.4 | SendInput Unicode "Ươ ạ ẫ ỹ" vào Notepad, Word, Electron, trình duyệt (bộ gõ Việt bật và tắt) | văn bản trong ô khớp chính xác |
| 0.5 | Chạm chuột trong lúc app đang SendInput | được nhận là "người dùng" (không có `TEACH_MAGIC`), kể cả khi bộ gõ Việt bật |
| 0.6 | `Get-WindowsCapability -Online \| ? Name -like 'Language.OCR*'`; OCR 10 nút nhãn tiếng Việt | selector `ocr_text` (so khớp bỏ dấu) tìm đúng ≥ 9/10 nút (R-5) |
| 0.7 | Chụp cửa sổ trên màn hình 150% và hai màn hình khác DPI | ảnh đúng khung cửa sổ; `xy_in_window` bấm đúng điểm |
| 0.8 | Phím tắt dừng khẩn cấp và tạm dừng/dừng ghi qua hook khi cửa sổ khác đang focus (kể cả ứng dụng toàn màn hình) | nhận được 20/20 lần; tổ hợp không lọt vào ứng dụng đích |
| 0.10 | `cargo run --example spike_probe` (T012) với DeepSeek, rồi với GLM — từ M3 dùng `teach probe-model` (T068) | in ra: tương thích OpenAI?, `json_schema`?, phần suy nghĩ ở đâu, p50/p95 độ trễ, giới hạn ngữ cảnh; **≥ 19/20** câu JSON một dòng hợp lệ (mã thoát 0) |

## M1

1. `teach record "Lưu ghi chú hôm nay"`.
2. Mở Notepad, gõ `Xin chào`, `Ctrl+S`, gõ tên `ghichu-test.txt`, bấm **Save**. Nhấn phím tắt Dừng (`Ctrl+Alt+S`).
3. Kỳ vọng: `skills\luu-ghi-chu-hom-nay\skill.yaml` hợp lệ theo [skill-format](./contracts/skill-format.md),
   ~5 bước, mỗi bước có `intent`, ≥ 1 selector, và ≥ 1 `expect` hoặc cảnh báo `unverifiable`; trên bộ workflow
   chuẩn ≥ 80% số bước có `expect` tự sinh; trạng thái `draft` (SC-001).
4. Ghi lại một lần có gõ mật khẩu vào trang đăng nhập bất kỳ → `events.jsonl` không có ký tự đó; ảnh bị che.
5. Tắt model (để trống base URL) và lặp bước 1–3 → câu ý định dạng tất định (`Bấm "Save" (Button)`).
6. Để ghi chạy quá 10 phút → tự dừng, skill vẫn được sinh.

## M2

```powershell
teach bench bench\skills --mode 1      # model tắt, giao diện không đổi
```

- Kỳ vọng: 10/10 skill `succeeded`, tổng `model_calls` = 0, mã thoát 0 (SC-002).
- Dừng khẩn cấp: chạy một skill dài, nhấn `Ctrl+Alt+Esc` → thao tác cuối cùng trong log cách lúc nhấn
  ≤ 1 s và `cancelled(emergency_stop)` ≤ 2 s; lặp 10 lần, 10/10; thêm 3 lần nhấn khi ứng dụng đích bị treo
  (SC-003). Phần "nhấn khi đang chờ model" đo ở §M3.
- Chạm chuột giữa lần chạy (`teach run`) → trạng thái `paused(user_input)`, console hỏi; chọn `continue` → chạy tiếp.
- Bước `approval: true` → dừng tới khi chọn `approve`; `reject` → `failed(rejected)`.
- Sửa `expect` của một bước thành điều không thể đúng → sau 2 lần sửa, `paused(stuck)` hiện ảnh và lý do.

## M3

```powershell
teach eval --publish                   # với model trong cài đặt; ghi last_result.json khi ≥ 90%
teach bench bench\skills --mode 2      # selector chính bị làm hỏng → pick_element
teach bench bench\skills --mode 3      # hộp thoại chèn giữa chừng → diagnose
```

Chạy cả ba lệnh với DeepSeek rồi với GLM; chọn model mặc định theo kết quả. Model được chọn phải đạt:
`eval` ≥ 90%; mode 2 ≥ 8/10; mode 3 ≥ 8/10; trong log không có bước nào thao tác sai phần tử mà `expect`
vẫn đạt (SC-004, SC-005). Sau mode 2: các skill có selector `pending_review: true`; `teach skill patches <skill>` liệt kê,
`--undo` xoá đúng selector đó, `status` không đổi.

Kiểm tra riêng tư (SC-009): đặt một bí mật `{{secret:demo}}` = `S3cret!xyz` và tham số `tuan` =
`KHACH-HANG-ABC`; chạy mode 2 cho skill đó; tìm trong toàn bộ thư mục dữ liệu và trong `model_call.request`:

```powershell
Select-String -Path "$env:LOCALAPPDATA\teach\*" -Pattern 'S3cret!xyz' -Recurse   # 0 kết quả
# request gửi model không chứa 'KHACH-HANG-ABC' hay văn bản đã gõ
```

Dừng khẩn cấp khi đang chờ model (máy chủ giả trả chậm 30 s): 3/3 lần không có thao tác sau 1 s và
`cancelled(emergency_stop)` ≤ 2 s; `start_run` ngay sau đó trả `busy` tới khi lời gọi cũ trả về (SC-003).

Ngắt mạng → skill chế độ 1 vẫn chạy xong; skill chế độ 2 chuyển `paused(stuck)` với lý do "không kết nối
được máy chủ model" (SC-010).

## M4

→ Spec 002 (giao diện). Điều kiện từ lõi: vòng dạy → chạy thử (`teach run <skill> --trial`) → phê duyệt →
xác nhận sẵn sàng làm trọn bằng CLI, không sửa file bằng tay ngoài bước đánh dấu tham số (SC-007 phần lõi).

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

Trên mọi kịch bản: 0 `run_started` khi chưa có lựa chọn `run_now` cho lần đó (SC-008).
