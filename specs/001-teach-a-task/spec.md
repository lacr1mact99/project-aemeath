# Feature Specification: Teach a Task — dạy máy làm việc bằng cách làm mẫu

**Feature Branch**: `001-teach-a-task`

**Created**: 2026-11-22

**Status**: Draft

**Input**: User description: "nạp spec — Spec: ứng dụng Rust 'Teach a Task' (dạy việc bằng cách làm mẫu), bản nháp v0.3". Bản nháp gốc (gồm các lựa chọn công nghệ, kiến trúc, lộ trình M0–M5) được giữ nguyên văn ở [draft-v0.3.md](./draft-v0.3.md) làm đầu vào cho `/speckit-plan`.

## Bối cảnh

Người dùng văn phòng lặp lại cùng một chuỗi thao tác trên ứng dụng desktop (xuất báo cáo, điền biểu mẫu, lưu file). Sản phẩm cho phép họ **làm mẫu một lần**, nhận lại một **kỹ năng (skill)** có thể chạy lại trên chính máy mình, kể cả theo lịch. Khác với các sản phẩm tương tự dùng model lớn đa phương thức trên máy ảo đám mây, sản phẩm này:

- thao tác **trên chính máy người dùng** (Windows 10/11, mọi ứng dụng desktop);
- dùng **model ngôn ngữ do công ty tự host, chỉ nhận văn bản, có thể yếu** (cỡ model nhỏ), cấu hình bằng đúng ba tham số: khoá truy cập, địa chỉ máy chủ, tên model;
- vì vậy **phần lớn công việc do chương trình làm theo cách tất định**; model chỉ được hỏi những câu trắc nghiệm hẹp khi có sự cố.

**Ranh giới với spec 002**: spec này là **lõi** — ghi, biên dịch, chạy, hỏi model, quy tắc lịch, lưu trữ, riêng tư, và mọi quy tắc an toàn. Toàn bộ **giao diện người dùng** (màn hình, thanh ghi nổi, khay, hộp hỏi, thông báo Windows) nằm ở [002-tauri-gui](../002-tauri-gui/spec.md). Lõi quyết định; giao diện hiển thị và chuyển lựa chọn của người dùng. Lõi MUST dùng được và kiểm chứng được không cần giao diện (qua dòng lệnh và phím tắt toàn cục). Yêu cầu nào đã chuyển sang 002 được giữ số và ghi "→ 002".

## Clarifications

### Session 2026-11-22

- Q: Khi model sửa được một bước, có tự ghi bộ chọn mới vào skill luôn hay chờ người dùng duyệt? → A: Tự vá ngay, skill giữ "sẵn sàng"; cuối lần chạy hiện danh sách các bước đã được vá, người dùng có thể hoàn tác từng bước.
- Q: Khi nhờ model viết câu ý định cho bước gõ chữ, có gửi kèm nội dung đã gõ không? → A: Không; chỉ gửi "gõ N ký tự vào ô <tên ô>".
- Q: Giao diện và các câu mô tả tự sinh dùng ngôn ngữ nào? → A: Tiếng Việt và tiếng Anh, đổi trong Cài đặt.
- Q: Giữ nguyên các giá trị mặc định về thời gian và ngân sách? → A: Giữ nguyên toàn bộ (3 phút / 2 giờ / 30 phút / 5 phút / 30 phút + 20 lần gọi / 2 lần sửa mỗi bước / 30 giây).
- Q: Log các lần chạy (kèm ảnh) được giữ bao lâu? → A: Giữ 50 lần chạy gần nhất cho mỗi skill, cũ hơn tự xoá; người dùng chỉnh được con số.

### Session 2026-11-23

- Q: Phần giao diện nằm ở đâu, dùng công nghệ gì? → A: Tách sang spec 002 (`002-tauri-gui`), giao diện làm bằng Tauri; spec 001 chỉ còn lõi.
- Q: Khi tiêu đề cửa sổ hoặc tên phần tử chứa đúng văn bản người dùng đã gõ, xử lý thế nào trước khi gửi model? → A: Lọc trước khi gửi: mọi đoạn trùng (không phân biệt hoa thường) với văn bản đã gõ, giá trị tham số hoặc bí mật đều thay bằng `‹đã ẩn›`; SC-009 đo bằng tìm các chuỗi đó trong lưu lượng gửi đi.
- Q: Ảnh chụp, bản ghi, log lưu ở đâu để không bị đồng bộ lên máy chủ? → A: Mặc định thư mục cục bộ không roaming của người dùng; khi đổi thư mục, từ chối ổ mạng/UNC và thư mục nằm trong vùng đồng bộ đám mây đã biết; chỉ cho ổ cục bộ.
- Q: Bước không có điều kiện kết quả mà không tìm thấy phần tử thì có được nhờ model chọn thay (và tự vá) không? → A: Không; cấm cả chọn phần tử bằng model lẫn chẩn đoán ở bước đó, chuyển thẳng sang hỏi người dùng; bước được gắn cảnh báo "không kiểm chứng được" từ lúc biên dịch.
- Q: Khi chẩn đoán chọn "thử lại", có được lặp lại hành động có thể gây hậu quả nếu làm hai lần không? → A: Không ở bước có cờ phê duyệt: `retry` ở đó được coi là "hỏi người dùng"; bước khác được thử lại trong ngân sách sửa chữa.
- Q: Ảnh chụp (lúc ghi và lúc chạy) chụp phạm vi nào? → A: Chỉ vùng cửa sổ đích và các cửa sổ/hộp thoại cùng tiến trình đang hiện; phần có cửa sổ của tiến trình khác chồng lên được tô kín.
- Q: (analyze) Kết nối tới máy chủ model có bắt buộc mã hoá không? → A: Bắt buộc `https://` (trừ 127.0.0.1/localhost), luôn kiểm tra chứng chỉ, không có tuỳ chọn tắt.
- Q: (analyze) SC-001 đo thế nào khi bước được phép không có điều kiện kết quả? → A: Mỗi bước có điều kiện hoặc cảnh báo "không kiểm chứng được"; ≥ 80% bước có điều kiện tự sinh.
- Q: (analyze) Làm sao bắt buộc chạy lại bộ câu hỏi mẫu khi đổi prompt/bộ lọc/bộ phân tích? → A: Kết quả Kiểm tra model ghi dấu vân tay của các thành phần đó; test tự động fail nếu dấu vân tay hiện tại không có kết quả ≥ 90%.
- Q: (analyze) `label_step` có vào bộ câu hỏi mẫu không? → A: Có, ≥ 10 ca, chấm theo quy tắc (phân tích được, một dòng 1–120 ký tự, đúng ngôn ngữ, chứa tên phần tử đích, không chứa chuỗi bị cấm).

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Dạy một việc bằng cách làm mẫu (Priority: P1)

Người dùng bắt đầu ghi (qua giao diện ở spec 002, hoặc dòng lệnh), nhập mô tả kết quả mong muốn rồi tự làm workflow một lần trên các ứng dụng desktop thường dùng. Trong lúc ghi, người dùng tạm dừng/tiếp tục/dừng bằng phím tắt toàn cục (hoặc giao diện). Khi dừng, ứng dụng sinh ra một **skill nháp**: danh sách bước có thứ tự, mỗi bước có mô tả ý định dễ đọc, cách nhận ra phần tử cần thao tác, và điều kiện kiểm tra kết quả.

**Why this priority**: Không có skill thì không có gì để chạy, sửa hay lên lịch. Đây là lát cắt nhỏ nhất tạo ra giá trị thấy được.

**Independent Test**: Ghi một workflow 5 bước trên Notepad (mở, gõ chữ, lưu với tên file) → nhận được skill nháp 5 bước, mỗi bước có ý định, cách nhận diện phần tử và điều kiện kiểm tra; mở bản ghi không thấy ký tự đã gõ trong ô mật khẩu.

**Acceptance Scenarios**:

1. **Given** người dùng đã nhập mô tả kết quả, **When** họ bắt đầu ghi, làm workflow rồi dừng ghi, **Then** ứng dụng tạo skill nháp với các bước tương ứng các hành động có nghĩa (bấm, gõ, phím tắt, cuộn, chuyển cửa sổ), bỏ di chuột thuần tuý và bỏ thao tác trên chính cửa sổ của ứng dụng.
2. **Given** đang ghi, **When** người dùng gõ vào một ô mật khẩu, **Then** bản ghi chỉ lưu "có nhập dữ liệu bí mật", không lưu ký tự nào, và vùng ô đó bị che trong ảnh chụp.
3. **Given** đang ghi, **When** thời gian ghi đạt 10 phút, **Then** việc ghi tự dừng và skill nháp được sinh từ phần đã ghi.
4. **Given** không cấu hình model hoặc máy chủ model không trả lời, **When** skill được sinh, **Then** mỗi bước vẫn có ý định mô tả theo mẫu tất định (ví dụ: `Bấm "Lưu" (Button)`) và skill vẫn chạy được.
5. **Given** một bước được thao tác trên ứng dụng không cung cấp thông tin phần tử và không đọc được chữ quanh điểm bấm, **When** bước đó được ghi, **Then** hệ thống phát cảnh báo ngay lúc ghi rằng bước này chỉ dựa vào vị trí trên cửa sổ và dễ gãy (giao diện 002 hiển thị; dòng lệnh in ra), và bước được gắn cảnh báo đó.

---

### User Story 2 - Chạy skill an toàn, không cần model khi giao diện không đổi (Priority: P2)

Người dùng ra lệnh chạy một skill. Ứng dụng đưa đúng cửa sổ lên (mở ứng dụng nếu chưa chạy), tự tìm phần tử theo thông tin đã ghi, thực hiện hành động, chờ giao diện ứng dụng đích ổn định rồi kiểm tra điều kiện kết quả của từng bước. Hệ thống phát trạng thái trực tiếp (bước hiện tại, nấc đang dùng, log) để giao diện 002 hoặc dòng lệnh hiển thị. Người dùng luôn có thể dừng khẩn cấp; mọi thao tác chuột/phím của họ trong lúc chạy làm lần chạy tạm dừng. Bước được đánh dấu cần phê duyệt luôn chờ người dùng bấm Đồng ý. Khi bị kẹt, ứng dụng tạm dừng, chỉ ra bước đang kẹt để người dùng làm tay rồi bấm Tiếp tục, hoặc huỷ.

**Why this priority**: Đây là lời hứa cốt lõi — tự động hoá đáng tin trên máy thật. Phần tất định phải tự đứng vững trước khi thêm model.

**Independent Test**: Tắt hẳn kết nối model, chạy 10 skill chuẩn trên giao diện không đổi → cả 10 chạy xong, không lần nào gọi model; nhấn phím dừng khẩn cấp giữa chừng → mọi thao tác dừng trong vòng 1 giây.

**Acceptance Scenarios**:

1. **Given** giao diện ứng dụng đích không đổi so với lúc ghi, **When** chạy skill, **Then** mọi bước hoàn tất mà không có lần gọi model nào, và log ghi rõ điều đó.
2. **Given** một bước có điều kiện bỏ qua đang thoả, **When** chạy tới bước đó, **Then** bước bị bỏ qua và log ghi lý do.
3. **Given** bước có đánh dấu cần phê duyệt, **When** chạy tới bước đó, **Then** lần chạy dừng hẳn cho tới khi người dùng bấm Đồng ý (chạy tiếp) hoặc Từ chối (dừng lần chạy).
4. **Given** đang chạy, **When** người dùng nhấn phím dừng khẩn cấp, **Then** mọi thao tác tự động dừng trong vòng 1 giây.
5. **Given** đang chạy, **When** người dùng di chuột hoặc gõ phím, **Then** lần chạy tạm dừng và chờ người dùng bấm Tiếp tục.
6. **Given** điều kiện kết quả của một bước không đạt và không còn cách tự sửa, **When** hết ngân sách sửa chữa của bước (mặc định 2 lần), **Then** lần chạy tạm dừng, hệ thống cung cấp bước đang kẹt kèm ảnh và lý do, người dùng có thể làm tay rồi ra lệnh Tiếp tục hoặc huỷ.
7. **Given** skill có tham số đầu vào, **When** ra lệnh chạy mà thiếu giá trị tham số, **Then** lần chạy không bắt đầu và hệ thống báo tham số còn thiếu (kèm giá trị ví dụ).
8. **Given** một lần chạy kết thúc (thành công, thất bại hay huỷ), **When** xem log của lần chạy, **Then** có log đầy đủ: từng bước, cách giải quyết đã dùng, câu hỏi gửi model và câu trả lời (nếu có), ảnh chụp.

---

### User Story 3 - Hoàn thiện skill và đưa vào trạng thái sẵn sàng (Priority: P3)

Người dùng hoàn thiện skill nháp (qua màn hình Sửa ở spec 002, hoặc sửa file skill). Họ đánh dấu đoạn văn bản nào là tham số đầu vào (ví dụ "tuần cần xuất"), bước nào cần phê duyệt, thêm điều kiện bỏ qua, xoá hoặc sắp xếp lại bước, sửa điều kiện kết quả quá lỏng/quá chặt, và khai báo chỗ dùng thông tin bí mật. Sau một lần chạy thử thành công được người dùng xác nhận, skill chuyển từ "nháp" sang "sẵn sàng".

**Why this priority**: Một lần làm mẫu không thể hiện được tham số, ranh giới phê duyệt hay điều kiện bỏ qua; người dùng phải bổ sung. Skill "sẵn sàng" là điều kiện để lên lịch.

**Independent Test**: Lấy skill nháp "lưu file Notepad", đánh dấu tên file là tham số, đánh dấu bước Lưu cần phê duyệt, chạy thử với giá trị mới, xác nhận → skill chuyển sang "sẵn sàng" và lần chạy sau dùng đúng giá trị tham số mới.

**Acceptance Scenarios**:

1. **Given** một bước gõ văn bản, **When** người dùng chọn một đoạn và đánh dấu là tham số có tên, mô tả, ví dụ, **Then** skill có thêm tham số đầu vào đó và bước dùng giá trị tham số khi chạy.
2. **Given** người dùng khai báo một bước nhập thông tin bí mật bằng tên khoá, **When** chạy, **Then** giá trị được lấy từ kho bí mật của hệ điều hành và không xuất hiện trong skill, log, ảnh hay bất kỳ câu hỏi nào gửi model.
3. **Given** skill đang "nháp", **When** một lần chạy thử thành công và người dùng bấm xác nhận, **Then** skill chuyển sang "sẵn sàng".
4. **Given** skill "sẵn sàng", **When** người dùng sửa các bước, **Then** skill quay về "nháp" cho tới khi được chạy thử và xác nhận lại.
5. **Given** người dùng xoá hoặc đổi thứ tự bước, **When** lưu, **Then** skill được lưu với thứ tự mới và vẫn mở/chạy được.

---

### User Story 4 - Tự sửa chữa khi giao diện thay đổi, bằng model chỉ văn bản (Priority: P4)

Khi cách nhận diện đã ghi không tìm thấy (hoặc tìm thấy nhiều) phần tử, ứng dụng lập danh sách tối đa 40 phần tử ứng viên đã lọc, đánh số, mô tả bằng văn bản (tên, vai trò, trạng thái), và hỏi model một câu trắc nghiệm: chọn số nào, hoặc "không có". Khi điều kiện kết quả không đạt, ứng dụng mô tả tình trạng màn hình bằng văn bản và hỏi model chọn một trong bốn cách: chờ thêm, thử lại, đóng hộp thoại (số thứ tự), hoặc hỏi người dùng. Mọi lựa chọn của model đều được kiểm tra lại bằng điều kiện kết quả. Khi sửa chữa thành công, cách nhận diện mới được bổ sung vào bước (tự vá) và ghi log.

**Why this priority**: Giao diện đổi là điểm yếu đã biết của tự động hoá ghi-phát. Đây là giá trị gia tăng sau khi phần tất định đã chạy được.

**Independent Test**: Cố ý làm hỏng cách nhận diện chính của 10 skill chuẩn (xoá mã định danh, đổi tên) → ít nhất 8/10 chạy xong; chèn một hộp thoại bất ngờ giữa chừng của 10 skill → ít nhất 8/10 chạy xong; không lần nào thao tác sai phần tử mà không bị điều kiện kết quả phát hiện.

**Acceptance Scenarios**:

1. **Given** phần tử đích đã đổi mã định danh nhưng vẫn còn trên màn hình, **When** chạy tới bước đó, **Then** model được hỏi chọn trong danh sách ứng viên, phần tử được chọn được thao tác, điều kiện kết quả đạt, và cách nhận diện mới được lưu vào bước; skill vẫn "sẵn sàng", và cách nhận diện mới nằm trong danh sách "đã tự vá" chờ người dùng Giữ hoặc Hoàn tác.
2. **Given** model trả lời sai định dạng, **When** ứng dụng phân tích câu trả lời, **Then** ứng dụng hỏi lại đúng một lần kèm thông báo lỗi; nếu vẫn sai thì coi như "không có"/"hỏi người dùng".
3. **Given** model là loại có phần suy nghĩ trước câu trả lời, **When** ứng dụng nhận câu trả lời, **Then** chỉ phần lựa chọn được dùng, phần suy nghĩ và chữ thừa bị bỏ qua.
4. **Given** một hộp thoại bất ngờ che bước tiếp theo, **When** điều kiện kết quả không đạt, **Then** model được hỏi chẩn đoán, có thể chọn đóng đúng hộp thoại đó, rồi bước được thử lại.
5. **Given** mất kết nối tới máy chủ model, hết thời gian chờ hoặc sai khoá truy cập, **When** cần sửa chữa, **Then** ứng dụng chuyển thẳng sang hỏi người dùng kèm nguyên nhân rõ ràng (giao diện 002 / dòng lệnh hiển thị).
6. **Given** model chọn một phần tử sai, **When** thao tác xong, **Then** điều kiện kết quả phát hiện sai và lần chạy không tiếp tục như thể đã thành công.

---

### User Story 5 - Cấu hình và kiểm tra model của công ty (Priority: P5)

Người dùng cấu hình địa chỉ máy chủ, khoá truy cập, tên model (màn hình Cài đặt ở spec 002, hoặc dòng lệnh). Chức năng "Kiểm tra kết nối" xác nhận gọi được máy chủ. Chức năng "Kiểm tra model" chạy bộ câu hỏi mẫu đã lưu kèm đáp án đúng (không cần mở ứng dụng nào) và báo tỉ lệ trả lời đúng. Hệ thống không gọi model khi người dùng chưa xác nhận thông báo về dữ liệu được gửi đi.

**Why this priority**: Cần cho User Story 4 và để chọn giữa các model công ty đang host (DeepSeek, GLM), nhưng skill vẫn chạy được khi chưa cấu hình.

**Independent Test**: Cấu hình ba tham số của một model, chạy Kiểm tra model → nhận tỉ lệ đúng trên bộ câu hỏi mẫu; đổi sang model khác, chạy lại → nhận tỉ lệ của model đó để so sánh.

**Acceptance Scenarios**:

1. **Given** ba tham số hợp lệ, **When** chạy Kiểm tra kết nối, **Then** ứng dụng báo thành công; sai tham số thì báo rõ lỗi (không kết nối được / sai khoá / không có model đó).
2. **Given** kết nối được, **When** chạy Kiểm tra model, **Then** ứng dụng chạy bộ câu hỏi mẫu và báo tỉ lệ đúng cho từng loại câu hỏi (chọn phần tử, chẩn đoán).
3. **Given** lần đầu cấu hình model, **When** người dùng lưu, **Then** cấu hình chỉ có hiệu lực sau khi người dùng xác nhận thông báo (giao diện 002 / dòng lệnh hiển thị nội dung): chỉ văn bản (ý định bước, tiêu đề cửa sổ, tên và vai trò phần tử) được gửi đi; tên phần tử có thể chứa dữ liệu nghiệp vụ; ảnh chụp, bí mật, nội dung bộ nhớ tạm và nội dung đã gõ không bao giờ được gửi.
4. **Given** khoá truy cập đã lưu, **When** kiểm tra file cấu hình và log, **Then** không tìm thấy khoá ở đâu ngoài kho bí mật của hệ điều hành.

---

### User Story 6 - Lên lịch như lời nhắc có sẵn nút chạy (Priority: P6)

Người dùng tạo lịch cho một skill "sẵn sàng": chọn giá trị đầu vào và thời gian (biểu thức cron; giao diện 002 cung cấp mẫu hằng ngày, hằng tuần, mỗi N phút). Ứng dụng chạy nền (khay hệ thống và khởi động cùng Windows thuộc spec 002). Đến giờ, ứng dụng **luôn hỏi trước** — kể cả khi máy đang rảnh — với ba lựa chọn: Chạy ngay, Hoãn N phút, Bỏ qua lần này. Không bao giờ tự chạy khi chưa được đồng ý.

**Why this priority**: Giá trị cộng thêm sau khi chạy tay đã đáng tin. Lịch là lời nhắc, không phải tự động hoá không người trông.

**Independent Test**: Tạo lịch mỗi 5 phút cho một skill sẵn sàng; để máy rảnh → hệ thống phát yêu cầu "hỏi lịch" và không chạy cho tới khi nhận Chạy ngay; khoá máy qua giờ hẹn → chỉ sau khi mở khoá mới phát yêu cầu; để quá thời hạn chờ → lịch sử ghi "bỏ lỡ". Kiểm chứng được bằng mô phỏng thời gian và trạng thái khoá máy, không cần giao diện.

**Acceptance Scenarios**:

1. **Given** skill đang "nháp", **When** người dùng cố tạo lịch, **Then** ứng dụng không cho và giải thích cần chạy thử thành công trước.
2. **Given** đến giờ và người dùng đang dùng máy, **When** lịch kích hoạt, **Then** yêu cầu "hỏi lịch" ba lựa chọn được phát; không trả lời trong thời hạn thì hỏi lại khi máy rảnh; không bao giờ chạy đè lên người dùng.
3. **Given** đến giờ và máy rảnh, đang mở khoá, **When** lịch kích hoạt, **Then** yêu cầu "hỏi lịch" được phát, và lần chạy chờ người dùng đồng ý.
4. **Given** máy đang khoá lúc đến giờ, **When** người dùng mở khoá (trong thời hạn chờ), **Then** yêu cầu "hỏi lịch" được phát.
5. **Given** hết thời hạn chờ của lịch mà không ai đồng ý, **When** thời hạn qua, **Then** lần chạy được ghi là "bỏ lỡ" trong lịch sử.
6. **Given** lần chạy theo lịch gặp bước cần phê duyệt hoặc bị kẹt, **When** việc đó xảy ra, **Then** ứng dụng gửi thông báo Windows và chờ có thời hạn; hết hạn thì đánh dấu thất bại. Không có chế độ tự phê duyệt.
7. **Given** một skill đang chạy, **When** lịch của skill khác đến giờ, **Then** lịch đó chờ (không chạy song song) và chỉ hỏi sau khi lần chạy hiện tại kết thúc.
8. **Given** bất kỳ lần hoãn, bỏ qua, bỏ lỡ, thất bại hay thành công nào, **When** xem lịch sử lịch, **Then** thấy đủ trong lịch sử cùng lần chạy kế tiếp.

---

### Edge Cases

- Ứng dụng đích không cung cấp thông tin phần tử và chữ quá nhỏ/khó đọc (game, canvas): bước chỉ còn vị trí tương đối trên cửa sổ, không tự vá được; cảnh báo ngay lúc ghi và lúc sửa.
- Cửa sổ đích có kích thước/vị trí khác lúc ghi, hoặc màn hình có tỉ lệ hiển thị khác: vị trí tương đối được tính theo cửa sổ hiện tại.
- Ứng dụng đích chưa mở khi chạy: ứng dụng mở nó rồi mới tiếp tục.
- Có nhiều cửa sổ cùng ứng dụng: chọn theo tiêu đề đã ghi; không phân biệt được thì coi như "khớp nhiều" và đi theo nhánh sửa chữa.
- Ngân sách tổng của lần chạy (thời gian, số lần gọi model) cạn giữa chừng: lần chạy dừng ở trạng thái thất bại, log ghi lý do.
- Người dùng dừng ghi ngay sau khi bắt đầu (0 hành động): không tạo skill, thông báo không có bước nào.
- Ngắt ghi bằng Tạm dừng: các thao tác trong lúc tạm dừng không được ghi.
- Model trả về số thứ tự ngoài danh sách hoặc giá trị ngoài các lựa chọn cho phép: coi là sai định dạng (hỏi lại một lần rồi chuyển sang hỏi người dùng).
- Bí mật được tham chiếu nhưng không có trong kho bí mật: lần chạy dừng trước bước đó và yêu cầu người dùng bổ sung, không gửi gì ra ngoài.
- Máy đã đăng xuất (không chỉ khoá) hoặc ứng dụng không chạy lúc đến giờ: không có gì được chạy. Khi ứng dụng khởi động lại, mỗi lịch xét lần đến hạn gần nhất đã qua: còn trong thời hạn chờ → hỏi như bình thường; đã quá thời hạn → ghi "bỏ lỡ"; các lần cũ hơn chỉ ghi "bỏ lỡ", không bao giờ chạy bù hàng loạt (giao diện 002 nói rõ giới hạn này).
- Mất mạng công ty / ngoài VPN: skill có giao diện không đổi vẫn chạy bình thường; bước cần sửa chữa chuyển thẳng sang hỏi người dùng.
- Ứng dụng tự thoát hoặc máy khởi động lại giữa lần chạy: lần chạy được ghi là bị gián đoạn khi ứng dụng mở lại, không tự chạy tiếp.
- Một cửa sổ khác (thông báo, chat, hộp thoại của ứng dụng khác) cướp focus giữa bước: không thao tác nào được gửi vào cửa sổ đó (FR-049); bước nhập bí mật thì tạm dừng hỏi người dùng.
- Máy bị khoá hoặc UAC hiện lên giữa lần chạy: lần chạy tạm dừng ngay (FR-022); hết hạn chót thì thất bại theo FR-050.
- Lần chạy từ dòng lệnh trong script không có người trả lời gặp bước phê duyệt: lần chạy thất bại với lý do `no_responder` (FR-050).

## Requirements *(mandatory)*

### Functional Requirements

**Ghi (dạy việc)**

- **FR-001**: Hệ thống MUST cho bắt đầu ghi kèm mô tả kết quả mong muốn (bắt buộc), tạm dừng/tiếp tục và dừng ghi; tạm dừng/dừng MUST làm được bằng phím tắt toàn cục. Thanh ghi nổi → 002 (UI-020).
- **FR-002**: Hệ thống MUST ghi các loại hành động: bấm chuột, cuộn, gõ văn bản (gộp phím liên tiếp thành một đoạn), phím tắt, chuyển cửa sổ, thao tác bộ nhớ tạm; MUST bỏ di chuột thuần tuý và các thao tác trên mọi cửa sổ của chính ứng dụng (kể cả giao diện 002).
- **FR-003**: Với mỗi hành động, hệ thống MUST lưu: thời điểm, cửa sổ (tiêu đề, tiến trình, kích thước), thông tin phần tử được thao tác (tên, vai trò, mã định danh, đường dẫn trong cây giao diện, vị trí) nếu có, ảnh chụp trước/sau, và mô tả trạng thái dạng văn bản sau hành động (tiêu đề cửa sổ đang focus, các phần tử mới xuất hiện). Mọi ảnh chụp (lúc ghi, lúc chạy, ảnh "kẹt") MUST chỉ bao gồm vùng cửa sổ đích và các cửa sổ/hộp thoại thuộc cùng tiến trình đang hiện; phần bị cửa sổ của tiến trình khác chồng lên MUST được tô kín trước khi ghi xuống đĩa; không chụp toàn màn hình.
- **FR-004**: Khi phần tử dưới con trỏ không có thông tin hữu ích, hệ thống MUST nhận dạng chữ quanh điểm bấm và lưu chữ gần nhất làm thông tin nhận diện dự phòng.
- **FR-005**: Hệ thống MUST tự dừng ghi khi đạt 10 phút.
- **FR-006**: Hệ thống MUST NOT ghi ký tự gõ vào ô được hệ điều hành đánh dấu là ô mật khẩu; thay vào đó ghi sự kiện "nhập bí mật" và che vùng ô đó trong ảnh chụp.
- **FR-007**: Hệ thống MUST NOT ghi âm; bản ghi, ảnh chụp và log MUST chỉ lưu trên ổ cục bộ của máy người dùng. Thư mục dữ liệu mặc định MUST là thư mục ứng dụng cục bộ, không roaming, của người dùng (không phải thư mục roaming của hồ sơ).
- **FR-008**: Hệ thống MUST phát cảnh báo ngay lúc ghi khi một bước chỉ có thể nhận diện bằng vị trí tương đối trên cửa sổ, và gắn cảnh báo đó vào bước.

**Biên dịch bản ghi thành skill**

- **FR-009**: Hệ thống MUST biến bản ghi thành skill gồm danh sách bước có thứ tự; mỗi bước có: ý định (một câu ngắn), loại hành động (bấm, gõ, phím tắt, cuộn, chờ, mở ứng dụng), cửa sổ đích, danh sách cách nhận diện phần tử theo thứ tự ưu tiên (mã định danh → vai trò + tên → chữ nhận dạng được → vị trí tương đối trên cửa sổ), điều kiện kết quả, cờ cần phê duyệt, điều kiện bỏ qua (tuỳ chọn).
- **FR-010**: Điều kiện kết quả MUST được suy ra tất định từ mô tả trạng thái dạng văn bản đã ghi (ví dụ: tiêu đề cửa sổ chứa X, phần tử Y tồn tại, giá trị ô bằng Z).
- **FR-011**: Hệ thống MUST dùng model để viết câu ý định cho từng bước, mỗi lần chỉ đưa bước đó cùng bước liền trước và liền sau; với bước gõ chữ MUST chỉ gửi độ dài và ô đích ("gõ N ký tự vào ô <tên ô>"), không gửi nội dung đã gõ; khi model lỗi hoặc chưa cấu hình MUST dùng câu mô tả tất định theo mẫu.
- **FR-012**: Skill mới sinh MUST ở trạng thái "nháp" và MUST chạy được mà không cần model.
- **FR-013**: Skill MUST được lưu dưới dạng file văn bản người đọc được trên máy người dùng, không cần cơ sở dữ liệu.

**Hoàn thiện skill**

- **FR-014**: Hệ thống MUST cho sửa ý định, điều kiện kết quả, điều kiện bỏ qua, cờ phê duyệt, xoá và sắp xếp lại bước, và MUST kiểm tra hợp lệ toàn bộ skill trước khi lưu (không lưu skill không hợp lệ, trả về lỗi theo từng trường). Xoá skill MUST xoá cả ảnh, log lần chạy và lịch của skill đó; bí mật trong kho của hệ điều hành được giữ lại (có thể dùng chung giữa các skill) và người dùng xoá riêng. Màn hình Sửa → 002 (UI-040).
- **FR-015**: Người dùng MUST đánh dấu được một đoạn văn bản trong bước gõ là tham số đầu vào (tên, mô tả, giá trị ví dụ). Hệ thống MUST NOT tự đoán tham số.
- **FR-016**: Người dùng MUST khai báo được bước nhập thông tin bí mật bằng tên khoá; giá trị MUST được lấy từ kho bí mật của hệ điều hành lúc chạy và MUST NOT xuất hiện trong skill, log, ảnh hay câu hỏi gửi model.
- **FR-017**: Skill MUST chuyển từ "nháp" sang "sẵn sàng" chỉ sau một lần chạy thành công được người dùng xác nhận. Người dùng sửa các bước của skill "sẵn sàng" MUST đưa nó về "nháp"; tự vá (FR-027) và hoàn tác tự vá không tính là sửa.

**Chạy skill**

- **FR-018**: Với mỗi bước, hệ thống MUST xử lý theo bậc thang và dừng ở nấc đầu tiên thành công: (0) kiểm tra điều kiện bỏ qua và đưa đúng cửa sổ lên trước, mở ứng dụng nếu chưa chạy; (1) thử lần lượt các cách nhận diện đã lưu — khớp đúng một phần tử thì thao tác mà không gọi model; (2) nếu không khớp hoặc khớp nhiều, hỏi model chọn phần tử; (3) thực hiện hành động, chờ giao diện ổn định, kiểm tra điều kiện kết quả; (4) nếu điều kiện không đạt, hỏi model chẩn đoán; (5) hết ngân sách sửa chữa của bước (mặc định 2 lần) thì tạm dừng chờ người dùng.
- **FR-019**: Khi giao diện không đổi so với lúc ghi, một lần chạy MUST hoàn tất mà không gọi model lần nào.
- **FR-020**: Việc xác định một bước đã xong MUST chỉ dựa trên điều kiện kết quả kiểm tra tất định; model MUST NOT được tuyên bố bước hay lần chạy đã xong. Bước không có điều kiện kết quả nào MUST NOT dùng nấc 2 (chọn phần tử bằng model), nấc 4 (chẩn đoán) hay tự vá: khi bộ chọn không khớp đúng một phần tử, hệ thống chuyển thẳng sang nấc 5 (hỏi người dùng). Bước loại này MUST được gắn cảnh báo "không kiểm chứng được" từ lúc biên dịch và giữ cảnh báo cho tới khi có điều kiện kết quả.
- **FR-021**: Bước có cờ cần phê duyệt MUST luôn dừng chờ người dùng bấm Đồng ý hoặc Từ chối, kể cả khi chạy theo lịch. Không có chế độ tự phê duyệt.
- **FR-022**: Phím tắt dừng khẩn cấp MUST luôn hoạt động và dừng mọi thao tác tự động trong vòng 1 giây; khi dừng, mọi phím và nút chuột do hệ thống đang giữ MUST được nhả và không còn thao tác nào xếp hàng. Thao tác chuột/phím của người dùng trong lúc chạy MUST làm lần chạy tạm dừng. Phiên Windows bị khoá hoặc màn hình bảo mật (UAC) xuất hiện giữa lần chạy MUST làm lần chạy tạm dừng ngay, không gửi thêm thao tác nào.
- **FR-049**: Ngay trước **mỗi** lần gửi chuột/phím (không chỉ đầu bước), hệ thống MUST kiểm tra lại cửa sổ đang focus vẫn khớp cửa sổ đích của bước; với bước nhập bí mật, MUST kiểm tra thêm phần tử đang focus chính là phần tử đích. Không khớp → không gửi, xử lý như bước chưa đạt (đi tiếp bậc thang); với bước nhập bí mật → tạm dừng hỏi người dùng, không bao giờ gửi bí mật vào cửa sổ khác. (Constitution IV, V)
- **FR-050**: Khi một yêu cầu người dùng (phê duyệt, kẹt, chạm, thiếu bí mật, hỏi lịch) không thể được trả lời — chạy dòng lệnh không có người tương tác, đầu vào bị đóng, hay hết hạn chót — hệ thống MUST xử lý theo hướng an toàn: lần chạy kết thúc ở trạng thái thất bại với lý do `no_responder` (không có người trả lời được) hoặc `approval_timeout` / `stuck_timeout` (hết hạn chót); yêu cầu hỏi lịch → không chạy (hỏi lại hoặc bỏ lỡ theo FR-042/FR-043); không bao giờ coi là đồng ý. (Constitution IV)
- **FR-023**: Mỗi lần chạy MUST có giới hạn tổng thời gian và giới hạn tổng số lần gọi model; vượt giới hạn thì dừng ở trạng thái thất bại.
- **FR-024**: Khi tạm dừng (ở nấc 5, chờ phê duyệt, do người dùng chạm, hoặc thiếu bí mật), hệ thống MUST phát một **yêu cầu người dùng** gồm lý do, bước (ý định, ảnh lúc ghi, ảnh hiện tại) và các lựa chọn hợp lệ, và chỉ đi tiếp khi nhận một lựa chọn hợp lệ (Tiếp tục / Huỷ / Đồng ý / Từ chối). Cách hiển thị → 002 (UI-032).
- **FR-025**: Hệ thống MUST NOT chạy hai skill cùng lúc.
- **FR-026**: Mỗi lần chạy MUST lưu log trên máy: từng bước, nấc đã dùng, câu hỏi gửi model và câu trả lời, ảnh chụp, kết quả cuối. Hệ thống MUST giữ log (kể cả ảnh) của 50 lần chạy gần nhất cho mỗi skill và tự xoá log cũ hơn; người dùng chỉnh được con số này trong Cài đặt. Việc dọn MUST NOT xoá skill, lịch, lịch sử lịch hay log của lần chạy đang diễn ra.
- **FR-027**: Khi sửa chữa bằng model thành công và điều kiện kết quả đạt, hệ thống MUST bổ sung cách nhận diện mới vào bước đó (giữ các cách cũ) và ghi log việc tự vá. Tự vá MUST NOT đổi trạng thái skill. Hệ thống MUST giữ danh sách các cách nhận diện tự vá đang chờ duyệt (bước, cách nhận diện mới, lần chạy nào) cho tới khi người dùng Giữ hoặc Hoàn tác từng cái; Hoàn tác xoá đúng cách nhận diện đó. Hiển thị danh sách → 002 (UI-034).

**Hợp đồng với model**

- **FR-028**: Model MUST chỉ được hỏi ba loại câu, mỗi loại có câu hỏi cố định kèm 2–3 ví dụ mẫu có lời giải: chọn phần tử (trả về một số thứ tự trong danh sách tối đa 40 ứng viên, hoặc "không có"), chẩn đoán (trả về đúng một trong: chờ, thử lại, đóng hộp thoại số N, hỏi người dùng), viết ý định bước (một câu ngắn, chỉ dùng lúc biên dịch). Ở bước có cờ phê duyệt, lựa chọn "thử lại" MUST được xử lý như "hỏi người dùng" (không bao giờ tự lặp lại hành động đã được phê duyệt); ở bước khác, "thử lại" chỉ được dùng trong ngân sách sửa chữa của bước.
- **FR-029**: Model MUST NOT lập kế hoạch, sinh toạ độ, hay viết văn bản tự do lúc chạy; ngoài đóng hộp thoại, model không thể khiến hệ thống thao tác lên thứ gì nằm ngoài bước đang chạy.
- **FR-030**: Mọi thông tin gửi model MUST là văn bản; ngữ cảnh mỗi lần gọi chỉ gồm bước hiện tại và danh sách ứng viên, dưới khoảng 2.000 token; không gửi lịch sử hay toàn bộ skill.
- **FR-031**: Hệ thống MUST tự phân tích câu trả lời: bỏ phần suy nghĩ và chữ thừa, lấy lựa chọn đầu tiên, kiểm tra nằm trong tập cho phép; sai thì hỏi lại đúng một lần kèm thông báo lỗi, vẫn sai thì coi là "không có"/"hỏi người dùng". Hệ thống MUST NOT phụ thuộc vào việc máy chủ hỗ trợ đầu ra có cấu trúc, gọi công cụ hay đầu vào ảnh (được phép dùng thêm nếu có, nhưng bước tự phân tích vẫn giữ).
- **FR-032**: Lời gọi model MUST dùng chế độ trả lời tất định nhất máy chủ cho phép; quá thời gian, lỗi kết nối, lỗi xác thực MUST được xử lý như "hỏi người dùng" và nguyên nhân MUST có trong yêu cầu người dùng (FR-024) và log.
- **FR-033**: Ảnh chụp, giá trị bí mật, nội dung bộ nhớ tạm và nội dung văn bản người dùng đã gõ (kể cả giá trị tham số) MUST NOT được gửi tới máy chủ model. Trước mỗi lần gửi, hệ thống MUST lọc toàn bộ văn bản sẽ gửi (tiêu đề cửa sổ, tên phần tử, ý định): mọi đoạn trùng — không phân biệt hoa thường — với văn bản đã gõ trong bản ghi/lần chạy, giá trị tham số của lần chạy hoặc giá trị bí mật được dùng trong lần chạy MUST được thay bằng `‹đã ẩn›`.
- **FR-034**: Ứng dụng MUST dùng một model tại một thời điểm, chọn bằng tên model trong Cài đặt.

**Cài đặt**

- **FR-035**: Người dùng MUST cấu hình model bằng đúng ba tham số: địa chỉ máy chủ, khoá truy cập, tên model. Địa chỉ máy chủ MUST dùng kết nối mã hoá (`https://`), trừ địa chỉ vòng lặp cục bộ (`127.0.0.1`, `localhost`) dùng cho kiểm thử; chứng chỉ máy chủ MUST luôn được kiểm tra theo kho chứng chỉ của hệ điều hành (nhận CA nội bộ do IT cài) và không có tuỳ chọn tắt kiểm tra. Khoá truy cập MUST chỉ lưu trong kho bí mật của hệ điều hành, không trong file cấu hình hay log.
- **FR-036**: Hệ thống MUST có chức năng Kiểm tra kết nối và Kiểm tra model; Kiểm tra model chạy bộ câu hỏi mẫu có đáp án cho **cả ba** loại câu hỏi (chọn phần tử, chẩn đoán, viết ý định — loại cuối chấm theo quy tắc: phân tích được, một dòng 1–120 ký tự, đúng ngôn ngữ yêu cầu, chứa tên phần tử đích, không chứa chuỗi bị cấm) không cần giao diện ứng dụng đích và báo tỉ lệ đúng theo loại và tổng. Mỗi kết quả Kiểm tra model MUST ghi kèm dấu vân tay của các thành phần quyết định câu hỏi và cách đọc câu trả lời (prompt, bộ lọc ứng viên, bộ phân tích); bản phát hành chỉ hợp lệ khi có kết quả ≥ 90% trùng dấu vân tay hiện tại (Constitution VI).
- **FR-037**: Hệ thống MUST NOT gọi model khi người dùng chưa xác nhận thông báo về dữ liệu được gửi đi (FR-030, FR-033) và việc tên phần tử có thể chứa dữ liệu nghiệp vụ; nội dung thông báo do lõi cung cấp, cách hiển thị → 002 (UI-051). Đổi địa chỉ máy chủ hoặc tên model MUST đặt lại trạng thái đã xác nhận, phải xác nhận lại trước lần gọi kế tiếp.
- **FR-038**: Người dùng MUST cấu hình được phím tắt (dừng khẩn cấp, tạm dừng/dừng ghi) và thư mục dữ liệu. Khi đổi thư mục dữ liệu, hệ thống MUST từ chối ổ mạng/đường dẫn UNC, ổ ánh xạ mạng và thư mục nằm trong vùng đồng bộ đám mây đã biết (ví dụ thư mục OneDrive, thư mục cá nhân được chuyển hướng lên máy chủ), kèm lý do; chỉ chấp nhận thư mục trên ổ cục bộ.
- **FR-048**: Người dùng MUST chọn được ngôn ngữ ứng dụng trong Cài đặt: tiếng Việt hoặc tiếng Anh (mặc định theo ngôn ngữ Windows, không khớp thì tiếng Việt). Ở lõi, lựa chọn áp dụng cho câu ý định tất định, ngôn ngữ yêu cầu model viết câu ý định, và các thông điệp lõi trả về (lỗi, lý do tạm dừng); chữ trên giao diện và thông báo Windows → 002 (UI-004). Câu ý định đã có trong skill MUST NOT bị dịch lại tự động khi đổi ngôn ngữ.

**Lịch**

- **FR-039**: Hệ thống MUST cho tạo, sửa, xoá lịch gồm: skill, giá trị đầu vào, thời gian (biểu thức cron, giờ địa phương), thời hạn chờ; chỉ skill "sẵn sàng" được lên lịch; skill về "nháp" thì lịch của nó tạm ngưng (có lý do) cho tới khi skill sẵn sàng lại.
- **FR-040**: → 002 (UI-060, UI-061: khay hệ thống, khởi động cùng Windows).
- **FR-041**: Đến giờ, hệ thống MUST luôn phát yêu cầu người dùng loại "hỏi lịch" (Chạy ngay / Hoãn N phút / Bỏ qua lần này), bất kể máy đang được dùng hay rảnh, và MUST NOT chạy khi chưa nhận lựa chọn Chạy ngay cho lần đó. Hộp hỏi và thông báo Windows → 002 (UI-064).
- **FR-042**: Nếu người dùng đang dùng máy và không trả lời trong thời hạn hỏi, hệ thống MUST hỏi lại khi máy rảnh.
- **FR-043**: Nếu máy đang khoá lúc đến giờ, hệ thống MUST chờ tới khi mở khoá rồi hỏi; hết thời hạn chờ của lịch mà chưa được đồng ý thì MUST ghi là "bỏ lỡ".
- **FR-044**: Lần chạy theo lịch gặp bước cần phê duyệt hoặc bị kẹt MUST phát yêu cầu người dùng có hạn chót (mặc định 30 phút); hết hạn thì đánh dấu thất bại. Thông báo Windows → 002 (UI-065).
- **FR-045**: Hệ thống MUST cung cấp danh sách lịch, lần chạy kế tiếp của từng lịch, và lịch sử đầy đủ (chạy, hoãn, bỏ qua, bỏ lỡ, thất bại). Màn hình Lịch → 002 (UI-062).
- **FR-046**: → 002 (UI-062: nhắc lịch chỉ chạy khi Windows mở khoá).

**Bề mặt điều khiển**

- **FR-047**: Mọi thao tác người dùng của lõi (ghi, chạy, trả lời yêu cầu người dùng, sửa skill, duyệt tự vá, cài đặt, kiểm tra model, quản lý lịch) MUST làm được qua một bề mặt điều khiển duy nhất mà giao diện 002 dùng, và các thao tác cần cho nghiệm thu lõi (ghi, chạy, kiểm tra model, bộ đo) MUST làm được qua dòng lệnh không cần giao diện. Lõi MUST phát trạng thái và yêu cầu người dùng dưới dạng sự kiện để bên hiển thị theo dõi. Danh sách skill → 002 (UI-010).

### Key Entities

- **Bản ghi (Trace)**: kết quả một lần làm mẫu — mô tả kết quả mong muốn, chuỗi sự kiện có thời điểm, ảnh chụp, mô tả trạng thái dạng văn bản. Chỉ lưu trên máy.
- **Skill**: tên, mô tả, trạng thái (nháp / sẵn sàng), danh sách tham số đầu vào, danh sách bước. Sinh từ một bản ghi, người dùng sửa được.
- **Bước (Step)**: ý định, loại hành động, cửa sổ đích, danh sách cách nhận diện có thứ tự ưu tiên, văn bản cần gõ (có thể chứa tham số hoặc tham chiếu bí mật), điều kiện kết quả, cờ phê duyệt, điều kiện bỏ qua.
- **Tham số đầu vào (Input)**: tên, mô tả, giá trị ví dụ; giá trị cụ thể được nhập khi chạy hoặc lưu trong lịch.
- **Tham chiếu bí mật (Secret reference)**: tên khoá trỏ tới kho bí mật của hệ điều hành; skill chỉ giữ tên, không giữ giá trị.
- **Lần chạy (Run)**: skill, giá trị đầu vào, nguồn kích hoạt (tay / lịch), thời điểm, kết quả (thành công / thất bại / huỷ / gián đoạn), log từng bước kèm nấc đã dùng, câu hỏi/trả lời model, ảnh.
- **Lịch (Schedule)**: skill, giá trị đầu vào, biểu thức thời gian, thời hạn chờ, lần chạy kế tiếp, lịch sử các lần kích hoạt (chạy / hoãn / bỏ qua / bỏ lỡ / thất bại).
- **Cấu hình model (Model settings)**: địa chỉ máy chủ, tên model (lưu trong cấu hình), khoá truy cập (chỉ trong kho bí mật), trạng thái đã xác nhận thông báo dữ liệu.
- **Bộ câu hỏi mẫu (Offline eval set)**: các câu hỏi chọn phần tử / chẩn đoán đã lưu kèm đáp án đúng, dùng cho Kiểm tra model.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Ghi một workflow 5 bước cho ra skill nháp có đủ 5 bước, mỗi bước có ý định, ít nhất một cách nhận diện, và hoặc ít nhất một điều kiện kết quả hoặc cảnh báo "không kiểm chứng được" (FR-020); trên bộ workflow chuẩn, ≥ 80% số bước có điều kiện kết quả do hệ thống tự sinh; 0 ký tự gõ vào ô mật khẩu xuất hiện trong bản ghi.
- **SC-002**: Trên bộ 10 skill chuẩn (Notepad, trình quản lý file, máy tính, Cài đặt Windows, một trình duyệt, một ứng dụng văn phòng hoặc tương đương) với giao diện không đổi và model tắt hẳn: 10/10 skill chạy xong, 0 lần gọi model.
- **SC-003**: Dừng khẩn cấp làm ngừng mọi thao tác tự động trong vòng 1 giây, 100% số lần thử.
- **SC-004**: Với model được chọn làm mặc định: Kiểm tra model đạt ≥ 90% đúng trên bộ câu hỏi mẫu; khi cách nhận diện chính bị cố ý làm hỏng, ≥ 8/10 skill chuẩn vẫn chạy xong; khi chèn hộp thoại bất ngờ, ≥ 8/10 skill chuẩn vẫn chạy xong.
- **SC-005**: 0 trường hợp thao tác sai phần tử mà không bị điều kiện kết quả phát hiện, trên toàn bộ các lần chạy đo ở SC-004.
- **SC-006**: Mỗi model công ty đang host trả về câu trả lời phân tích được và hợp lệ ≥ 19/20 lần cho câu hỏi trắc nghiệm mẫu.
- **SC-007**: → 002 (SCU-001). Ở lõi: vòng dạy → chạy thử → phê duyệt → xác nhận sẵn sàng làm trọn được bằng dòng lệnh và phím tắt, không cần giao diện.
- **SC-008**: Trong mọi phép thử lịch (kể cả mô phỏng thời gian và khoá/mở khoá): 100% lần đến giờ đều phát yêu cầu "hỏi lịch" (kể cả khi máy rảnh), 0 lần chạy khi chưa nhận lựa chọn Chạy ngay; máy khoá thì chỉ hỏi sau khi mở khoá; hết thời hạn chờ thì 100% được ghi "bỏ lỡ". Phần hiển thị → 002 (SCU-003).
- **SC-009**: Kiểm tra toàn bộ file cấu hình, log, skill và lưu lượng gửi tới máy chủ model: 0 lần xuất hiện khoá truy cập, giá trị bí mật, ảnh chụp hay nội dung bộ nhớ tạm; tìm trong toàn bộ lưu lượng gửi tới máy chủ model các chuỗi văn bản đã gõ, giá trị tham số và giá trị bí mật của các lần chạy thử (kể cả khi chúng xuất hiện trong tiêu đề cửa sổ hoặc tên phần tử) cho 0 kết quả.
- **SC-010**: Khi mất kết nối máy chủ model, skill có giao diện không đổi vẫn chạy xong như bình thường; bước cần sửa chữa chuyển sang hỏi người dùng với nguyên nhân hiển thị rõ.

## Assumptions

- **Nền tảng**: Windows 10/11; ngoài phạm vi bản này: macOS/Linux, đầu vào ảnh cho model, model chạy trong tiến trình, vòng lặp/nhánh rẽ phức tạp (chỉ có điều kiện bỏ qua đơn giản), kích hoạt theo sự kiện, chia sẻ skill, ghi video, ghi âm.
- **Ứng dụng đích**: đa số ứng dụng desktop cung cấp thông tin phần tử qua cơ chế trợ năng của hệ điều hành; ứng dụng không có thì dùng nhận dạng chữ trên màn hình làm dự phòng. Chất lượng nhận dạng chữ tiếng Việt cần được đo trước khi xây dựng.
- **Máy chủ model**: giả định dùng giao thức chat completions tương thích OpenAI; khả năng đầu ra có cấu trúc, giới hạn ngữ cảnh, tốc độ, hạn mức, việc có trả phần suy nghĩ hay không — với cả DeepSeek và GLM — là ẩn số được trả lời bằng phép đo ở bước thử nghiệm kỹ thuật đầu tiên (M0), không cần hỏi thêm. Model mặc định được chọn theo kết quả đo SC-004.
- **Mạng**: máy chủ model nằm trong mạng nội bộ công ty (có thể cần VPN); rủi ro gửi văn bản giao diện tới đó thấp hơn dịch vụ bên ngoài nhưng vẫn phải thông báo cho người dùng.
- **Ngôn ngữ**: giao diện có tiếng Việt và tiếng Anh (FR-048); ngôn ngữ của prompt gửi model (khác với ngôn ngữ câu ý định model trả về) do plan chọn theo kết quả đo.
- **Người dùng**: một người dùng trên một máy; skill và lịch là của riêng người đó; không có phân quyền.
- **Lưu trữ**: toàn bộ dữ liệu là file trong thư mục dữ liệu của người dùng (mặc định thư mục ứng dụng cục bộ, không roaming — FR-007, FR-038); log lần chạy giữ 50 lần gần nhất mỗi skill (FR-026).
- **Giá trị mặc định đã chốt** (người dùng chỉnh được; M0 có thể đề xuất sửa nếu đo thấy không hợp, đặc biệt thời gian chờ gọi model với model suy luận):
  - "Đang dùng máy" = có thao tác chuột/phím trong 3 phút gần nhất.
  - Thời hạn chờ của lịch mặc định: 2 giờ; thời hạn chờ phê duyệt/kẹt khi chạy theo lịch: 30 phút; thời hạn hỏi khi người dùng đang dùng máy: 5 phút.
  - Ngân sách mỗi lần chạy: tổng 30 phút, tổng 20 lần gọi model; ngân sách sửa chữa mỗi bước: 2 lần.
  - Thời gian chờ mỗi lần gọi model: 30 giây.
- **Lộ trình**: thứ tự nghiệm thu theo bản nháp — M0 thử nghiệm kỹ thuật, M1 ghi + biên dịch (chưa có model), M2 chạy tất định, M3 gọi model + tự vá, M4 giao diện đầy đủ (**spec 002**), M5 lịch (quy tắc lịch ở spec này; khay, hộp hỏi, thông báo ở spec 002). Phần tất định (M2) phải đạt trước khi thêm model (M3).
- **Lựa chọn công nghệ** (ghi, thao tác, chụp màn hình, nhận dạng chữ, lịch) chốt ở [plan.md](./plan.md); giao diện dùng Tauri theo spec 002.
