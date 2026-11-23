# Contract: `skill.yaml`

Định dạng skill người dùng đọc/sửa được. Ứng dụng là bên duy nhất ghi file này; người dùng sửa qua giao diện
002 hoặc bề mặt điều khiển (`save_skill`), nhưng file vẫn phải hợp lệ khi sửa tay. Ngữ nghĩa trường: [data-model.md](../data-model.md#skill).

## Ví dụ đầy đủ

```yaml
version: 1
name: xuat-bao-cao-tuan
description: Xuất báo cáo doanh thu tuần ra PDF
status: ready
source_trace: 20261122-091500
last_confirmed_run: 20261122-093012
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
      - {xy_in_window: [0.42, 0.41]}
      - {role: Button, name: "Lưu lại", added_by_run: 20261129-080102, pending_review: true}
    expect: [{window_title_contains: "Save As"}]
    approval: false
    skip_if: {element_exists: {name: "Đã lưu"}}
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
   `xy_in_window` chỉ được nằm ở vị trí cuối danh sách selector "gốc" hoặc sau đó là các selector tự vá.
4. `{{x}}` phải khớp một Input; `{{secret:k}}` chỉ được xuất hiện trong `text`, `k` theo `[a-z0-9_]+`.
5. `expect`, `skip_if` không chứa `{{secret:...}}` và không dùng `xy_in_window`.
6. Không có mẫu escape cho `{{` trong v1: chuỗi chứa `{{` không khớp quy tắc 4 → lỗi.

## Thay thế giá trị lúc chạy

`{{name}}` → giá trị Input của lần chạy. `{{secret:k}}` → giá trị đọc từ Credential Manager **ngay
trước khi gõ**, không lưu vào biến log, không đi qua bất kỳ chuỗi nào được ghi ra file hoặc gửi model.
Log ghi `text` ở dạng chưa thay thế bí mật (vẫn là `{{secret:k}}`).
