# Contract: bản ghi `events.jsonl`

Mỗi dòng một sự kiện JSON. Do Recorder ghi, Compiler đọc. Bất biến sau khi ghi xong.

## Trường chung

| Trường | Kiểu | Ghi chú |
|---|---|---|
| t_ms | số | ms tính từ lúc bắt đầu ghi (không tính thời gian tạm dừng) |
| kind | enum | `click` \| `scroll` \| `type` \| `hotkey` \| `window_focus` \| `clipboard` \| `state` |
| window | object | `{title, process, process_path, rect:[x,y,w,h]}` của cửa sổ focus |
| element | object \| null | `{name, role, automation_id, class_name, path:[...], rect, is_password}` |
| ocr_text | chuỗi \| null | chữ gần điểm bấm nhất, chỉ khi `element` thiếu tên/mã hữu ích (FR-004) |
| shot_before / shot_after | chuỗi \| null | tên file trong `shots\`; `null` khi không chụp được hoặc bị bỏ vì bí mật không xác định được vùng (FR-006) |
| uncertain | danh sách chuỗi | phần dữ liệu có thể không thuộc đúng hành động (FR-003): `element`, `shot_before`, `state`; rỗng thì bỏ trường |
| lag_ms | số | thời điểm luồng xử lý bắt đầu xử lý sự kiện − `t_ms`; dùng để đặt `uncertain` |

## Theo loại

```json
{"t_ms":4210,"kind":"click","button":"left","x":812,"y":440,"double":false,
 "window":{"title":"Hoá đơn - Excel","process":"EXCEL.EXE","process_path":"C:\\...\\EXCEL.EXE","rect":[0,0,1920,1040]},
 "element":{"name":"Lưu","role":"Button","automation_id":"FileSave","class_name":"NetUIRibbonButton",
            "path":["Window","Pane","ToolBar","Button"],"rect":[790,428,44,24],"is_password":false},
 "ocr_text":null,"shot_before":"0007a.png","shot_after":"0007b.png"}
{"t_ms":5100,"kind":"type","text":"BaoCao-2026-W47.pdf","window":{...},"element":{...,"is_password":false}}
{"t_ms":6100,"kind":"type","secret":true,"length":null,"window":{...},"element":{...,"is_password":true}}
{"t_ms":7000,"kind":"hotkey","keys":"Ctrl+S","window":{...}}
{"t_ms":7400,"kind":"scroll","dx":0,"dy":-3,"x":900,"y":500,"window":{...},"element":{...}}
{"t_ms":8000,"kind":"window_focus","window":{...}}
{"t_ms":8100,"kind":"clipboard","op":"copy","source":"hotkey","cause_t_ms":8090,"window":{...}}
{"t_ms":8200,"kind":"clipboard","op":"paste","source":"menu","cause_t_ms":8180,"window":{...}}
{"t_ms":8300,"kind":"state","after_t_ms":8000,"focused_title":"Save As",
 "new_elements":[{"name":"File name:","role":"Edit","automation_id":"1001"},{"name":"Save","role":"Button"}]}
```

## Quy tắc bắt buộc

- `type` với `element.is_password = true` (hoặc không xác định) MUST có `"secret": true` và MUST NOT có trường
  `text`; ảnh trước/sau MUST bị che vùng `element.rect` trước khi ghi xuống đĩa; không có `element.rect` → không
  ghi ảnh (`shot_before`/`shot_after` = `null`) (FR-006).
- **Gán dữ liệu cho đúng hành động** (FR-003): `lag_ms` vượt ngưỡng `RECORD_LAG_LIMIT_MS` (M0 chốt, ghi vào
  research.md) hoặc cửa sổ focus lúc xử lý khác cửa sổ lúc sự kiện xảy ra → đặt `uncertain` cho `element` và
  `shot_before`; `state` của hành động mà hành động kế tiếp xảy ra trước khi kịp đọc trạng thái → `uncertain` có
  `state`. Compiler MUST NOT sinh selector từ `element` không chắc chắn (rơi về `ocr_text`/`xy_in_window` từ toạ độ
  thô) và MUST NOT sinh `expect` từ `state` không chắc chắn (bước được `unverifiable`).
- Ảnh chỉ gồm vùng cửa sổ đích + cửa sổ/hộp thoại cùng tiến trình; vùng bị cửa sổ tiến trình khác chồng lên tô
  kín trước khi ghi (FR-003). Không bao giờ ghi ảnh toàn màn hình.
- Phiên khoá / màn hình UAC trong lúc ghi: không ghi sự kiện và không chụp cho tới khi trở lại desktop thường.
- `clipboard` MUST NOT chứa nội dung bộ nhớ tạm — chỉ thao tác (`copy` | `paste` | `cut`), nguồn
  (`source`: `hotkey` | `menu` | `unknown`) và `cause_t_ms` trỏ tới sự kiện `hotkey`/`click` gây ra nó.
- Di chuột thuần tuý không được ghi. Sự kiện trên cửa sổ của chính `teach.exe` không được ghi.
- `state` được ghi sau mỗi hành động khi giao diện ổn định (cây phần tử + tiêu đề cửa sổ đích không đổi trong 300 ms liên tiếp; chờ tối đa 1,5 s khi ghi để không làm chậm người dùng — cùng định nghĩa "ổn định" với lúc chạy, nơi chờ tối đa 5 s), là nguồn để Compiler sinh `expect`.
  `new_elements` = phần tử có trong cây cửa sổ focus sau hành động mà không có trước đó (giới hạn 50).
  `new_elements` MUST NOT chứa giá trị (Value) của ô nhập.
- Phím liên tiếp vào cùng một phần tử được Recorder gộp thành một `type`; phím có Ctrl/Alt/Win → `hotkey`.

## Ánh xạ sự kiện → bước (Compiler)

- `window_focus` không tạo bước riêng: nó cập nhật `window` của bước kế tiếp. Nếu cửa sổ thuộc ứng dụng chưa mở ở đầu bản ghi, bước đầu tiên trên nó được đi kèm một bước `launch` với `process_path`.
- `clipboard` không tạo bước riêng: nó gắn nhãn cho sự kiện gốc `cause_t_ms` (bước `hotkey` hoặc `click` menu
  tương ứng), để không bao giờ sinh hai bước cho một lần sao chép/dán. Bước gắn `op: paste` mà trước nó không có
  bước nào gắn `copy`/`cut` → cảnh báo `clipboard_external` (FR-002). `source: unknown` (không tìm được sự kiện gốc)
  → không sinh bước, chỉ ghi cảnh báo cho bước kế tiếp.
- `wait` không sinh tự động (người dùng tự thêm khi sửa skill).
