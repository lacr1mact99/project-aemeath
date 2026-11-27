# Feature Specification: Giao diện desktop cho Teach a Task

**Feature Branch**: `002-tauri-gui`

**Created**: 2026-11-23

**Status**: Draft

**Input**: User description: "tách phần spec giao diện sang spec 002, UI bằng tauri"

## Bối cảnh và ranh giới

Spec [001-teach-a-task](../001-teach-a-task/spec.md) mô tả **lõi** của sản phẩm: ghi thao tác, biên dịch thành
skill, chạy theo bậc thang tất định, hỏi model, quy tắc lịch, lưu trữ, riêng tư. Spec này mô tả **giao diện
người dùng** đặt trên lõi đó: các màn hình, thanh ghi nổi, biểu tượng khay, hộp hỏi khi lịch đến giờ, thông
báo Windows, và cách người dùng phê duyệt / tiếp tục / dừng.

Nguyên tắc phân chia:

- **Lõi (001) quyết định, giao diện (002) hiển thị và chuyển lựa chọn của người dùng.** Mọi quy tắc (khi nào
  cần phê duyệt, khi nào được chạy theo lịch, skill nào được lên lịch, cái gì được gửi model) nằm ở 001.
  Giao diện MUST NOT tự đưa ra các quyết định đó và MUST NOT có đường tắt bỏ qua lõi.
- Mã yêu cầu `FR-xxx` / `SC-xxx` trỏ về spec 001; yêu cầu của spec này dùng tiền tố `UI-` và `SCU-`.
- Phím dừng khẩn cấp do lõi bắt và xử lý (hoạt động cả khi giao diện ẩn, treo hoặc chưa mở); giao diện chỉ
  có thêm nút Dừng.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Dạy một việc mới qua giao diện (Priority: P1)

Người dùng mở ứng dụng, thấy danh sách skill (rỗng ở lần đầu, có hướng dẫn ngắn), bấm **Dạy việc mới**, nhập
mô tả kết quả mong muốn, bấm Bắt đầu. Cửa sổ chính thu lại, thay bằng một **thanh nổi nhỏ luôn nằm trên
cùng** có đồng hồ thời gian đã ghi, nút Tạm dừng và Dừng, kèm phím tắt tương ứng. Khi một bước chỉ nhận diện
được bằng vị trí, thanh nổi báo ngay. Khi dừng, cửa sổ chính trở lại và mở skill nháp vừa sinh.

**Why this priority**: Là cửa vào của toàn bộ sản phẩm; không dạy được qua giao diện thì người dùng không
chuyên không dùng được.

**Independent Test**: Chỉ dùng chuột trên giao diện, dạy một workflow 5 bước trên Notepad → thấy skill nháp
5 bước mở ra trong màn hình Sửa; thanh nổi không bị ghi vào skill.

**Acceptance Scenarios**:

1. **Given** chưa có skill nào, **When** mở ứng dụng, **Then** màn hình chính hiện danh sách rỗng với nút Dạy việc mới và một câu hướng dẫn.
2. **Given** đã nhập mô tả, **When** bấm Bắt đầu, **Then** cửa sổ chính thu lại, thanh nổi xuất hiện trên cùng, đồng hồ chạy.
3. **Given** đang ghi, **When** bấm Tạm dừng (hoặc phím tắt), **Then** đồng hồ dừng, thanh nổi hiện trạng thái "Tạm dừng" rõ ràng; bấm lại thì tiếp tục.
4. **Given** đang ghi, **When** người dùng bấm vào thanh nổi, **Then** thao tác đó không trở thành bước trong skill.
5. **Given** một bước vừa ghi chỉ có nhận diện theo vị trí, **When** bước đó được ghi, **Then** thanh nổi hiện cảnh báo trong vài giây mà không chặn thao tác.
6. **Given** bấm Dừng (hoặc đạt 10 phút), **When** lõi sinh xong skill nháp, **Then** cửa sổ chính trở lại và mở màn hình Sửa của skill đó; nếu không có bước nào thì báo "không có bước nào được ghi" và không tạo skill.

---

### User Story 2 - Chạy, theo dõi và can thiệp (Priority: P1)

Từ danh sách, người dùng bấm **Chạy**. Nếu skill có tham số, một biểu mẫu hiện từng tham số với giá trị ví
dụ. Màn hình **Theo dõi** hiện bước hiện tại (ý định, ảnh), nấc đang dùng, log cập nhật trực tiếp, số lần đã
gọi model, và nút Dừng. Khi lõi tạm dừng vì cần phê duyệt, vì người dùng chạm chuột/phím, vì kẹt, hay vì thiếu
bí mật, màn hình hiện lý do và đúng các nút phù hợp. Kết thúc lần chạy, màn hình hiện kết quả, và nếu có bước
được tự vá thì hiện danh sách để người dùng Giữ hoặc Hoàn tác từng bước. Lần chạy thử của skill nháp thành
công thì hỏi "Skill chạy đúng chưa?" để xác nhận chuyển sang sẵn sàng.

**Why this priority**: Phê duyệt, tiếp tục và dừng là lớp an toàn mà con người dùng tới; phải rõ ràng như
lõi đòi hỏi (Nguyên tắc IV).

**Independent Test**: Chạy một skill có bước phê duyệt và một bước bị làm kẹt có chủ đích → giao diện hiện
đúng lý do tạm dừng, các nút Đồng ý/Từ chối và Tiếp tục/Huỷ hoạt động; kết quả cuối khớp log của lõi.

**Acceptance Scenarios**:

1. **Given** skill có tham số, **When** bấm Chạy, **Then** biểu mẫu nhập giá trị hiện ra; thiếu giá trị thì không cho bắt đầu.
2. **Given** đang chạy, **When** lõi chuyển sang bước mới, **Then** màn hình Theo dõi cập nhật trong vòng 1 giây.
3. **Given** lõi tạm dừng chờ phê duyệt, **When** màn hình hiện, **Then** có ý định của bước, ảnh lúc ghi, và hai nút Đồng ý / Từ chối; không có lựa chọn "tự đồng ý lần sau".
4. **Given** lõi tạm dừng vì bị kẹt, **When** màn hình hiện, **Then** có ý định, ảnh lúc ghi, ảnh hiện tại, lý do (kể cả "không kết nối được máy chủ model" / "sai khoá truy cập"), và nút Tiếp tục (sau khi người dùng làm tay) / Huỷ.
5. **Given** lõi tạm dừng vì người dùng chạm chuột/phím, **When** màn hình hiện, **Then** có nút Tiếp tục và Dừng.
6. **Given** lõi tạm dừng vì thiếu bí mật, **When** màn hình hiện, **Then** có ô nhập giá trị (ẩn ký tự) để lưu vào kho bí mật của hệ điều hành, rồi Tiếp tục.
7. **Given** lần chạy kết thúc với các bước đã tự vá, **When** màn hình kết quả hiện, **Then** mỗi bước có Giữ / Hoàn tác; trạng thái skill không đổi.
8. **Given** lần chạy thử skill nháp thành công, **When** người dùng bấm Xác nhận, **Then** skill hiện là "sẵn sàng" trong danh sách.

---

### User Story 3 - Sửa skill (Priority: P2)

Màn hình **Sửa** liệt kê các bước. Mỗi bước có ảnh trước/sau, ý định, danh sách cách nhận diện, điều kiện
kết quả, cảnh báo (chỉ có vị trí, chưa ghi được văn bản). Người dùng sửa ý định và điều kiện, bật/tắt phê
duyệt, thêm điều kiện bỏ qua, xoá và kéo-thả sắp xếp bước, bôi đen một đoạn văn bản trong bước gõ để biến
thành tham số, chèn tham chiếu bí mật. Lỗi hợp lệ do lõi báo hiện ngay cạnh trường sai.

**Why this priority**: Cần để có skill sẵn sàng và để lên lịch, nhưng skill nháp vẫn chạy được trước khi sửa.

**Independent Test**: Trên skill "lưu file Notepad": biến tên file thành tham số, bật phê duyệt bước Lưu, đổi
thứ tự hai bước rồi lưu → mở lại thấy đúng; skill đang sẵn sàng thì về nháp kèm thông báo lý do.

**Acceptance Scenarios**:

1. **Given** một bước gõ, **When** người dùng bôi đen một đoạn và chọn "Biến thành tham số", **Then** hiện ô nhập tên, mô tả, ví dụ; lưu xong đoạn đó hiển thị dạng tham số.
2. **Given** người dùng chèn bí mật, **When** nhập tên khoá, **Then** giao diện cho nhập giá trị một lần (ẩn ký tự) để lưu vào kho bí mật; giá trị không bao giờ được hiển thị lại.
3. **Given** một bước có cảnh báo, **When** xem danh sách bước, **Then** cảnh báo hiển thị nổi bật cạnh bước đó.
4. **Given** skill đang sẵn sàng, **When** người dùng lưu thay đổi bước, **Then** giao diện báo trước "skill sẽ về nháp và các lịch của nó tạm ngưng" và chỉ lưu khi người dùng đồng ý.
5. **Given** dữ liệu sửa không hợp lệ (ví dụ tham số chưa khai báo), **When** bấm Lưu, **Then** không lưu, lỗi hiện cạnh trường sai.

---

### User Story 4 - Cài đặt và kiểm tra model (Priority: P2)

Màn hình **Cài đặt** có: địa chỉ máy chủ, khoá truy cập, tên model; nút Kiểm tra kết nối và Kiểm tra model
(hiện tỉ lệ đúng theo từng loại câu hỏi); phím tắt; thư mục dữ liệu; số lần chạy giữ log; ngôn ngữ; khởi động
cùng Windows. Lần đầu lưu cấu hình model, một hộp thông báo dữ liệu phải được xác nhận.

**Why this priority**: Cần cho tự sửa chữa bằng model, nhưng skill vẫn chạy được khi chưa cấu hình.

**Independent Test**: Nhập ba tham số, xác nhận thông báo dữ liệu, bấm Kiểm tra model → thấy tỉ lệ đúng; đóng
mở lại Cài đặt → ô khoá truy cập chỉ hiện "đã lưu", không hiện giá trị.

**Acceptance Scenarios**:

1. **Given** lần đầu cấu hình model, **When** bấm Lưu, **Then** hộp thông báo dữ liệu hiện ra (cái gì được gửi, cái gì không bao giờ được gửi, tên phần tử có thể chứa dữ liệu nghiệp vụ); chỉ lưu khi bấm Đồng ý.
2. **Given** khoá truy cập đã lưu, **When** mở lại Cài đặt, **Then** ô khoá chỉ hiện trạng thái "đã lưu" và nút Thay; giao diện không đọc được giá trị khoá.
3. **Given** bấm Kiểm tra kết nối, **When** lõi trả kết quả, **Then** hiện thành công hoặc lý do lỗi dễ hiểu (không kết nối được / sai khoá / không có model đó); trong lúc chờ có chỉ báo đang chạy và giao diện không bị treo.
4. **Given** bấm Kiểm tra model, **When** xong, **Then** hiện tỉ lệ đúng theo từng loại và tổng, so với ngưỡng 90%.
5. **Given** đổi ngôn ngữ, **When** bấm Lưu, **Then** toàn bộ giao diện đổi ngôn ngữ ngay, không cần khởi động lại; câu ý định đã có trong skill giữ nguyên.
6. **Given** đặt phím tắt trùng với ứng dụng khác hoặc trùng nhau, **When** bấm Lưu, **Then** báo lỗi và giữ phím tắt cũ.

---

### User Story 5 - Lịch, khay hệ thống và hộp hỏi khi đến giờ (Priority: P3)

Ứng dụng chạy nền với biểu tượng ở khay hệ thống (đóng cửa sổ thì xuống khay, không thoát; thoát bằng menu
khay). Màn hình **Lịch** liệt kê lịch, lần chạy kế tiếp, lịch sử; tạo lịch bằng mẫu (hằng ngày, hằng tuần,
mỗi N phút) hoặc ô cron cho người cần. Khi lõi báo một lần kích hoạt cần hỏi, giao diện hiện **hộp hỏi** luôn
nằm trên cùng (Chạy ngay / Hoãn N phút / Bỏ qua lần này) kèm thông báo Windows. Khi lần chạy theo lịch cần phê
duyệt hoặc bị kẹt, thông báo Windows dẫn người dùng tới màn hình Theo dõi.

**Why this priority**: Phụ thuộc skill sẵn sàng và chạy tay đã ổn; là giá trị cộng thêm.

**Independent Test**: Tạo lịch mỗi 5 phút bằng mẫu → đóng cửa sổ (xuống khay) → đến giờ hộp hỏi và thông báo
xuất hiện; bấm Hoãn 10 phút → lịch sử có dòng hoãn; không bấm gì → lần chạy không bao giờ bắt đầu.

**Acceptance Scenarios**:

1. **Given** skill đang nháp, **When** mở form tạo lịch, **Then** skill đó không chọn được và có lý do hiển thị.
2. **Given** chọn mẫu "hằng tuần, thứ Hai 8:00", **When** lưu, **Then** lần chạy kế tiếp hiển thị đúng ngày giờ địa phương.
3. **Given** cửa sổ chính đã đóng xuống khay, **When** lõi báo cần hỏi, **Then** hộp hỏi xuất hiện trên cùng và có thông báo Windows; bấm thông báo cũng mở hộp hỏi.
4. **Given** hộp hỏi đang hiện, **When** người dùng chọn một nút, **Then** lựa chọn được chuyển cho lõi và hộp đóng; không có nút "luôn chạy không cần hỏi".
5. **Given** lõi báo thời hạn hỏi đã hết, **When** đó là lúc người dùng đang bận, **Then** hộp hỏi được ẩn đi và hiện lại khi lõi yêu cầu hỏi lại.
6. **Given** màn hình Lịch, **When** mở, **Then** có dòng nhắc rằng lịch chỉ chạy khi Windows đang mở khoá, và lịch sử hiện mọi lần chạy, hoãn, bỏ qua, bỏ lỡ, thất bại.
7. **Given** bật "Khởi động cùng Windows", **When** đăng nhập lại, **Then** ứng dụng khởi động thẳng xuống khay, không mở cửa sổ.
8. **Given** ứng dụng đang chạy, **When** người dùng mở nó lần nữa, **Then** không có tiến trình thứ hai; cửa sổ của tiến trình đang chạy được đưa lên.

---

### Edge Cases

- Giao diện bị đóng/treo khi lõi đang chạy skill: lõi vẫn tôn trọng phím dừng khẩn cấp; mở lại giao diện thì thấy đúng trạng thái hiện tại của lần chạy.
- Hai yêu cầu cần người dùng cùng lúc (hộp hỏi lịch và phê duyệt của lần chạy đang chạy): chỉ hiện yêu cầu của lần chạy đang chạy; hộp hỏi lịch chờ (lõi xếp hàng).
- Màn hình DPI cao, hai màn hình khác tỉ lệ: thanh nổi và hộp hỏi hiện đúng trên màn hình đang dùng, chữ không mờ.
- Người dùng thu nhỏ hoặc che hộp hỏi: hộp hỏi vẫn có thể mở lại từ khay và từ thông báo cho tới khi lõi đánh dấu hết hạn.
- Ảnh chụp bị thiếu (đã bị dọn theo giới hạn log): hiện ô giữ chỗ "ảnh không còn", không lỗi.
- Tên skill hoặc nhãn phần tử rất dài, có tiếng Việt có dấu: hiển thị đủ dấu, cắt gọn có dấu "…", xem đầy đủ khi rê chuột.
- Thông báo Windows bị tắt bởi người dùng hoặc chính sách công ty: hộp hỏi và biểu tượng khay (có chấm báo) vẫn hoạt động.

## Requirements *(mandatory)*

### Functional Requirements

**Chung**

- **UI-001**: Giao diện MUST chỉ thực hiện thao tác thông qua các chức năng do lõi (001) cung cấp; MUST NOT tự đọc/ghi file skill, cài đặt, lịch, hay gọi máy chủ model trực tiếp.
- **UI-002**: Giao diện MUST NOT tải hay hiển thị nội dung từ mạng; mọi tài nguyên (chữ, biểu tượng, font) nằm sẵn trong ứng dụng; ảnh chụp chỉ đọc từ thư mục dữ liệu trên máy.
- **UI-003**: Khoá truy cập model và giá trị bí mật MUST chỉ đi một chiều từ giao diện vào kho bí mật qua lõi; giao diện MUST NOT nhận lại hay hiển thị các giá trị đó.
- **UI-004**: Toàn bộ chữ trên giao diện và thông báo MUST có đủ tiếng Việt và tiếng Anh, đổi ngay khi người dùng đổi ngôn ngữ (FR-048); chữ tiếng Việt có dấu MUST hiển thị đúng.
- **UI-005**: Giao diện MUST phản ánh trạng thái của lõi trong vòng 1 giây sau mỗi thay đổi (bước mới, tạm dừng, kết thúc, yêu cầu hỏi lịch) và MUST NOT bị treo khi lõi đang làm việc dài (kiểm tra model, chạy skill).
- **UI-006**: Chỉ có một tiến trình ứng dụng; mở lần nữa thì đưa cửa sổ hiện có lên.

**Danh sách skill**

- **UI-010**: Màn hình chính MUST liệt kê skill với tên, mô tả, trạng thái (nháp / sẵn sàng), kết quả và thời điểm lần chạy gần nhất, số bước đã tự vá đang chờ duyệt; có nút Dạy việc mới, Chạy, Sửa, Xoá (xoá phải xác nhận). (thay FR-047)

**Ghi**

- **UI-020**: Bắt đầu ghi sau khi nhập mô tả; trong lúc ghi, cửa sổ chính MUST thu lại và một thanh nổi nhỏ luôn-trên-cùng MUST hiện thời gian đã ghi, Tạm dừng/Tiếp tục, Dừng, và trạng thái tạm dừng rõ ràng. (thay phần giao diện của FR-001)
- **UI-021**: Thao tác trên thanh nổi và mọi cửa sổ của ứng dụng MUST NOT thành bước trong skill.
- **UI-022**: Thanh nổi MUST hiện cảnh báo không chặn khi lõi báo một bước chỉ có nhận diện theo vị trí (FR-008).
- **UI-023**: Khi ghi xong, MUST mở màn hình Sửa của skill vừa sinh; không có bước nào thì báo và không tạo skill.

**Chạy và theo dõi**

- **UI-030**: Bấm Chạy với skill có tham số MUST hiện biểu mẫu nhập giá trị (gợi ý ví dụ, bắt buộc đủ).
- **UI-031**: Màn hình Theo dõi MUST hiện: bước hiện tại (ý định, ảnh lúc ghi), nấc đang dùng (0–5) bằng lời dễ hiểu, log trực tiếp, số lần gọi model / ngân sách, thời gian đã chạy / ngân sách, nút Dừng.
- **UI-032**: Với mỗi lý do tạm dừng do lõi báo, MUST hiện lý do và đúng các lựa chọn: phê duyệt → Đồng ý / Từ chối; người dùng chạm → Tiếp tục / Dừng; kẹt → ảnh lúc ghi + ảnh hiện tại + nguyên nhân + Tiếp tục / Huỷ; thiếu bí mật → ô nhập ẩn ký tự + Tiếp tục / Huỷ. (thay phần giao diện của FR-021, FR-024, FR-032)
- **UI-033**: Giao diện MUST NOT có lựa chọn tự phê duyệt, "nhớ lựa chọn", hay bỏ qua phê duyệt.
- **UI-034**: Kết thúc lần chạy, MUST hiện kết quả và danh sách bước đã tự vá với Giữ / Hoàn tác từng bước; danh sách tự vá của lần chạy theo lịch MUST hiện ở lần mở giao diện kế tiếp. (thay phần giao diện của FR-027)
- **UI-035**: Lần chạy thử thành công của skill nháp MUST hỏi xác nhận; Xác nhận → chuyển lõi đổi trạng thái sang sẵn sàng.
- **UI-036**: Người dùng MUST xem được lịch sử lần chạy của mỗi skill (trong giới hạn lõi giữ log) gồm từng bước, nấc, câu hỏi gửi model và câu trả lời, ảnh.

**Sửa skill**

- **UI-040**: Màn hình Sửa MUST hiện từng bước với ảnh trước/sau, ý định, cách nhận diện, điều kiện kết quả, cờ phê duyệt, điều kiện bỏ qua, cảnh báo; và cho sửa ý định, điều kiện, điều kiện bỏ qua, cờ phê duyệt, xoá bước, kéo-thả sắp xếp. (thay FR-014)
- **UI-041**: Bôi đen một đoạn văn bản trong bước gõ → "Biến thành tham số" (tên, mô tả, ví dụ). Giao diện MUST NOT gợi ý tham số tự động.
- **UI-042**: Chèn tham chiếu bí mật bằng tên khoá, nhập giá trị một lần (ẩn ký tự), không bao giờ hiển thị lại.
- **UI-043**: Lưu thay đổi bước của skill sẵn sàng MUST cảnh báo trước rằng skill về nháp và lịch của nó tạm ngưng.
- **UI-044**: Lỗi hợp lệ do lõi trả về MUST hiện cạnh trường liên quan; không lưu khi còn lỗi.

**Cài đặt**

- **UI-050**: Màn hình Cài đặt MUST có: địa chỉ máy chủ, khoá truy cập (chỉ ghi), tên model, thời gian chờ gọi model, Kiểm tra kết nối, Kiểm tra model, phím tắt (dừng khẩn cấp, tạm dừng/dừng ghi), thư mục dữ liệu, số lần chạy giữ log, ngân sách mặc định, ngôn ngữ, khởi động cùng Windows. (thay phần giao diện của FR-035–FR-038, FR-048)
- **UI-051**: Lần đầu lưu cấu hình model MUST hiện thông báo dữ liệu và chỉ lưu khi người dùng Đồng ý (FR-037).
- **UI-052**: Kết quả Kiểm tra kết nối / Kiểm tra model MUST hiện lý do lỗi dễ hiểu hoặc tỉ lệ đúng theo loại và tổng so với ngưỡng 90%.

**Lịch, khay, thông báo**

- **UI-060**: Ứng dụng MUST có biểu tượng khay; đóng cửa sổ thì xuống khay; menu khay có Mở, Tạm ngưng tất cả lịch, Thoát. Thoát khi đang chạy skill MUST hỏi xác nhận và dừng lần chạy trước. (thay FR-040)
- **UI-061**: Tuỳ chọn khởi động cùng Windows MUST khởi động thẳng xuống khay. (thay FR-040)
- **UI-062**: Màn hình Lịch MUST hiện danh sách lịch (skill, thời gian bằng lời dễ hiểu, lần chạy kế tiếp, bật/tắt, lý do tạm ngưng nếu skill về nháp), lịch sử đầy đủ, và dòng nhắc lịch chỉ chạy khi Windows mở khoá. (thay FR-045, FR-046)
- **UI-063**: Tạo/sửa lịch bằng mẫu (hằng ngày, hằng tuần, mỗi N phút) hoặc biểu thức cron, kèm giá trị tham số và thời hạn chờ; chỉ chọn được skill sẵn sàng; hiện trước 3 lần chạy kế tiếp để người dùng kiểm tra.
- **UI-064**: Khi lõi yêu cầu hỏi, MUST hiện hộp hỏi luôn-trên-cùng (Chạy ngay / Hoãn N phút / Bỏ qua lần này) kèm thông báo Windows; bấm thông báo mở hộp hỏi. Hộp hỏi MUST NOT có lựa chọn chạy tự động. (thay phần giao diện của FR-041)
- **UI-065**: Khi lần chạy theo lịch cần phê duyệt, bị kẹt, hoặc kết thúc, MUST gửi thông báo Windows; bấm vào mở màn hình Theo dõi. (thay phần giao diện của FR-044)
- **UI-066**: Khi thông báo Windows không hiển thị được, biểu tượng khay MUST có dấu báo có việc đang chờ người dùng.

### Key Entities

Giao diện không có dữ liệu riêng ngoài trạng thái hiển thị; mọi thực thể (Skill, Bước, Lần chạy, Lịch, Cài
đặt…) thuộc spec 001. Thêm:

- **Yêu cầu người dùng (User prompt)**: một việc lõi đang chờ người dùng quyết — loại (phê duyệt, kẹt, chạm
  chuột/phím, thiếu bí mật, hỏi lịch, xác nhận chạy thử, duyệt tự vá), ngữ cảnh hiển thị, các lựa chọn hợp lệ,
  hạn chót (nếu có). Giao diện chỉ hiển thị và trả về một trong các lựa chọn hợp lệ.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SCU-001**: Người dùng làm trọn vòng dạy → sửa → chạy thử → phê duyệt → xác nhận sẵn sàng → lên lịch chỉ bằng giao diện, không mở hay sửa file bằng tay. (thay SC-007)
- **SCU-002**: Người dùng lần đầu (chưa được hướng dẫn) dạy và chạy được một skill 5 bước trên Notepad trong ≤ 10 phút.
- **SCU-003**: Trong mọi phép thử lịch: 100% lần lõi yêu cầu hỏi đều có hộp hỏi hiển thị (kể cả khi cửa sổ đã xuống khay hoặc máy rảnh), và giao diện không có đường nào bắt đầu lần chạy theo lịch mà không qua nút Chạy ngay. (phần giao diện của SC-008)
- **SCU-004**: Màn hình Theo dõi và hộp hỏi phản ánh thay đổi trạng thái của lõi trong ≤ 1 giây ở 95% lần đo.
- **SCU-005**: 0 chuỗi chưa dịch khi chuyển qua lại tiếng Việt/tiếng Anh trên toàn bộ màn hình; 100% chữ tiếng Việt có dấu hiển thị đúng.
- **SCU-006**: 0 lần giao diện hiển thị khoá truy cập hoặc giá trị bí mật; 0 yêu cầu mạng phát ra từ phần giao diện.
- **SCU-007**: Ứng dụng nằm ở khay 8 giờ liền vẫn hiện hộp hỏi đúng giờ và mở lại cửa sổ được ngay khi bấm biểu tượng khay.

## Assumptions

- **Phụ thuộc**: cần lõi của spec 001 cung cấp đủ chức năng (ghi, biên dịch, chạy, tạm dừng/tiếp tục, phê duyệt,
  tự vá, cài đặt, kiểm tra model, lịch và lịch sử) cùng luồng thông báo trạng thái và "yêu cầu người dùng". Danh
  sách chức năng/sự kiện cụ thể chốt ở `/speckit-plan` của 002 và phải khớp với plan 001.
- **Công nghệ giao diện (người dùng đã chốt)**: Tauri. Đây là quyết định của người dùng, ghi ở đây để
  `/speckit-plan` dùng; spec không phụ thuộc chi tiết của nó. Thay cho đề xuất GUI trong plan 001.
- **Constitution**: v1.1.0 ghi Tauri là công nghệ giao diện đã chốt (Nguyên tắc VII) và coi ranh giới lõi ⇄
  giao diện là ranh giới tin cậy (mục Ràng buộc nền tảng).
- **Nền tảng**: Windows 10 22H2+ / 11 có sẵn thành phần hiển thị web của hệ điều hành (mặc định trên các bản này).
- **Phong cách**: giao diện văn phòng gọn, dễ đọc; theo chế độ sáng/tối của Windows. Không cần trợ năng nâng cao
  ngoài điều hướng bàn phím cơ bản và tương phản đủ đọc.
- **Ngoài phạm vi**: giao diện cho macOS/Linux, giao diện web từ xa, nhiều người dùng, chia sẻ skill, trình sửa
  trực quan dạng sơ đồ luồng.
- **Lộ trình**: tương ứng M4 (giao diện đầy đủ) và phần giao diện của M5 (khay, hộp hỏi, thông báo) trong spec 001.
