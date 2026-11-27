# Hướng dẫn chạy M0 trên máy công ty

Áp dụng cho [quickstart.md §M0](./quickstart.md#m0). Máy công ty phải đo:

- **Cổng B**: OCR (0.6a, 0.6b — T008) và giao thức model (0.10 — T012). Đây là điều kiện để bắt đầu US4/M3 và phần OCR.
- **Đo lại cổng A**: 0.1–0.5, 0.7–0.9, 0.11, 0.13, 0.14. Không chặn task nào, nhưng là điều kiện của checkpoint M2.

Kết quả đo trên máy dev **không** thay được kết quả máy công ty. Ghi mọi kết quả vào
[research.md §Kết quả M0](./research.md#kết-quả-m0), mục "Cổng B — máy công ty" và "Cổng A — máy công ty". Sau đó
đánh dấu T013b trong tasks.md.

---

## 0. Trước khi đi

1. **Hỏi IT trước**: có được chạy exe tự build (chưa ký số) trên máy công ty không, có được cài Rust không, có được
   dùng API key model cho mục đích đo không. Không tự vượt chính sách nhóm (GPO), AppLocker hay phần mềm diệt virus.
2. Chuẩn bị đủ thông tin model cho **cả DeepSeek và GLM**: base URL (dạng `https://.../v1`), tên model, API key.
3. Chọn một trong hai cách đưa chương trình sang:

**Cách A — mang exe đã build sẵn** (khi máy công ty không có Rust hoặc không vào được crates.io). Trên máy dev:

```powershell
cd F:\project-aemeath
git pull
cargo build --release --examples
$out = "$env:USERPROFILE\Desktop\teach-m0"
New-Item -ItemType Directory -Force $out | Out-Null
Copy-Item target\release\examples\*.exe $out -Exclude *-*.exe
Copy-Item specs\001-teach-a-task\m0-company-runbook.md $out
Get-ChildItem $out
```

Chép thư mục `teach-m0` sang máy công ty bằng kênh IT cho phép. Ở các bước dưới, thay
`cargo run --release --example X --` bằng `.\X.exe`.

**Cách B — build trên máy công ty** (có Rust ≥ 1.89, MSVC Build Tools, vào được crates.io qua proxy công ty):

```powershell
git clone <repo> ; cd project-aemeath ; git checkout 001-teach-a-task
cargo build --release --examples
```

Nếu `cargo` không tải được crate qua proxy: đặt `$env:HTTPS_PROXY = "<proxy IT cung cấp>"` cho phiên PowerShell đó.
Không ghi proxy vào file nào trong repo.

4. Mở một file ghi chú `m0-cty-<ngày>.txt` để dán nguyên văn output từng lệnh. Ghi thêm phiên bản Windows
   (`winver`), DPI màn hình, bộ gõ tiếng Việt đang dùng (Unikey/EVKey/bộ gõ Windows), trình duyệt, phiên bản Office.

Chạy mọi lệnh bằng **tài khoản thường, không Run as administrator**.

---

## 1. Cổng B — OCR (0.6a, 0.6b) — T008

```powershell
Get-WindowsCapability -Online | ? Name -like 'Language.OCR*' | ft Name, State
cargo run --release --example spike_ocr -- --save ocr.png
```

- Lệnh đầu cho biết gói OCR nào đã cài. Dán nguyên văn vào ghi chú. Nếu `Get-WindowsCapability` bị chặn quyền, ghi
  lại lỗi đó và vẫn chạy spike.
- Spike tự mở một cửa sổ có 10 nút nhãn tiếng Việt, chụp lại, OCR rồi tự đóng. **Không bấm gì** trong lúc nó chạy.
- Đọc hai dòng kết quả:
  - `0.6a: PASS|FAIL engine=... err=...` — OCR có gọi được từ exe **không đóng gói** hay không.
  - `0.6b: k/10 PASS|FAIL ...` — cần ≥ 9/10.
- Nếu **0.6a FAIL**: dừng phần OCR, ghi lỗi `err=` (HRESULT). Theo plan D-3 phải chọn một trong ba: sparse package
  cấp identity, Tesseract `vie`, hoặc bỏ `ocr_text` khỏi MVP. Quyết định làm sau, ở máy dev.
- Kiểu phân phối của plan 002 chưa chốt, nên phần "OCR trong kiểu phân phối 002" của 0.6a để mở, ghi rõ trong
  research.md.

## 2. Cổng B — giao thức model (0.10) — T012

Chạy **cho từng model** (DeepSeek, rồi GLM). API key chỉ đặt trong biến môi trường của phiên PowerShell, không
ghi vào file, xoá ngay sau khi chạy:

```powershell
$env:TEACH_API_KEY = "<api key>"
cargo run --release --example spike_probe -- --url "https://<máy chủ>/v1" --model "<tên model>" --n 20
Remove-Item Env:TEACH_API_KEY
```

- Nếu báo lỗi proxy hoặc kết nối: thêm `--proxy "<proxy IT cung cấp>"`. Spike in cả cấu hình proxy hệ thống
  (chỉ đọc) để đối chiếu.
- Nếu báo lỗi chứng chỉ TLS: ghi nguyên văn lỗi. Spike dùng kho chứng chỉ Windows, nên CA nội bộ do IT cài phải
  được nhận. Không tắt kiểm tra chứng chỉ.
- Dòng cuối cần ghi lại:
  `0.10: openai=... json_schema=... thinking=... p50_ms=... p95_ms=... ctx=... json_ok=k/20 PASS|FAIL`.
  Đạt khi `json_ok` ≥ 19/20.
- Trước khi dán output vào ghi chú, kiểm lại không có API key trong đó.

---

## 3. Đo lại cổng A

Đóng các ứng dụng không cần thiết. **Chạy từng spike một**, không chạy hai spike cùng lúc (các spike cùng đăng ký
Ctrl+Alt+Esc). Mỗi mục: dán output và ghi "đạt / không" theo cột "Đạt khi" ở quickstart §M0.

### 0.9 — bí mật (T011), không cần thao tác

```powershell
cargo run --release --example spike_secret
```

Đạt: dòng cuối `0.9: PASS persist=2 method=CredWriteW` (máy dev đo `keyring` lưu persist=3). Nếu GPO cấm mức
`LOCAL_MACHINE`, ghi lại lỗi.

### 0.1, 0.2, 0.3 — ghi thao tác (T006)

```powershell
cargo run --release --example spike_record -- 120
```

Trong 120 giây:

1. **0.1**: bấm 10 lần vào các nút/menu khác nhau trên Notepad, rồi 10 lần trên Edge (hoặc trình duyệt công ty).
   Đạt khi mỗi dòng `click` có `role` và (`name` hoặc `automation_id`).
2. **0.2**: bật bộ gõ tiếng Việt, gõ vào Notepad: `Tiếng Việt có dấu: Ươ ạ ẫ ỹ`. Đạt khi trường `typed` ghép lại
   khớp đúng chữ trong ô.
3. **0.3**: gõ vào ô mật khẩu ở (a) trang đăng nhập trên trình duyệt, (b) hộp thoại Win32 (ví dụ hộp đăng nhập
   ổ mạng), (c) một app Electron (Teams/Slack nếu có). Đạt khi các dòng phím ở ô mật khẩu có `is_password: true`
   (hoặc `unknown`) và **không có** ký tự nào. Ghi lại app nào ra `unknown`.

Cuối cùng xem dòng `summary`: `hook_over_1ms` nên bằng 0.

### 0.4, 0.5 — gửi phím tiếng Việt (T007)

```powershell
cargo run --release --example spike_input -- 5
```

Trong 5 giây đếm ngược, click vào ô đích. Spike gõ `Ươ ạ ẫ ỹ` lặp lại trong khoảng 10 giây.

- **0.4**: làm lần lượt với Notepad, Word, một app Electron, trình duyệt; mỗi app hai lần (bộ gõ Việt bật và tắt).
  Đạt khi chữ trong ô khớp chính xác.
- **0.5**: trong lúc spike đang gõ, chạm chuột/phím vài lần. Đạt khi các lần chạm hiện ở dòng `user`, kể cả khi bộ
  gõ Việt bật (`summary` có `user_key`/`user_mouse` > 0).

### 0.7 — chụp cửa sổ (T009)

```powershell
cargo run --release --example spike_capture -- notepad.exe .\cap
```

Chạy ba lần: (1) màn hình đặt 150 %; (2) nếu có hai màn hình khác DPI, đặt Notepad ở màn hình thứ hai; (3) mở một
cửa sổ app khác đè lên một phần Notepad. Mở các file PNG trong `.\cap`:

- Ảnh phải đúng khung cửa sổ, không lệch, không thừa viền. So `xcap.png` với `bitblt.png`; máy dev thấy `xcap` hụt
  5 px.
- Ở lần (3), vùng bị che phải tô đen.

### 0.8 — phím tắt (T010)

```powershell
cargo run --release --example spike_hotkeys
```

Focus lần lượt Notepad, rồi một ứng dụng toàn màn hình (video hoặc trình chiếu PowerPoint). Bấm **mỗi tổ hợp 20
lần**: Ctrl+Alt+Esc, Ctrl+Alt+P, Ctrl+Alt+S. Nhấn Ctrl+C trong cửa sổ PowerShell để kết thúc.

- Đạt khi `summary` có Ctrl+Alt+Esc `hook=20 wm_hotkey=20`, P và S `hook=20`.
- Kiểm bằng mắt: ứng dụng đích **không** nhận phím nào (Notepad không gõ ra `p`/`s`, trình chiếu không thoát vì
  Esc).
- Nếu có dòng `CẢNH BÁO: RegisterHotKey ... lỗi`: ghi lại. Nghĩa là máy công ty có chương trình khác giữ tổ hợp
  này. Chạy thêm `spike_hotkeys -- 0 --no-hotkey` để đo riêng đường hook.

### 0.11 — khoá máy, UAC, RDP (T090)

```powershell
cargo run --release --example spike_session -- --simulate record
```

Mỗi ca làm 5 lần: khoá máy (Win+L) rồi mở khoá; bật UAC (`Start-Process cmd -Verb RunAs`, rồi bấm No); nếu được
phép RDP vào máy: ngắt rồi nối lại phiên RDP (chạy spike **bên trong** phiên RDP). Lặp lại toàn bộ với
`--simulate run`. Đạt khi mọi lần đều có dòng sự kiện tương ứng (`WTS_SESSION_LOCK`, chuyển desktop `Winlogon`,
`WTS_REMOTE_DISCONNECT`). Ca thu nhỏ cửa sổ RDP ở máy khách: chỉ ghi lại hành vi.

### 0.13 — dừng khi app treo (T087, T088)

Mở hai cửa sổ PowerShell:

```powershell
# cửa sổ 1
cargo run --release --example bench_target
# cửa sổ 2, chạy từng ca
cargo run --release --example spike_stop -- --case uia_read  --runs 5 --auto-stop-after 1000
cargo run --release --example spike_stop -- --case invoke    --runs 5 --auto-stop-after 1000
cargo run --release --example spike_stop -- --case setvalue  --runs 5 --auto-stop-after 1000 --uia-timeout 5000
cargo run --release --example spike_stop -- --case sendinput --runs 5 --auto-stop-after 1000
```

Ca `sendinput` di chuột thật, **không chạm** chuột/phím trong lúc chạy. Đạt khi mỗi ca có
`5/5 pass (no dispatch begins > 0 ms after stop)`.

Thêm một lượt dừng bằng phím thật: bỏ `--auto-stop-after`, nhấn Ctrl+Alt+Esc khoảng 1 giây sau khi mỗi lần chạy
bắt đầu, `--runs 5` cho ca `invoke`.

### 0.14 — mất hook (T089)

```powershell
reg query "HKCU\Control Panel\Desktop" /v LowLevelHooksTimeout
cargo run --release --example spike_hookloss -- --mode force --duration 15
cargo run --release --example spike_hookloss -- --mode lag --synthetic-clicks 40 --rate 5 --real-uia
```

Không chạm chuột/phím trong lúc chạy. Ghi lại dòng `0.14: ...` của cả hai lệnh và giá trị `LowLevelHooksTimeout`.
Nếu GPO công ty đặt giá trị này, đó là thông tin quan trọng cho FR-053.

---

## 4. Sau khi đo

1. Kiểm lại file ghi chú không có API key, mật khẩu hay dữ liệu nghiệp vụ.
2. Mang file ghi chú về. Trên máy dev, ghi kết quả vào research.md §Kết quả M0 (bảng giống phần máy dev), đánh dấu
   T013b. Đánh dấu T008/T012 khi cổng B có kết quả.
3. Nếu 0.6a FAIL hoặc 0.10 FAIL: chưa bắt đầu US4/M3. Mở quyết định theo plan D-3 / research R-6 trước.
4. Xoá thư mục `teach-m0` khỏi máy công ty nếu IT yêu cầu.
