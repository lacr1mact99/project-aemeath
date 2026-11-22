# Spec: ứng dụng Rust "Teach a Task" (dạy việc bằng cách làm mẫu)

Ngày: 2026-11-22 · Trạng thái: bản nháp v0.3, chưa có code

> Bản nháp gốc do người dùng cung cấp qua `/speckit-specify`, lưu nguyên văn làm đầu vào cho `/speckit-plan`. Spec chính thức: [spec.md](./spec.md).

Các quyết định đã chốt với người dùng:

1. Mục tiêu là **mọi ứng dụng desktop**, không riêng trình duyệt.
2. **Có GUI.**
3. LLM là **model do công ty tự host**; ứng dụng chỉ biết ba tham số: API key, base URL, tên model. Harness phải đủ chặt để model yếu (cỡ Haiku) vẫn làm được.
4. **Lên lịch ngay trong ứng dụng.**
5. **Model chỉ có văn bản (không đa phương thức) phải dùng được.**
6. Lịch đến giờ: **luôn hỏi rồi mới chạy**, kể cả khi máy đang rảnh.
7. Model công ty đang host: **DeepSeek và GLM** (phiên bản cụ thể chưa rõ).

## 1. Tính năng tham chiếu: Grok Bot "Teach a task"

Tóm tắt từ tài liệu chính thức của xAI (docs.x.ai) và các bài đánh giá bên thứ ba:

| Khía cạnh | Grok Bot làm gì | Độ tin cậy nguồn |
|---|---|---|
| Luồng dùng | Mở computer view → chọn *Teach a task* → mô tả kết quả mong muốn → tự làm workflow một lần → dừng ghi → xem lại skill được sinh ra | Chính thức |
| Ghi gì | Hoạt động nhìn thấy trên màn hình, tối đa 10 phút, không ghi âm micro | Chính thức |
| Đầu ra | Một **skill nháp**; người dùng phải bổ sung quy tắc quyết định, xử lý lỗi, ranh giới phê duyệt | Chính thức |
| Kiểm thử | Chạy thử trên một việc thật, an toàn, một lần, trước khi biến thành routine | Chính thức |
| Lên lịch | *Routines*: chạy theo lịch kiểu cron hoặc theo sự kiện | Bên thứ ba |
| Bí mật | Không đưa mật khẩu vào bản ghi; có luồng bàn giao thông tin đăng nhập riêng | Chính thức |
| Điểm yếu đã biết | Đường đi đã ghi dễ gãy khi giao diện đổi; agent có thể làm tiếp theo bước cũ mà không báo | Bên thứ ba |

Khác biệt so với sản phẩm này: Grok Bot dùng model lớn đa phương thức trên **máy ảo đám mây**; ứng dụng này thao tác **trên chính máy người dùng** và dùng **model nhỏ, chỉ văn bản, trên máy chủ của công ty**. Vì vậy kiến trúc bên dưới khác hẳn: phần lớn công việc do code làm, model chỉ được hỏi những câu hẹp.

## 2. Phạm vi

**Trong phạm vi**
- Windows 10/11, mọi ứng dụng desktop có cây accessibility (UI Automation); OCR làm phương án dự phòng cho ứng dụng không có.
- GUI: ghi, xem/sửa skill, chạy và theo dõi, phê duyệt, lên lịch, cài đặt.
- Model tự host của công ty, cấu hình bằng đúng ba tham số: API key, base URL, tên model. Giả định giao thức là chat completions tương thích OpenAI (kiểm tra ở M0).
- Lên lịch trong ứng dụng, chạy nền ở khay hệ thống.

**Ngoài phạm vi** (thêm khi có nhu cầu thật)
- macOS/Linux, đầu vào ảnh cho model đa phương thức, nhúng model vào trong tiến trình, vòng lặp/nhánh rẽ phức tạp trong skill, kích hoạt theo sự kiện, chia sẻ skill, ghi video, ghi âm.

## 3. Nguyên tắc thiết kế

Bốn nguyên tắc này là cách đáp ứng yêu cầu 3 và 5.

**3.1 Code làm trước, model chỉ sửa chữa.** Skill là danh sách bước có cấu trúc, mỗi bước mang sẵn bộ chọn phần tử lấy từ bản ghi. Khi chạy, harness tự tìm phần tử và tự kiểm tra kết quả. Nếu giao diện không đổi, cả lần chạy **không gọi model lần nào**. Model chỉ được gọi khi bộ chọn không khớp hoặc kết quả sai.

**3.2 Mỗi lần gọi model là một câu hỏi trắc nghiệm.** Model không bao giờ lập kế hoạch, không sinh toạ độ, không viết văn bản tự do lúc chạy. Nó chọn một số thứ tự trong danh sách ứng viên, hoặc một giá trị trong enum cố định. Vì không biết máy chủ có hỗ trợ structured output hay không, harness **không dựa vào nó**: câu trả lời được yêu cầu ngắn tới mức một dòng, harness tự phân tích và kiểm tra, sai thì hỏi lại.

**3.3 Mọi thứ model thấy đều là văn bản.** Màn hình được biểu diễn bằng danh sách phần tử UIA đã lọc và đánh số (tên, vai trò, trạng thái). Ứng dụng không có UIA thì dùng OCR tích hợp của Windows để ra danh sách chữ kèm vị trí. Ảnh chụp vẫn được lưu, nhưng chỉ để **người dùng** xem trong GUI.

**3.4 Kiểm tra bằng code, không bằng model.** Điều kiện trước và sau mỗi bước là các phép so khớp tất định (cửa sổ nào đang focus, phần tử có tồn tại không, giá trị ô có bằng X không). Model không được tự tuyên bố "xong".

## 4. Kiến trúc

Một binary `teach.exe` (GUI + chạy nền ở khay). Dữ liệu là file trong `%APPDATA%\teach\`, không dùng cơ sở dữ liệu.

```
Recorder ──► trace/ (events.jsonl + ảnh)
                │  tiền xử lý tất định + model gắn nhãn từng bước
Compiler ───────┴──► skills/<tên>/skill.yaml  ◄── người dùng sửa trong GUI
                                │
Runner ◄────────────────────────┘   bậc thang phân giải (mục 4.3)
   ▲
Scheduler (trong tiến trình khay)
```

### 4.1 Recorder

- Bắt đầu từ GUI: nhập mô tả kết quả → cửa sổ thu nhỏ thành thanh nổi có Tạm dừng/Dừng; có hotkey toàn cục tương ứng. Tự dừng sau 10 phút.
- Mỗi sự kiện một dòng JSON:
  ```json
  {"t_ms":4210,"kind":"click","button":"left","x":812,"y":440,
   "window":{"title":"Hoá đơn - Excel","process":"EXCEL.EXE","rect":[0,0,1920,1040]},
   "element":{"name":"Lưu","role":"Button","automation_id":"FileSave",
              "path":["Window","Ribbon","Button"],"rect":[790,428,834,452]},
   "shot_before":"0007a.png","shot_after":"0007b.png"}
  ```
- Loại sự kiện: `click`, `scroll`, `type` (gộp phím liên tiếp thành một đoạn), `hotkey`, `window_focus`, `clipboard`. Bỏ di chuột thuần tuý.
- Sau mỗi hành động, lưu thêm **ảnh chụp trạng thái dạng văn bản**: tiêu đề cửa sổ focus và danh sách phần tử UIA mới xuất hiện. Compiler dùng nó để sinh điều kiện sau.
- Nếu phần tử dưới con trỏ không có thông tin UIA hữu ích, chạy OCR vùng quanh điểm bấm và lưu chữ gần nhất làm `ocr_text`.
- **Riêng tư (bắt buộc):** không ghi phím khi phần tử focus có `IsPassword`; thay bằng `{"kind":"type","secret":true}` và che vùng đó trong ảnh. Không ghi âm. Bản ghi và ảnh chụp chỉ lưu trên máy.
- **Thứ được gửi tới máy chủ model:** chỉ văn bản của từng lần gọi, gồm ý định của bước, tiêu đề cửa sổ, tên và vai trò các phần tử ứng viên. Ảnh chụp, giá trị bí mật và nội dung clipboard không bao giờ được gửi. Tên phần tử có thể chứa dữ liệu nghiệp vụ (tên file, tên khách hàng), nên GUI phải nói rõ điều này ở lần cấu hình đầu.

### 4.2 Compiler (bản ghi → skill)

Chia thành các bước nhỏ; chỉ bước 3 cần model.

1. **Tiền xử lý (code):** gộp phím, bỏ nhiễu, bỏ thao tác trên chính cửa sổ của ứng dụng này, tách bước theo hành động có nghĩa.
2. **Sinh bộ chọn và điều kiện (code):** mỗi bước nhận danh sách bộ chọn theo thứ tự ưu tiên (`automation_id` → `role+name` → `ocr_text` → toạ độ tương đối cửa sổ) và điều kiện sau suy ra từ ảnh chụp trạng thái văn bản.
3. **Gắn nhãn ý định (model, từng bước một):** đầu vào là một bước cùng bước trước và sau nó; đầu ra là một câu `intent` ngắn. Nếu model lỗi, dùng nhãn tất định kiểu `Bấm "Lưu" (Button)`. Skill vẫn chạy được khi không có model.
4. **Người dùng hoàn thiện trong GUI:** đánh dấu đoạn văn bản nào là tham số đầu vào, bước nào cần phê duyệt, thêm điều kiện bỏ qua bước. Việc tham số hoá do người dùng quyết, không giao cho model đoán.

Định dạng `skill.yaml`:

```yaml
name: xuat-bao-cao-tuan
description: Xuất báo cáo doanh thu tuần ra PDF
status: draft            # draft → ready sau một lần chạy thành công được người dùng xác nhận
inputs:
  - {name: tuan, description: "Tuần cần xuất", example: "2026-W47"}
steps:
  - id: 3
    intent: Bấm nút Lưu trên thanh công cụ
    action: click          # click | type | hotkey | scroll | wait | launch
    window: {process: EXCEL.EXE, title_contains: Excel}
    target:
      - {automation_id: FileSave}
      - {role: Button, name: Lưu}
      - {ocr_text: Lưu}
      - {xy_in_window: [0.42, 0.41]}
    expect: {window_title_contains: "Save As"}
    approval: false
    skip_if: null          # ví dụ {element_exists: {name: "Đã lưu"}}
  - id: 4
    intent: Nhập tên file
    action: type
    text: "BaoCao-{{tuan}}.pdf"
    target: [{role: Edit, name: "File name:"}]
    expect: {target_value_equals: "BaoCao-{{tuan}}.pdf"}
```

Bí mật khai báo dạng `text: "{{secret:ten_khoa}}"`, lấy từ Windows Credential Manager lúc chạy, không bao giờ vào prompt hay log.

### 4.3 Runner: bậc thang phân giải

Với mỗi bước, dừng ở nấc đầu tiên thành công:

| Nấc | Ai làm | Việc |
|---|---|---|
| 0 | Code | Kiểm tra `skip_if`; đưa đúng cửa sổ lên trước (mở ứng dụng nếu chưa chạy) |
| 1 | Code | Thử lần lượt các bộ chọn trong `target`. Khớp đúng một phần tử → hành động, không gọi model |
| 2 | Model: `pick_element` | Không khớp hoặc khớp nhiều: liệt kê tối đa 40 ứng viên đã lọc theo vai trò và độ giống tên, đánh số. Model trả `{"choice": <số> \| "none"}` |
| 3 | Code | Thực hiện hành động, đợi giao diện ổn định, kiểm tra `expect` |
| 4 | Model: `diagnose` | `expect` sai: đưa tiêu đề cửa sổ hiện tại và danh sách phần tử mới. Model chọn một trong `wait`, `retry`, `dismiss_dialog(<số>)`, `ask_user` |
| 5 | Người dùng | Hết ngân sách (mặc định 2 lần sửa mỗi bước): tạm dừng, hiện bước đang kẹt trong GUI để người dùng làm tay rồi bấm Tiếp tục, hoặc huỷ |

Khi nấc 2 thành công và `expect` đạt, bộ chọn mới được thêm vào `target` của bước đó và ghi log (tự vá).

**Hợp đồng gọi model** (áp dụng cho `pick_element`, `diagnose`, `label_step`):
- Một prompt cố định cho mỗi loại, kèm 2–3 ví dụ mẫu có lời giải.
- Ngữ cảnh chỉ gồm bước hiện tại và danh sách ứng viên, dưới khoảng 2.000 token. Không đưa lịch sử hay toàn bộ skill.
- Đầu ra là một dòng JSON nhỏ, ví dụ `{"choice": 7}`. Harness lấy đối tượng JSON đầu tiên trong câu trả lời (bỏ qua chữ thừa và khối suy nghĩ nếu có), kiểm tra giá trị nằm trong danh sách cho phép. Sai định dạng thì hỏi lại một lần kèm thông báo lỗi; vẫn sai thì coi là `none`/`ask_user`.
- Nếu M0 cho thấy máy chủ hỗ trợ `response_format` theo JSON Schema thì bật thêm, nhưng bước phân tích và kiểm tra ở trên vẫn giữ nguyên.
- Hai họ model đang có là DeepSeek và GLM. Một số bản của cả hai là model suy luận, có thể trả phần suy nghĩ trong thẻ `<think>` hoặc trong trường riêng; harness chỉ đọc phần nội dung trả lời và bỏ mọi thứ trước đối tượng JSON. Ứng dụng dùng một model tại một thời điểm (tên model trong Cài đặt); không có cơ chế định tuyến giữa hai model.
- Nhiệt độ 0. Quá thời gian, lỗi kết nối hoặc lỗi xác thực được xử lý như `ask_user`; GUI hiện rõ nguyên nhân.
- API key lưu trong Windows Credential Manager, không nằm trong file cấu hình hay log.

**An toàn (bắt buộc):**
- Bước có `approval: true` luôn dừng chờ người dùng bấm Đồng ý trong GUI.
- Hotkey dừng khẩn cấp luôn hoạt động; chuột/phím của người dùng trong lúc chạy cũng làm tạm dừng.
- Giới hạn tổng thời gian và tổng số lần gọi model mỗi lần chạy.
- Model không có hành động nào ngoài hai enum ở trên, nên không thể tự ý bấm thứ nằm ngoài bước đang chạy, trừ nút đóng hộp thoại.
- Mỗi lần chạy ghi log (bước, nấc đã dùng, prompt, câu trả lời, ảnh) vào `runs/<thời điểm>/`.

### 4.4 Scheduler

- Chạy trong tiến trình khay hệ thống; tuỳ chọn khởi động cùng Windows.
- Mỗi lịch gồm: skill, giá trị đầu vào, biểu thức thời gian. GUI cung cấp mẫu sẵn (hằng ngày, hằng tuần, mỗi N phút) và ô cron cho người cần.
- **Đến giờ mà người dùng đang dùng máy** (có chuột/phím trong vài phút gần đây): hiện hộp hỏi với ba lựa chọn Chạy ngay, Hoãn N phút, Bỏ qua lần này. Không trả lời trong thời hạn thì hỏi lại khi máy rảnh; không bao giờ tự chạy đè lên người dùng.
- **Đến giờ mà máy rảnh và đang mở khoá:** vẫn hiện đúng hộp hỏi đó kèm thông báo Windows, và chờ. Không có chế độ tự chạy: mọi lần chạy theo lịch đều cần một lần bấm đồng ý của người dùng.
- Hệ quả: lịch là lời nhắc có sẵn nút chạy, không phải tự động hoá không người trông. Nếu không ai ngồi ở máy thì lần chạy nằm chờ, và hết thời hạn chờ của lịch đó thì bị ghi là bỏ lỡ.
- **Ràng buộc:** tự động hoá giao diện cần phiên Windows đang mở khoá. Máy đang khoá thì lần chạy chờ tới khi mở khoá rồi hỏi như trên; mọi lần hoãn hay bỏ qua đều ghi vào lịch sử.
- Chạy theo lịch mà gặp bước cần phê duyệt hoặc nấc 5: gửi thông báo Windows và chờ có thời hạn; hết hạn thì đánh dấu thất bại. Không có chế độ tự phê duyệt.
- Chỉ skill ở trạng thái `ready` mới được lên lịch. Không chạy hai skill cùng lúc.

### 4.5 GUI

| Màn hình | Nội dung |
|---|---|
| Danh sách skill | Tên, trạng thái, lần chạy gần nhất; nút Dạy việc mới, Chạy, Sửa |
| Thanh ghi | Thanh nổi nhỏ khi đang ghi: thời gian, Tạm dừng, Dừng |
| Sửa skill | Danh sách bước; mỗi bước có ảnh trước/sau, ý định, bộ chọn, điều kiện; đánh dấu tham số, phê duyệt, điều kiện bỏ qua; xoá và sắp xếp lại bước |
| Theo dõi chạy | Bước hiện tại, nấc đang dùng, log trực tiếp; nút Đồng ý/Từ chối, Tiếp tục, Dừng |
| Lịch | Danh sách lịch, lần chạy kế tiếp, lịch sử |
| Cài đặt | Base URL, API key, tên model; nút Kiểm tra kết nối và Kiểm tra model; hotkey, thư mục dữ liệu |

## 5. Công nghệ

| Nhu cầu | Lựa chọn | Ghi chú |
|---|---|---|
| GUI | `eframe`/`egui` + `tray-icon` | Thuần Rust, một binary, không cần stack web. Đổi sang Tauri nếu sau này cần giao diện trau chuốt hơn |
| Ghi workflow | `terminator-workflow-recorder` | Chỉ Windows; có sẵn click/phím/clipboard/sự kiện UIA. **Rủi ro:** bản 0.23.35 build tài liệu lỗi trên docs.rs, phải build thử ở M0 |
| Dự phòng cho ghi | `rdev` + `uiautomation` | `uiautomation` chưa được kiểm chứng |
| Tìm và thao tác phần tử | `terminator-rs` | API kiểu Playwright cho desktop |
| Giả lập chuột/phím | `enigo` | Dùng pixel vật lý, tự xử lý DPI |
| Chụp màn hình | `xcap` | Chỉ phục vụ người xem và OCR |
| OCR | `Windows.Media.Ocr` qua crate `windows` | Có sẵn trong Windows, không thêm phụ thuộc nặng. Chất lượng tiếng Việt cần đo ở M0 |
| Model | `reqwest` gọi `POST {base_url}/chat/completions` với header `Authorization: Bearer <key>` | Giả định tương thích OpenAI, chưa xác nhận. Không phụ thuộc structured output, tool calling hay đầu vào ảnh |
| Lịch | `cron` | Chỉ để phân tích biểu thức và tính lần chạy kế tiếp |
| Còn lại | `serde`, `serde_yaml`, `serde_json`, `tokio`, `reqwest`, `keyring` | |

## 6. Bộ đo harness

Yêu cầu "model yếu vẫn làm được" chỉ có nghĩa khi đo được. Bộ đo gồm:

- **10 skill chuẩn** ghi trên các ứng dụng có sẵn (Notepad, Explorer, Calculator, Settings, một trình duyệt, một ứng dụng Office hoặc tương đương).
- **Ba chế độ chạy cho mỗi skill:**
  1. Giao diện không đổi, model tắt hẳn → kiểm chứng nguyên tắc 3.1.
  2. Bộ chọn chính bị làm hỏng có chủ đích (xoá `automation_id`, đổi tên) → buộc gọi `pick_element`.
  3. Chèn hộp thoại bất ngờ giữa chừng → buộc gọi `diagnose`.
- **Bộ kiểm tra ngoại tuyến:** các prompt `pick_element`/`diagnose` đã lưu kèm đáp án đúng, chạy lại được với bất kỳ model nào mà không cần giao diện. Nút Kiểm tra model trong Cài đặt chạy bộ này và báo tỉ lệ đúng.

## 7. Lộ trình và tiêu chí nghiệm thu

| Mốc | Nội dung | Nghiệm thu |
|---|---|---|
| M0 | Thử nghiệm kỹ thuật | Build được recorder; ghi được click kèm phần tử UIA trên Notepad và một trình duyệt; OCR đọc được nhãn nút tiếng Việt; gọi được model công ty bằng ba tham số đã cho và xác định: giao thức có tương thích OpenAI không, có structured output không, giới hạn ngữ cảnh và tốc độ, có trả phần suy nghĩ không; làm phép đo này cho cả DeepSeek và GLM; mỗi model trả được JSON một dòng hợp lệ ≥ 19/20 lần sau bước phân tích |
| M1 | Recorder + Compiler (chưa có model) | Ghi workflow 5 bước → `skill.yaml` hợp lệ có bộ chọn và `expect`; phím trong ô mật khẩu bị che |
| M2 | Runner nấc 0, 1, 3, 5 (chưa có model) | Bộ đo chế độ 1: 10/10 skill chạy xong mà không gọi model; dừng khẩn cấp có tác dụng trong 1 giây |
| M3 | Gọi model: `pick_element`, `diagnose`, `label_step`, tự vá | Chạy với cả DeepSeek và GLM, chỉ dùng văn bản, rồi chọn model mặc định theo kết quả. Model được chọn phải đạt: bộ kiểm tra ngoại tuyến ≥ 90% đúng; bộ đo chế độ 2 và 3 ≥ 8/10; không lần nào bấm sai phần tử mà không bị `expect` bắt |
| M4 | GUI đầy đủ | Làm trọn vòng dạy → sửa → chạy → phê duyệt chỉ bằng GUI |
| M5 | Scheduler + khay | Đến giờ: luôn hiện hộp hỏi, kể cả khi máy rảnh, và không lần nào chạy khi chưa được đồng ý; máy khoá: chờ mở khoá rồi hỏi; hết thời hạn chờ thì ghi là bỏ lỡ; bước cần phê duyệt gửi thông báo và chờ |

M2 đứng trước M3 có chủ đích: phần tất định phải tự đứng vững trước khi thêm model.

## 8. Rủi ro

- **Ứng dụng không có UIA và OCR đọc kém** (game, canvas, chữ nhỏ): chỉ còn toạ độ tương đối, không tự vá được. GUI phải cảnh báo ngay lúc ghi khi một bước chỉ có bộ chọn toạ độ.
- **Agent thao tác thật trên máy thật**, không có máy ảo cách ly. Cổng phê duyệt, dừng khẩn cấp và `expect` sau mỗi bước là lớp bảo vệ chính.
- **Một lần làm mẫu không thể hiện nhánh rẽ.** Skill chỉ hỗ trợ `skip_if` đơn giản; workflow cần logic phức tạp hơn nằm ngoài khả năng bản này.
- **Điều kiện sau suy ra tự động có thể quá lỏng hoặc quá chặt.** Người dùng sửa được trong GUI; lần chạy thử đầu tiên là để phát hiện việc này.
- **Model nhỏ chọn sai nhưng tự tin.** Giảm thiểu bằng danh sách ứng viên đã lọc, lựa chọn `none`, và `expect` kiểm tra sau hành động. Tỉ lệ thực tế chỉ biết sau M3.
- **Lịch phụ thuộc phiên đang mở khoá.** Không chạy được khi máy khoá hay đã đăng xuất; cần nói rõ trong GUI.
- **Không biết gì về máy chủ model ngoài ba tham số.** Giao thức, structured output, giới hạn ngữ cảnh, tốc độ và hạn mức đều là ẩn số cho tới M0. Thiết kế đã chọn mẫu số chung thấp nhất (chat văn bản thuần), nên rủi ro còn lại chủ yếu là giao thức không tương thích OpenAI; khi đó chỉ phải viết lại một hàm gọi HTTP.
- **Phụ thuộc mạng công ty.** Mất kết nối hoặc ngoài VPN thì các nấc 2 và 4 không dùng được; skill có giao diện không đổi vẫn chạy bình thường, bước cần sửa chữa sẽ chuyển thẳng sang hỏi người dùng.
- **Văn bản giao diện được gửi tới máy chủ công ty.** Đây là máy chủ nội bộ nên rủi ro thấp hơn dịch vụ bên ngoài, nhưng vẫn cần người dùng biết.

## 9. Câu hỏi còn mở

Không còn câu hỏi nào chặn việc viết spec. Các ẩn số về máy chủ model được trả lời bằng phép đo ở M0, không cần hỏi thêm.

## Nguồn

- [xAI docs: Skills and routines](https://docs.x.ai/grok-bot/skills-routines-and-automations)
- [xAI docs: FAQ](https://docs.x.ai/grok-bot/faq)
- [xAI docs: Grok Bot overview](https://docs.x.ai/grok-bot/overview)
- [xAI docs: Approvals, security, and privacy](https://docs.x.ai/grok-bot/approvals-security-and-privacy)
- [Layer3 Labs: Grok Bot Teach-a-Task](https://www.layer3labs.io/guides/grok-bot-teach-a-task)
- [Layer3 Labs: Grok Bot review](https://www.layer3labs.io/guides/grok-bot-review)
- [MindStudio: What is Grokbot?](https://www.mindstudio.ai/blog/grokbot-xai-agent-app)
- [Daily Dose of DS: Grok Bot Masterclass](https://www.dailydoseofds.com/p/grok-bot-masterclass/)
- [docs.rs: terminator-workflow-recorder](https://docs.rs/terminator-workflow-recorder)
- [docs.rs: terminator-rs](https://docs.rs/crate/terminator-rs/0.23.9)
- [docs.rs: xcap](https://docs.rs/crate/xcap/0.9.3/source/README.md)
- [docs.rs: enigo](https://docs.rs/crate/enigo/latest)
- [GitHub: rdev](https://github.com/Narsil/rdev)
