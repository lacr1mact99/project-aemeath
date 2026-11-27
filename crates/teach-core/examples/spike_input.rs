//! T007 spike: SendInput Unicode với dấu hiệu `TEACH_MAGIC` + phân loại bằng hook LL. Chỉ đo, không phải code production.
//!
//! Đo quickstart §M0: 0.4 (văn bản gõ vào Notepad/Word/Electron/trình duyệt khớp chính xác,
//! bộ gõ Việt bật và tắt) và 0.5 (chạm chuột khi đang SendInput vẫn được nhận là `user`).
//!
//! Chạy: `cargo run --example spike_input -- [giây_đếm_ngược=5] [chuỗi] [--dry]`
//! `--dry`: chỉ in những gì sẽ gửi, không gửi, không cài hook.
//!
//! Người đo: chạy lệnh, trong thời gian đếm ngược click vào ô đích. Trong ~10 giây chương
//! trình gõ chuỗi mỗi giây một lần (cảnh báo: gõ thật vào cửa sổ đang focus).
//!   - 0.4: so nội dung ô với chuỗi lặp lại, lặp với bộ gõ Việt bật/tắt, từng ứng dụng.
//!   - 0.5: trong lúc đang gõ, chạm chuột / gõ tay một phím. Mỗi sự kiện `user` in ra 1 dòng
//!     JSON (gồm `injected`: bộ gõ Việt thường inject qua SendInput nhưng không mang magic).
//!     Cuối cùng in `summary`: `teach_*` phải bằng số đã gửi, mọi thứ còn lại là `user_*`.
//!   - Chuyển động chuột thuần bị bỏ qua. Ctrl+C để dừng sớm.

use serde_json::json;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::SeqCst};
use std::sync::{mpsc, OnceLock};
use std::time::{Duration, Instant};
use windows::core::BOOL;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Console::SetConsoleCtrlHandler;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    VIRTUAL_KEY,
};
use windows::Win32::UI::WindowsAndMessaging::*;

const TEACH_MAGIC: usize = 0x7EAC_4A5C;

static STOP: AtomicBool = AtomicBool::new(false);
static DONE: AtomicBool = AtomicBool::new(false);
static TX: OnceLock<mpsc::Sender<String>> = OnceLock::new();
static EPOCH: OnceLock<Instant> = OnceLock::new();
static TEACH_KEY: AtomicU64 = AtomicU64::new(0);
static TEACH_MOUSE: AtomicU64 = AtomicU64::new(0);
static USER_KEY: AtomicU64 = AtomicU64::new(0);
static USER_MOUSE: AtomicU64 = AtomicU64::new(0);
static USER_INJECTED: AtomicU64 = AtomicU64::new(0); // `user` nhưng cờ INJECTED = bộ gõ/ứng dụng khác

fn report(extra: usize, injected: bool, key: bool, desc: String) {
    let teach = extra == TEACH_MAGIC;
    let counter = match (teach, key) {
        (true, true) => &TEACH_KEY,
        (true, false) => &TEACH_MOUSE,
        (false, true) => &USER_KEY,
        (false, false) => &USER_MOUSE,
    };
    counter.fetch_add(1, SeqCst);
    if !teach {
        USER_INJECTED.fetch_add(injected as u64, SeqCst);
        if let Some(tx) = TX.get() {
            let t = EPOCH.get_or_init(Instant::now).elapsed().as_millis() as u64;
            let _ = tx.send(json!({"class": "user", "t_ms": t, "extra": extra,
                                   "injected": injected, "what": desc}).to_string());
        }
    }
}

unsafe extern "system" fn key_proc(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let k = unsafe { &*(lp.0 as *const KBDLLHOOKSTRUCT) };
        let desc = format!("key vk={} sc={} down={} flags={:#x}", k.vkCode, k.scanCode,
                           matches!(wp.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN), k.flags.0);
        report(k.dwExtraInfo, k.flags.0 & 0x10 != 0, true, desc);
    }
    unsafe { CallNextHookEx(None, code, wp, lp) }
}

unsafe extern "system" fn mouse_proc(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 && wp.0 as u32 != WM_MOUSEMOVE {
        let m = unsafe { &*(lp.0 as *const MSLLHOOKSTRUCT) };
        let desc = format!("mouse msg={:#x} at=({},{}) flags={:#x}", wp.0, m.pt.x, m.pt.y, m.flags);
        report(m.dwExtraInfo, m.flags & 1 != 0, false, desc);
    }
    unsafe { CallNextHookEx(None, code, wp, lp) }
}

unsafe extern "system" fn on_ctrl(_: u32) -> BOOL {
    STOP.store(true, SeqCst);
    BOOL(1)
}

fn key_input(unit: u16, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: unit,
                dwFlags: KEYEVENTF_UNICODE | if up { KEYEVENTF_KEYUP } else { Default::default() },
                time: 0,
                dwExtraInfo: TEACH_MAGIC,
            },
        },
    }
}

fn main() {
    let mut dry = false;
    let pos: Vec<String> = std::env::args().skip(1).filter(|a| { dry |= a == "--dry"; a != "--dry" }).collect();
    let countdown: u64 = pos.first().and_then(|s| s.parse().ok()).unwrap_or(5);
    let text = pos.get(1).cloned().unwrap_or_else(|| "Ươ ạ ẫ ỹ".to_string());
    let units: Vec<u16> = text.encode_utf16().collect();
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let _ = SetConsoleCtrlHandler(Some(on_ctrl), true);
    }
    if dry {
        println!("DRY: đếm ngược {countdown}s, rồi ~10 lần (mỗi 1s) gửi {text:?}, dwExtraInfo={TEACH_MAGIC:#x}");
        for u in &units {
            println!("  unit {u:#06x} ({}): KEYEVENTF_UNICODE down + up", String::from_utf16_lossy(&[*u]));
        }
        return;
    }
    let (tx, rx) = mpsc::channel();
    TX.set(tx).ok();
    EPOCH.get_or_init(Instant::now);
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

    let sender = std::thread::spawn(move || {
        for s in (1..=countdown).rev() {
            println!("gõ sau {s}s - hãy focus cửa sổ đích");
            std::thread::sleep(Duration::from_secs(1));
        }
        let (mut rounds, mut sent) = (0u32, 0u32);
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(10) && !STOP.load(SeqCst) {
            for &u in &units {
                let inputs = [key_input(u, false), key_input(u, true)];
                let n = unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) };
                if n != 2 {
                    eprintln!("SendInput trả {n}/2 cho unit {u:#06x} (UIPI chặn?)");
                }
                sent += n;
                std::thread::sleep(Duration::from_millis(30));
            }
            rounds += 1;
            std::thread::sleep(Duration::from_secs(1));
        }
        println!("đã gửi {rounds} vòng, {sent} INPUT (kỳ vọng teach_key = {sent})");
        DONE.store(true, SeqCst);
    });

    while !(DONE.load(SeqCst) || STOP.load(SeqCst)) {
        if let Ok(line) = rx.recv_timeout(Duration::from_millis(50)) {
            println!("{line}");
        }
    }
    std::thread::sleep(Duration::from_millis(300)); // chờ hook xử lý nốt
    STOP.store(true, SeqCst);
    unsafe {
        let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
    }
    let _ = hook_t.join();
    let _ = sender.join();
    while let Ok(line) = rx.try_recv() {
        println!("{line}");
    }
    println!(
        "{}",
        json!({"summary": true, "teach_key": TEACH_KEY.load(SeqCst), "teach_mouse": TEACH_MOUSE.load(SeqCst),
               "user_key": USER_KEY.load(SeqCst), "user_mouse": USER_MOUSE.load(SeqCst),
               "user_but_injected": USER_INJECTED.load(SeqCst)})
    );
}
