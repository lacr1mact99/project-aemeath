//! M0 spike T008 / quickstart rows 0.6a + 0.6b / plan D-3 / research R-5: Windows.Media.Ocr.
//!
//! Gate (1) 0.6a: can this UNPACKAGED exe create an `OcrEngine` (package identity, class
//! registration, installed OCR language packs)? Prints `0.6a: PASS|FAIL engine=<lang|none> err=<hr|->`;
//! no engine -> exit 2.
//! Gate (2) 0.6b: own visible window with 10 Vietnamese BUTTONs (Segoe UI 12pt, PerMonitorV2),
//! client area captured with GDI BitBlt -> SoftwareBitmap(Bgra8) -> `RecognizeAsync` (5 s limit,
//! FR-004). A label matches a button when an OCR word/line intersecting the button rect has
//! `fold(text) == fold(label)` or Levenshtein <= 1 per 4 chars. Need >= 9/10.
//! Exit code 0 iff both gates pass.
//!
//! Run (release): `cargo run --release --example spike_ocr [-- --save C:\temp\ocr.png]`
//!
//! MUST run on the COMPANY machine (inside the company network/image) to count; dev-machine
//! results are informational only. On the company machine also run, and paste the output into
//! research.md (section Ket qua M0):
//!   Get-WindowsCapability -Online | ? Name -like 'Language.OCR*'
//! Also run the exe as shipped by plan 002 (D-3) if that differs from this unpackaged exe.

use std::ffi::c_void;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use teach_core::fold;
use windows::Globalization::Language;
use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::DataWriter;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{
    AdjustWindowRectExForDpi, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForWindow,
    SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{HSTRING, PCWSTR};

const LABELS: [&str; 10] = [
    "Lưu", "Gửi", "Xoá", "Huỷ", "Đóng", "Mở tệp", "Xác nhận", "Thanh toán", "Đặt hàng", "Tìm kiếm",
];

fn w(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn hr(e: &windows::core::Error) -> String {
    format!("{:#010x} {}", e.code().0 as u32, e.message())
}

fn lang(tag: &str) -> Language {
    Language::CreateLanguage(&HSTRING::from(tag)).expect("CreateLanguage")
}

fn lev(a: &str, b: &str) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i];
        for j in 1..=b.len() {
            let sub = prev[j - 1] + (a[i - 1] != b[j - 1]) as usize;
            cur.push(sub.min(prev[j] + 1).min(cur[j - 1] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

/// Gate 0.6a. Returns (engine, language tag used).
fn gate_a() -> Option<(OcrEngine, String)> {
    println!("== 0.6a: OcrEngine from unpackaged exe ==");
    match OcrEngine::AvailableRecognizerLanguages() {
        Ok(v) => {
            let tags: Vec<String> =
                v.into_iter().filter_map(|l| l.LanguageTag().ok()).map(|t| t.to_string()).collect();
            println!("AvailableRecognizerLanguages = {tags:?}");
        }
        Err(e) => println!("AvailableRecognizerLanguages error: {}", hr(&e)),
    }
    let mut err = String::from("-");
    let mut engine = None;
    for tag in ["vi-VN", "en-US"] {
        match OcrEngine::IsLanguageSupported(&lang(tag)) {
            Ok(b) => println!("IsLanguageSupported({tag}) = {b}"),
            Err(e) => {
                println!("IsLanguageSupported({tag}) error: {}", hr(&e));
                err = hr(&e);
            }
        }
    }
    for tag in ["vi-VN", "en-US"] {
        match OcrEngine::TryCreateFromLanguage(&lang(tag)) {
            Ok(e) => {
                println!("TryCreateFromLanguage({tag}) -> engine");
                engine.get_or_insert((e, tag.to_string()));
            }
            Err(e) => {
                println!("TryCreateFromLanguage({tag}) -> none/error: {}", hr(&e));
                err = hr(&e);
            }
        }
    }
    match OcrEngine::TryCreateFromUserProfileLanguages() {
        Ok(e) => {
            let tag = e
                .RecognizerLanguage()
                .and_then(|l| l.LanguageTag())
                .map(|t| t.to_string())
                .unwrap_or_default();
            println!("TryCreateFromUserProfileLanguages -> engine ({tag})");
            engine.get_or_insert((e, tag));
        }
        Err(e) => {
            println!("TryCreateFromUserProfileLanguages -> none/error: {}", hr(&e));
            err = hr(&e);
        }
    }
    match &engine {
        Some((_, tag)) => println!("0.6a: PASS engine={tag} err=-"),
        None => println!("0.6a: FAIL engine=none err={err}"),
    }
    engine
}

extern "system" fn proc(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe { DefWindowProcW(h, m, wp, lp) }
}

type R = (i32, i32, i32, i32); // l,t,r,b

/// Window with the 10 buttons. Returns (hwnd, dpi, client w/h, button rects in client px).
fn make_window() -> (HWND, u32, i32, i32, Vec<R>) {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let hi: HINSTANCE = GetModuleHandleW(None).expect("GetModuleHandleW").into();
        let cls = w("TeachSpikeOcr");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(proc),
            hInstance: hi,
            lpszClassName: PCWSTR(cls.as_ptr()),
            hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as isize as *mut c_void),
            hCursor: LoadCursorW(None, IDC_ARROW).ok().unwrap_or_default(),
            ..Default::default()
        };
        assert!(RegisterClassW(&wc) != 0, "RegisterClassW");
        let title = w("Teach spike OCR");
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST,
            PCWSTR(cls.as_ptr()),
            PCWSTR(title.as_ptr()),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            100,
            100,
            800,
            700,
            None,
            None,
            Some(hi),
            None,
        )
        .expect("CreateWindowExW");
        let dpi = GetDpiForWindow(hwnd);
        let s = |v: i32| v * dpi as i32 / 96;
        let (bw, bh, gx, gy, pad) = (s(220), s(56), s(30), s(24), s(20));
        let (cw, ch) = (pad * 2 + bw * 2 + gx, pad * 2 + bh * 5 + gy * 4);
        let mut r = RECT { left: 0, top: 0, right: cw, bottom: ch };
        let _ = AdjustWindowRectExForDpi(&mut r, WS_OVERLAPPEDWINDOW, false, WINDOW_EX_STYLE(0), dpi);
        let _ = SetWindowPos(hwnd, None, 100, 100, r.right - r.left, r.bottom - r.top, SWP_NOZORDER);
        let face = w("Segoe UI");
        let font = CreateFontW(
            -(12 * dpi as i32 / 72),
            0,
            0,
            0,
            400,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            0,
            PCWSTR(face.as_ptr()),
        );
        let btn = w("BUTTON");
        let mut rects = vec![];
        for (i, label) in LABELS.iter().enumerate() {
            let (col, row) = ((i % 2) as i32, (i / 2) as i32);
            let (x, y) = (pad + col * (bw + gx), pad + row * (bh + gy));
            let t = w(label);
            let b = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                PCWSTR(btn.as_ptr()),
                PCWSTR(t.as_ptr()),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP,
                x,
                y,
                bw,
                bh,
                Some(hwnd),
                None,
                Some(hi),
                None,
            )
            .expect("CreateWindowExW button");
            SendMessageW(b, WM_SETFONT, Some(WPARAM(font.0 as usize)), Some(LPARAM(1)));
            rects.push((x, y, x + bw, y + bh));
        }
        let _ = SetForegroundWindow(hwnd);
        let until = Instant::now() + Duration::from_millis(1000);
        while Instant::now() < until {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let mut c = RECT::default();
        let _ = GetClientRect(hwnd, &mut c);
        (hwnd, dpi, c.right, c.bottom, rects)
    }
}

/// BitBlt the client area from the window DC into top-down BGRA (alpha forced to 255).
fn capture(hwnd: HWND, cw: i32, ch: i32) -> Vec<u8> {
    unsafe {
        let wdc = GetDC(Some(hwnd));
        let mdc = CreateCompatibleDC(Some(wdc));
        let bi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: cw,
                biHeight: -ch,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut c_void = std::ptr::null_mut();
        let bmp = CreateDIBSection(Some(wdc), &bi, DIB_RGB_COLORS, &mut bits, None, 0)
            .expect("CreateDIBSection");
        let old = SelectObject(mdc, bmp.into());
        BitBlt(mdc, 0, 0, cw, ch, Some(wdc), 0, 0, SRCCOPY).expect("BitBlt");
        let mut buf = std::slice::from_raw_parts(bits as *const u8, (cw * ch * 4) as usize).to_vec();
        for px in buf.chunks_exact_mut(4) {
            px[3] = 255;
        }
        SelectObject(mdc, old);
        let _ = DeleteObject(bmp.into());
        let _ = DeleteDC(mdc);
        ReleaseDC(Some(hwnd), wdc);
        buf
    }
}

fn hit(a: R, b: R) -> bool {
    a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3
}

fn gate_b(engine: OcrEngine, tag: &str, save: Option<String>) -> bool {
    println!("== 0.6b: OCR of 10 Vietnamese buttons (engine lang={tag}) ==");
    let (hwnd, dpi, cw, ch, rects) = make_window();
    println!("DPI = {dpi} (scale {}%), client = {cw}x{ch} px", dpi * 100 / 96);
    let pix = capture(hwnd, cw, ch);
    if let Some(p) = &save {
        let mut rgba = pix.clone();
        rgba.chunks_exact_mut(4).for_each(|p| p.swap(0, 2));
        match image::RgbaImage::from_raw(cw as u32, ch as u32, rgba).unwrap().save(p) {
            Ok(()) => println!("saved {p}"),
            Err(e) => println!("save failed: {e}"),
        }
    }
    let max = OcrEngine::MaxImageDimension().unwrap_or(0);
    if max > 0 && (cw as u32 > max || ch as u32 > max) {
        println!("WARN image {cw}x{ch} exceeds MaxImageDimension {max}");
    }
    let run = move || -> windows::core::Result<Vec<(String, R, bool)>> {
        let dw = DataWriter::new()?;
        dw.WriteBytes(&pix)?;
        let bmp = SoftwareBitmap::Create(BitmapPixelFormat::Bgra8, cw, ch)?;
        bmp.CopyFromBuffer(&dw.DetachBuffer()?)?;
        let res = engine.RecognizeAsync(&bmp)?.join()?;
        let mut out = vec![];
        for line in res.Lines()? {
            let mut lr: Option<R> = None;
            for word in line.Words()? {
                let b = word.BoundingRect()?;
                let r = (b.X as i32, b.Y as i32, (b.X + b.Width) as i32, (b.Y + b.Height) as i32);
                out.push((word.Text()?.to_string(), r, false));
                lr = Some(lr.map_or(r, |u| (u.0.min(r.0), u.1.min(r.1), u.2.max(r.2), u.3.max(r.3))));
            }
            if let Some(r) = lr {
                out.push((line.Text()?.to_string(), r, true));
            }
        }
        Ok(out)
    };
    // ponytail: WinRT .get() cannot be cancelled; worker thread + recv_timeout enforces the 5 s limit.
    let (tx, rx) = mpsc::channel();
    let t0 = Instant::now();
    std::thread::spawn(move || {
        let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        let _ = tx.send(run());
    });
    let items = match rx.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => {
            println!("OCR error: {}", hr(&e));
            println!("0.6b: FAIL 0/10 (need >= 9) lang={tag} ocr_ms=-");
            return false;
        }
        Err(_) => {
            println!("OcrTimeout (> 5 s)");
            println!("0.6b: FAIL 0/10 (need >= 9) lang={tag} ocr_ms=>5000");
            return false;
        }
    };
    let ms = t0.elapsed().as_millis();
    println!("-- OCR output ({} items) --", items.len());
    for (t, r, is_line) in &items {
        println!("{} {:?} rect=({},{},{},{})", if *is_line { "line" } else { "word" }, t, r.0, r.1, r.2, r.3);
    }
    let mut ok = 0;
    println!("-- per button --");
    for (label, rect) in LABELS.iter().zip(&rects) {
        let f = fold(label);
        let allow = f.chars().count().div_ceil(4);
        let found = items
            .iter()
            .filter(|(_, r, _)| hit(*r, *rect))
            .map(|(t, _, l)| (t, *l, lev(&fold(t), &f)))
            .find(|(_, _, d)| *d <= allow);
        match found {
            Some((t, l, d)) => {
                ok += 1;
                println!("{label:<12} PASS via {} {t:?} dist={d} (allow {allow})", if l { "line" } else { "word" });
            }
            None => println!("{label:<12} FAIL (allow {allow})"),
        }
    }
    let pass = ok >= 9 && ms <= 5000;
    println!("0.6b: {ok}/10 {} (need >= 9) lang={tag} ocr_ms={ms}", if pass { "PASS" } else { "FAIL" });
    unsafe {
        let _ = DestroyWindow(hwnd);
    }
    pass
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let save = args.iter().position(|a| a == "--save").and_then(|i| args.get(i + 1)).cloned();
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
    let Some((engine, tag)) = gate_a() else { std::process::exit(2) };
    let b = gate_b(engine, &tag, save);
    std::process::exit(if b { 0 } else { 1 });
}
