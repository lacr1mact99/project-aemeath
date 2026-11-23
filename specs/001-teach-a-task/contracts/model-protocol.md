# Contract: giao tiếp với model

Toàn bộ nằm sau **một mô-đun duy nhất** (`src/model.rs`, Constitution §Ràng buộc). Áp dụng FR-028…FR-034.

## Giao thức HTTP

```http
POST {base_url}/chat/completions
Authorization: Bearer <api_key từ Credential Manager>
Content-Type: application/json

{"model": "<settings.model.name>", "temperature": 0, "stream": false,
 "messages": [ {"role":"system","content": <prompt cố định>},
               ...ví dụ mẫu dạng cặp user/assistant...,
               {"role":"user","content": <đầu vào câu hỏi>} ]}
```

- Không gửi `tools`, ảnh, hay `response_format` trừ khi M0 xác nhận hỗ trợ `json_schema`; khi đó bật
  thêm qua cài đặt ẩn `model.use_json_schema`, nhưng bước phân tích dưới đây vẫn chạy.
- `max_tokens`: không gửi mặc định (model suy luận cần chỗ cho phần suy nghĩ); M0 quyết có đặt hay không.
- Chỉ đọc `choices[0].message.content`. Bỏ qua `reasoning_content` hoặc trường suy nghĩ riêng.
- Timeout `settings.model.timeout_seconds` (30). HTTP 401/403 → `auth_error`; lỗi mạng/timeout/5xx →
  `unavailable`; cả hai được xử lý như `ask_user` và nguyên nhân nằm trong yêu cầu người dùng + log (FR-032). Không tự thử lại
  khi lỗi mạng (mỗi lần thử lại tốn ngân sách và thời gian của người dùng).

## Phân tích câu trả lời (chung cho cả ba loại)

1. Xoá mọi khối `<think>…</think>` (và `<thinking>…</thinking>`).
2. Tìm đối tượng JSON cân bằng ngoặc đầu tiên (bỏ qua ngoặc trong chuỗi), parse.
3. Kiểm tra theo bảng "giá trị cho phép" của từng loại.
4. Sai ở bước 2 hoặc 3 → gửi lại **một lần** đúng hội thoại cũ + câu trả lời sai (role assistant) + user:
   `Invalid answer: <lý do>. Reply with exactly one line of JSON, for example <ví dụ hợp lệ>.`
5. Vẫn sai → kết quả là `none` (pick_element), `ask_user` (diagnose), hoặc nhãn tất định (label_step).

Mỗi lần gửi (kể cả gửi lại) tính 1 vào `model_calls` của lần chạy.

## Dữ liệu được phép gửi (FR-030, FR-033)

Được: `intent` của bước, loại hành động, tiêu đề cửa sổ, tên tiến trình, tên/vai trò/mã định danh/trạng
thái (`enabled`, `focused`, `selected`, `checked`) của phần tử, tên ô đích và **độ dài** văn bản cần gõ.
**Không bao giờ**: ảnh, giá trị bí mật, nội dung bộ nhớ tạm, văn bản đã/sẽ gõ, giá trị tham số, giá trị
(Value) của ô nhập. Bộ lọc nằm trong hàm dựng đầu vào; test tự động kiểm tra (SC-009).

**Che chuỗi trùng (FR-033)**: trước khi gửi, mọi đoạn trong phần user (tiêu đề cửa sổ, tên phần tử, ý định)
trùng — không phân biệt hoa thường — với văn bản đã gõ, giá trị tham số, hoặc giá trị bí mật của lần chạy/bản
ghi đều được thay bằng `‹đã ẩn›`. Chuỗi rỗng hoặc ngắn hơn 3 ký tự không đưa vào danh sách che (tránh che mất
mọi chữ "a"); giá trị bí mật thì luôn che bất kể độ dài.

## 1. `pick_element` (nấc 2)

Đầu vào (phần user):

```text
STEP: Bấm nút Lưu trên thanh công cụ
ACTION: click
WINDOW: "Hoá đơn - Excel" (EXCEL.EXE)
RECORDED TARGET: role=Button name="Lưu" automation_id=FileSave
CANDIDATES:
1. Button "Lưu lại" id=FileSave2 [enabled]
2. Button "Lưu thành..." id=FileSaveAs [enabled]
3. MenuItem "Lưu" [enabled]
Answer with one line of JSON: {"choice": <number>} or {"choice": "none"}
```

- Ứng viên: tối đa 40, lấy từ cửa sổ đích, lọc theo vai trò tương thích với hành động rồi xếp theo độ
  giống tên với target đã ghi; đánh số từ 1.
- Giá trị cho phép: `{"choice": n}` với 1 ≤ n ≤ số ứng viên, hoặc `{"choice": "none"}`.
- Kết quả được thao tác rồi kiểm tra `expect`; đạt → tự vá (thêm selector `role`+`name` hoặc
  `automation_id` của phần tử đã chọn, `pending_review: true`).

## 2. `diagnose` (nấc 4)

```text
STEP: Bấm nút Lưu trên thanh công cụ
EXPECTED: window title contains "Save As"
NOW: focused window "Microsoft Excel" (EXCEL.EXE)
NEW ELEMENTS:
1. Dialog "Microsoft Excel"
2. Text "Bạn có muốn lưu thay đổi?"
3. Button "Save"
4. Button "Don't Save"
5. Button "Cancel"
Answer with one line of JSON: {"action":"wait"} | {"action":"retry"} | {"action":"dismiss","choice":<number>} | {"action":"ask_user"}
```

- Giá trị cho phép: `wait` (chờ thêm 2 s rồi kiểm tra lại `expect`), `retry` (thực hiện lại hành động),
  `dismiss` với `choice` trỏ tới phần tử vai trò `Button` trong danh sách, `ask_user`.
- `dismiss` chỉ được bấm phần tử nằm trong danh sách NEW ELEMENTS (FR-029).
- Bước có `approval: true`: `retry` được đổi thành `ask_user` trước khi thực hiện (FR-028); log ghi cả câu trả
  gốc lẫn quyết định sau khi đổi. Không gọi `diagnose` ở bước có `expect` rỗng (FR-020).

## 3. `label_step` (lúc biên dịch)

```text
GOAL: Xuất báo cáo doanh thu tuần ra PDF
PREVIOUS: click Button "Lưu" in "Hoá đơn - Excel"
THIS: type 19 characters into Edit "File name:" in "Save As"
NEXT: click Button "Save" in "Save As"
LANGUAGE: vi
Answer with one line of JSON: {"intent": "<short sentence>"}
```

- Giá trị cho phép: chuỗi một dòng, 1–120 ký tự.
- Lỗi → nhãn tất định theo ngôn ngữ cài đặt: `Bấm "Lưu" (Button)` / `Click "Save" (Button)`,
  `Gõ vào ô "File name:"` / `Type into "File name:"`, `Nhấn Ctrl+S` / `Press Ctrl+S`.

## Prompt cố định

Mỗi loại có một system prompt và 2–3 ví dụ mẫu có lời giải, nhúng trong binary, có số phiên bản
(`PROMPT_VERSION`) ghi vào log mỗi lần gọi. Ngôn ngữ prompt: tiếng Anh (xem research.md R-7); tên phần tử
giữ nguyên ngôn ngữ gốc. Mọi thay đổi prompt phải chạy lại bộ câu hỏi mẫu (Constitution VI).
