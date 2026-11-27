# Contract: `skill.yaml`

Định dạng skill người dùng đọc/sửa được. Ứng dụng là bên duy nhất ghi file này; người dùng sửa qua giao diện
002 hoặc bề mặt điều khiển (`save_skill`), nhưng file vẫn phải hợp lệ khi sửa tay. File là **đầu vào không tin
cậy** (FR-047): không có trường `status` — trạng thái được tính lại mỗi lần tải từ `confirmed.fingerprint` và dấu
vân tay thực thi hiện tại (FR-017). Ngữ nghĩa trường: [data-model.md](../data-model.md#skill).

## Ví dụ đầy đủ

```yaml
version: 1
name: xuat-bao-cao-tuan
description: Xuất báo cáo doanh thu tuần ra PDF
source_trace: 20261122-091500
confirmed: {run: 20261122-093012, fingerprint: "9f2c41d07ab35e18"}   # ready nếu trùng dấu vân tay hiện tại
inputs:
  - {name: tuan, description: "Tuần cần xuất", example: "2026-W47"}
steps:
  - id: 1
    intent: Mở Excel
    action: launch
    window: {process: EXCEL.EXE, process_path: 'C:\Program Files\Microsoft Office\root\Office16\EXCEL.EXE'}
    launch: {path: 'C:\Program Files\Microsoft Office\root\Office16\EXCEL.EXE', args: []}
    expect: [{focused_process: EXCEL.EXE}]
  - id: 3
    intent: Bấm nút Lưu trên thanh công cụ
    action: click
    window: {process: EXCEL.EXE, title_contains: Excel}
    target:
      - {automation_id: FileSave}
      - {role: Button, name: Lưu}
      - {ocr_text: Lưu}
      - {xy_in_window: [0.42, 0.41], at: {role: Button, name: Lưu}}   # chỉ khớp khi phần tử tại điểm là Button "Lưu"
      - {role: Button, name: "Lưu lại", patch: {id: p-20261129-080102-3, run: 20261129-080102, pending_review: true}}
      # lúc chạy, selector có `patch` được thử TRƯỚC xy_in_window (data-model §Thứ tự thử selector)
    expect: [{window_title_contains: "Save As"}]
    approval: false
    repeat_safe: true          # người dùng bật: bấm lại nút mở hộp thoại Lưu không gây hậu quả (FR-028)
    skip_if: {element_exists: {role: Text, name: "Đã lưu"}}
  - id: 4
    intent: Nhập tên file
    action: type
    window: {process: EXCEL.EXE, title_contains: "Save As"}
    text: "BaoCao-{{tuan}}.pdf"
    target: [{role: Edit, name: "File name:"}]
    expect: [{target_value_equals: "BaoCao-{{tuan}}.pdf"}]
  - id: 5
    intent: Đăng nhập cổng báo cáo
    action: type
    window: {process: msedge.exe}
    text: "{{secret:cong_bao_cao}}"
    target: [{role: Edit, name: "Mật khẩu"}]
    expect: []
    approval: true
```

## Quy tắc hợp lệ (kiểm tra khi tải và khi lưu)

1. `version` = 1. Trường lạ → lỗi (không âm thầm bỏ qua).
2. Mỗi bước có đúng các trường bắt buộc theo `action` (bảng Step trong data-model). Trường không thuộc
   `action` đó → lỗi.
3. Mỗi selector có đúng một dạng chính (`automation_id` | `role`+`name` | `ocr_text` | `xy_in_window`).
   `at` chỉ đi kèm `xy_in_window` (là điều kiện chặn, không phải dạng chính thứ hai). Selector gốc (không có `patch`) đứng trước mọi selector có `patch`; `xy_in_window` chỉ được là selector gốc
   cuối cùng và không bao giờ có `patch`. `patch.id` duy nhất trong skill. Thứ tự **thử** lúc chạy khác thứ tự
   trong file: selector có `patch` được thử trước `xy_in_window` (data-model).
7. Không có trường `status`, `last_confirmed_run` (bản cũ) → trường lạ → lỗi.
8. `repeat_safe` chỉ được có ở bước `click`, `hotkey`, `type`; `repeat_safe: true` cùng `approval: true` → lỗi
   (phê duyệt luôn thắng, FR-028).
4. `{{x}}` phải khớp một Input; `{{secret:k}}` chỉ được xuất hiện trong `text`, `k` theo `[a-z0-9_]+`.
5. `expect`, `skip_if` không chứa `{{secret:...}}` và không dùng `xy_in_window`.
6. Không có mẫu escape cho `{{` trong v1: chuỗi chứa `{{` không khớp quy tắc 4 → lỗi.

## Thay thế giá trị lúc chạy

`{{name}}` → giá trị Input của lần chạy. `{{secret:k}}` → giá trị đọc từ Credential Manager **ngay
trước khi gõ**, không lưu vào biến log, không đi qua bất kỳ chuỗi nào được ghi ra file hoặc gửi model.
Log ghi `text` ở dạng chưa thay thế bí mật (vẫn là `{{secret:k}}`).
