//! T006 spike: ghi sự kiện chuột/phím bằng hook LL + UI Automation. Chỉ đo, không phải code production.
//!
//! Đo quickstart §M0: 0.1 (cú bấm có `element` role + name/automation_id), 0.2 (`typed` khớp
//! văn bản gõ bằng bộ gõ Việt), 0.3 (ô mật khẩu: `is_password` true/false/"unknown").
//!
//! Chạy: `cargo run --example spike_record -- [giây]` (mặc định 0 = chạy tới Ctrl+C).
//!
//! Mỗi sự kiện in 1 dòng JSON ra stdout. Hook proc chỉ đẩy sự kiện thô vào kênh rồi trả về
//! ngay; UIA và ToUnicodeEx chạy ở luồng xử lý.
//!   - 0.1: bấm 10 lần trên Notepad và Edge; đếm dòng `mouse_down` có `element.role` và
//!     (`element.name` hoặc `element.automation_id`) khác null. (Chuyển động chuột bị bỏ qua.)
//!   - 0.2: bật bộ gõ Việt, gõ "Tiếng Việt có dấu: Ươ ạ ẫ ỹ"; ghép các `typed` của dòng
//!     `key_down` và so với nội dung ô. Dòng `vk=231` (VK_PACKET) lấy ký tự từ scancode.
//!   - 0.3: gõ vào ô mật khẩu (trình duyệt, hộp thoại Win32, Electron); xem `is_password`
//!     (true/false/"unknown") và `focus` để ghi lại app nào "unknown". Khi true, `typed` bị ẩn.
//!   - Thời gian: `hook_us` = thời gian hook proc trước khi đẩy kênh; dòng cuối `summary` có
//!     `hook_max_us` (gồm cả gửi kênh, không gồm CallNextHookEx; phải <= 1000) và `max_lag_ms`.
//!     `queue_ms` = chờ trong kênh, `lag_ms` = từ lúc hook nhận tới khi xử lý xong (gồm UIA).

use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::SeqCst};
use std::sync::{mpsc, OnceLock};
use std::time::{Duration, Instant};
use uiautomation::types::Point;
use uiautomation::{UIAutomation, UIElement};
use windows::core::BOOL;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Console::SetConsoleCtrlHandler;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyboardLayout, ToUnicodeEx};
use windows::Win32::UI::WindowsAndMessaging::*;

static STOP: AtomicBool = AtomicBool::new(false);
static EPOCH: OnceLock<Instant> = OnceLock::new();
static TX: OnceLock<mpsc::Sender<Raw>> = OnceLock::new();
static HOOK_MAX_US: AtomicU64 = AtomicU64::new(0);
static HOOK_OVER_1MS: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
enum Kind {
    Mouse { msg: u32, x: i32, y: i32 },
    Key { down: bool, vk: u32, sc: u32 },
}

struct Raw {
    kind: Kind,
    flags: u32,
    extra: usize,
    time: u32,
    t_us: u64,
    hook_us: u64,
}

fn now_us() -> u64 {
    EPOCH.get_or_init(Instant::now).elapsed().as_micros() as u64
}

fn push(kind: Kind, flags: u32, extra: usize, time: u32, t0: u64) {
    let hook_us = now_us() - t0;
    if let Some(tx) = TX.get() {
        let _ = tx.send(Raw { kind, flags, extra, time, t_us: t0, hook_us });
    }
    let full = now_us() - t0;
    HOOK_MAX_US.fetch_max(full, SeqCst);
    if full > 1000 {
        HOOK_OVER_1MS.fetch_add(1, SeqCst);
    }
}

unsafe extern "system" fn mouse_proc(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 && wp.0 as u32 != WM_MOUSEMOVE {
        let t0 = now_us();
        let m = unsafe { &*(lp.0 as *const MSLLHOOKSTRUCT) };
        let kind = Kind::Mouse { msg: wp.0 as u32, x: m.pt.x, y: m.pt.y };
        push(kind, m.flags, m.dwExtraInfo, m.time, t0);
    }
    unsafe { CallNextHookEx(None, code, wp, lp) }
}

unsafe extern "system" fn key_proc(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let t0 = now_us();
        let k = unsafe { &*(lp.0 as *const KBDLLHOOKSTRUCT) };
        let down = matches!(wp.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
        let kind = Kind::Key { down, vk: k.vkCode, sc: k.scanCode };
        push(kind, k.flags.0, k.dwExtraInfo, k.time, t0);
    }
    unsafe { CallNextHookEx(None, code, wp, lp) }
}

unsafe extern "system" fn on_ctrl(_: u32) -> BOOL {
    STOP.store(true, SeqCst);
    BOOL(1)
}

fn elem_json(e: &UIElement) -> Value {
    json!({
        "role": e.get_control_type().ok().map(|c| format!("{c:?}")),
        "name": e.get_name().ok(),
        "automation_id": e.get_automation_id().ok(),
        "class_name": e.get_classname().ok(),
        "rect": e.get_bounding_rectangle().ok()
            .map(|r| [r.get_left(), r.get_top(), r.get_right(), r.get_bottom()]),
    })
}

/// Chuyển phím thành văn bản bằng ToUnicodeEx (cờ 4 = không đổi trạng thái phím chết).
fn typed(state: &mut [u8; 256], vk: u32, sc: u32, down: bool) -> Option<String> {
    if vk == 0xE7 {
        // VK_PACKET (SendInput KEYEVENTF_UNICODE, kể cả bộ gõ Việt): scancode = đơn vị UTF-16
        return down.then(|| String::from_utf16_lossy(&[sc as u16]));
    }
    if vk < 256 {
        state[vk as usize] = if down { 0x80 } else { 0 };
        if vk == 0x14 && down {
            state[0x14] ^= 1;
        }
        state[0x10] = state[0xA0] | state[0xA1];
        state[0x11] = state[0xA2] | state[0xA3];
        state[0x12] = state[0xA4] | state[0xA5];
    }
    let (ctrl, alt) = (state[0x11] & 0x80 != 0, state[0x12] & 0x80 != 0);
    if !down || (ctrl != alt) {
        return None; // Ctrl hoặc Alt đơn lẻ = phím tắt; Ctrl+Alt = AltGr thì vẫn dịch
    }
    let mut buf = [0u16; 8];
    let n = unsafe {
        let fg = GetForegroundWindow();
        let tid = GetWindowThreadProcessId(fg, None);
        ToUnicodeEx(vk, sc, state, &mut buf, 4, Some(GetKeyboardLayout(tid)))
    };
    (n > 0).then(|| String::from_utf16_lossy(&buf[..n as usize]))
}

fn process(rx: mpsc::Receiver<Raw>) {
    let uia = UIAutomation::new().map_err(|e| eprintln!("UIAutomation::new lỗi: {e}")).ok();
    let (mut n, mut max_lag_ms, mut state) = (0u64, 0u64, [0u8; 256]);
    loop {
        let ev = match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(ev) => ev,
            Err(mpsc::RecvTimeoutError::Timeout) if STOP.load(SeqCst) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(_) => break,
        };
        let queue_us = now_us() - ev.t_us;
        let u0 = Instant::now();
        let mut j = match ev.kind {
            Kind::Mouse { msg, x, y } => {
                let btn = match msg {
                    WM_LBUTTONDOWN => "l_down", WM_LBUTTONUP => "l_up",
                    WM_RBUTTONDOWN => "r_down", WM_RBUTTONUP => "r_up",
                    WM_MBUTTONDOWN => "m_down", WM_MBUTTONUP => "m_up",
                    WM_MOUSEWHEEL => "wheel", _ => "other",
                };
                let mut j = json!({"ev": "mouse", "btn": btn, "x": x, "y": y,
                                   "injected": ev.flags & 1 != 0});
                if matches!(msg, WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN) {
                    j["ev"] = "mouse_down".into();
                    j["element"] = match uia.as_ref().map(|a| a.element_from_point(Point::new(x, y))) {
                        Some(Ok(e)) => elem_json(&e),
                        Some(Err(e)) => json!({"error": e.to_string()}),
                        None => Value::Null,
                    };
                }
                j
            }
            Kind::Key { down, vk, sc } => {
                let focus = uia.as_ref().filter(|_| down).map(|a| a.get_focused_element());
                // ba trạng thái: true / false / "unknown" (không lấy được phần tử hoặc thuộc tính)
                let pw = match &focus {
                    Some(Ok(e)) => e.is_password().map_or(json!("unknown"), |b| json!(b)),
                    Some(Err(_)) | None => json!("unknown"),
                };
                let text = typed(&mut state, vk, sc, down);
                let hide = pw == json!(true);
                json!({"ev": if down { "key_down" } else { "key_up" }, "vk": vk, "sc": sc,
                       "typed": if hide { None } else { text },
                       "redacted": hide, "is_password": pw,
                       "injected": ev.flags & 0x10 != 0,
                       "focus": match &focus { Some(Ok(e)) => elem_json(e), _ => Value::Null }})
            }
        };
        let lag_ms = (now_us() - ev.t_us) / 1000;
        max_lag_ms = max_lag_ms.max(lag_ms);
        n += 1;
        j["flags"] = ev.flags.into();
        j["extra"] = ev.extra.into();
        j["time"] = ev.time.into();
        j["t_us"] = ev.t_us.into();
        j["hook_us"] = ev.hook_us.into();
        j["queue_ms"] = (queue_us / 1000).into();
        j["uia_ms"] = (u0.elapsed().as_millis() as u64).into();
        j["lag_ms"] = lag_ms.into();
        println!("{j}");
    }
    println!(
        "{}",
        json!({"summary": true, "events": n, "hook_max_us": HOOK_MAX_US.load(SeqCst),
               "hook_over_1ms": HOOK_OVER_1MS.load(SeqCst), "max_lag_ms": max_lag_ms})
    );
}

fn main() {
    let secs: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let _ = SetConsoleCtrlHandler(Some(on_ctrl), true);
    }
    now_us();
    let (tx, rx) = mpsc::channel();
    TX.set(tx).ok();
    let proc_t = std::thread::spawn(move || process(rx));

    let (tid_tx, tid_rx) = mpsc::channel::<Result<u32, String>>();
    let hook_t = std::thread::spawn(move || unsafe {
        let hmod = GetModuleHandleW(None).ok().map(Into::into);
        let hooks = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), hmod, 0)
            .and_then(|m| SetWindowsHookExW(WH_KEYBOARD_LL, Some(key_proc), hmod, 0).map(|k| (m, k)));
        let (m, k) = match hooks {
            Ok(h) => h,
            Err(e) => return drop(tid_tx.send(Err(e.to_string()))),
        };
        let _ = tid_tx.send(Ok(GetCurrentThreadId()));
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        let _ = UnhookWindowsHookEx(m);
        let _ = UnhookWindowsHookEx(k);
    });
    let tid = match tid_rx.recv() {
        Ok(Ok(t)) => t,
        r => {
            eprintln!("cài hook lỗi: {r:?}");
            std::process::exit(1);
        }
    };
    eprintln!("hook đã cài; chạy {}. Ctrl+C để dừng.", if secs > 0 { format!("{secs} giây") } else { "tới Ctrl+C".into() });
    let end = (secs > 0).then(|| Instant::now() + Duration::from_secs(secs));
    while !STOP.load(SeqCst) && end.is_none_or(|e| Instant::now() < e) {
        std::thread::sleep(Duration::from_millis(50));
    }
    STOP.store(true, SeqCst);
    unsafe {
        let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
    }
    let _ = hook_t.join();
    let _ = proc_t.join();
}
