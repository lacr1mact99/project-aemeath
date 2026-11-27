//! T010 spike: phím tắt toàn cục qua hook LL (nuốt tổ hợp) + RegisterHotKey. Chỉ đo, không phải code production.
//!
//! Đo quickstart §M0 0.8: nhận 20/20 lần Ctrl+Alt+Esc, Ctrl+Alt+P, Ctrl+Alt+S khi cửa sổ khác
//! đang focus (kể cả ứng dụng toàn màn hình) và tổ hợp không lọt vào ứng dụng đích.
//!
//! Chạy: `cargo run --example spike_hotkeys -- [giây] [--esc-passthru]`
//! (giây mặc định 0 = chạy tới Ctrl+C).
//!
//! Người đo: focus Notepad / game / video toàn màn hình, bấm mỗi tổ hợp 20 lần, xem bộ đếm
//! mỗi dòng và `summary` khi thoát. Kiểm tra bằng mắt: ứng dụng đích không nhận phím nào
//! (Notepad không gõ ra "p"/"s", không mở menu/Start). Tổ hợp bị nuốt cả key down lẫn key up.
//!   - đường `hook`: hook LL. `wm_hotkey`: RegisterHotKey trên cửa sổ message-only (cùng luồng).
//!   - Hook LL chạy TRƯỚC RegisterHotKey nên nếu hook nuốt Ctrl+Alt+Esc thì WM_HOTKEY không
//!     bao giờ tới. Để đo cả hai đường độc lập, thêm `--esc-passthru`: hook vẫn đếm Esc nhưng
//!     không nuốt, nên `wm_hotkey` cũng nhận (và tổ hợp lọt xuống ứng dụng đích).

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering::SeqCst};
use std::sync::{mpsc, OnceLock};
use std::time::{Duration, Instant};
use windows::core::{w, BOOL};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Console::SetConsoleCtrlHandler;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT,
    VIRTUAL_KEY, VK_CONTROL, VK_ESCAPE, VK_MENU,
};
use windows::Win32::UI::WindowsAndMessaging::*;

const NAMES: [&str; 3] = ["Ctrl+Alt+Esc", "Ctrl+Alt+P", "Ctrl+Alt+S"];
const VKS: [u32; 3] = [0x1B, 0x50, 0x53];

static STOP: AtomicBool = AtomicBool::new(false);
static EPOCH: OnceLock<Instant> = OnceLock::new();
static TX: OnceLock<mpsc::Sender<(usize, &'static str, u64, bool)>> = OnceLock::new();
static PASSTHRU_ESC: AtomicBool = AtomicBool::new(false);
static CTRL: AtomicBool = AtomicBool::new(false);
static ALT: AtomicBool = AtomicBool::new(false);
static SWALLOWING: AtomicU32 = AtomicU32::new(0); // bit i = phím VKS[i] đang bị nuốt
static SWALLOWED_DOWN: AtomicU64 = AtomicU64::new(0);
static SWALLOWED_UP: AtomicU64 = AtomicU64::new(0);

fn now_us() -> u64 {
    EPOCH.get_or_init(Instant::now).elapsed().as_micros() as u64
}

fn emit(i: usize, path: &'static str, injected: bool) {
    if let Some(tx) = TX.get() {
        let _ = tx.send((i, path, now_us(), injected));
    }
}

unsafe extern "system" fn key_proc(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let k = unsafe { &*(lp.0 as *const KBDLLHOOKSTRUCT) };
        let down = matches!(wp.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
        match k.vkCode {
            0x11 | 0xA2 | 0xA3 => CTRL.store(down, SeqCst),
            0x12 | 0xA4 | 0xA5 => ALT.store(down, SeqCst),
            _ => {}
        }
        if let Some(i) = VKS.iter().position(|&v| v == k.vkCode) {
            let bit = 1u32 << i;
            let swallow = !(i == 0 && PASSTHRU_ESC.load(SeqCst));
            if !down {
                if SWALLOWING.fetch_and(!bit, SeqCst) & bit != 0 && swallow {
                    SWALLOWED_UP.fetch_add(1, SeqCst);
                    return LRESULT(1);
                }
            } else if SWALLOWING.load(SeqCst) & bit != 0 {
                if swallow {
                    SWALLOWED_DOWN.fetch_add(1, SeqCst); // auto-repeat khi đang giữ
                    return LRESULT(1);
                }
            } else {
                // fallback GetAsyncKeyState cho trường hợp thiếu sự kiện modifier
                let held = |vk: VIRTUAL_KEY| unsafe { GetAsyncKeyState(vk.0 as i32) } as u16 & 0x8000 != 0;
                let ctrl = CTRL.load(SeqCst) || held(VK_CONTROL);
                let alt = ALT.load(SeqCst) || held(VK_MENU);
                if ctrl && alt {
                    SWALLOWING.fetch_or(bit, SeqCst);
                    emit(i, "hook", k.flags.0 & 0x10 != 0);
                    if swallow {
                        SWALLOWED_DOWN.fetch_add(1, SeqCst);
                        return LRESULT(1);
                    }
                }
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wp, lp) }
}

unsafe extern "system" fn wnd_proc(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if m == WM_HOTKEY {
        emit(0, "wm_hotkey", false);
        return LRESULT(0);
    }
    unsafe { DefWindowProcW(h, m, wp, lp) }
}

unsafe extern "system" fn on_ctrl(_: u32) -> BOOL {
    STOP.store(true, SeqCst);
    BOOL(1)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let passthru = args.iter().any(|a| a == "--esc-passthru");
    let secs: u64 = args.iter().find_map(|a| a.parse().ok()).unwrap_or(0);
    PASSTHRU_ESC.store(passthru, SeqCst);
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let _ = SetConsoleCtrlHandler(Some(on_ctrl), true);
    }
    now_us();
    let (tx, rx) = mpsc::channel();
    TX.set(tx).ok();

    let (tid_tx, tid_rx) = mpsc::channel::<Result<u32, String>>();
    let hook_t = std::thread::spawn(move || unsafe {
        let hmod = GetModuleHandleW(None).ok();
        let hinst: Option<HINSTANCE> = hmod.map(Into::into);
        let hook = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(key_proc), hinst, 0) {
            Ok(h) => h,
            Err(e) => return drop(tid_tx.send(Err(e.to_string()))),
        };
        let class = w!("teach_spike_hotkeys");
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinst.unwrap_or_default(),
            lpszClassName: class,
            ..Default::default()
        };
        RegisterClassExW(&wc);
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0), class, w!(""), WINDOW_STYLE(0), 0, 0, 0, 0,
            Some(HWND_MESSAGE), None, hinst, None,
        );
        let registered = match &hwnd {
            Ok(h) => RegisterHotKey(Some(*h), 1, MOD_CONTROL | MOD_ALT | MOD_NOREPEAT, VK_ESCAPE.0 as u32)
                .map_err(|e| eprintln!("CẢNH BÁO: RegisterHotKey(Ctrl+Alt+Esc) lỗi: {e}"))
                .is_ok(),
            Err(e) => {
                eprintln!("CẢNH BÁO: CreateWindowExW(HWND_MESSAGE) lỗi: {e}; không có đường wm_hotkey");
                false
            }
        };
        let _ = tid_tx.send(Ok(GetCurrentThreadId()));
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        if let (true, Ok(h)) = (registered, &hwnd) {
            let _ = UnregisterHotKey(Some(*h), 1);
        }
        if let Ok(h) = hwnd {
            let _ = DestroyWindow(h);
        }
        let _ = UnhookWindowsHookEx(hook);
    });
    let tid = match tid_rx.recv() {
        Ok(Ok(t)) => t,
        r => {
            eprintln!("cài hook lỗi: {r:?}");
            std::process::exit(1);
        }
    };
    eprintln!("sẵn sàng (esc-passthru={passthru}). Bấm Ctrl+Alt+Esc/P/S; Ctrl+C để dừng.");

    let end = (secs > 0).then(|| Instant::now() + Duration::from_secs(secs));
    let mut counts = [[0u64; 3]; 2]; // [hook, wm_hotkey][combo]
    let mut injected = 0u64;
    while !STOP.load(SeqCst) && end.is_none_or(|e| Instant::now() < e) {
        if let Ok((i, path, t_us, inj)) = rx.recv_timeout(Duration::from_millis(50)) {
            let p = (path == "wm_hotkey") as usize;
            counts[p][i] += 1;
            injected += inj as u64;
            println!(
                "{} {:<9} {:<12} #{} (hook->in {} us, injected={})",
                chrono::Local::now().format("%H:%M:%S%.3f"),
                path, NAMES[i], counts[p][i], now_us() - t_us, inj
            );
        }
    }
    STOP.store(true, SeqCst);
    unsafe {
        let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
    }
    let _ = hook_t.join();
    println!("--- summary ---");
    for i in 0..3 {
        println!("{:<12} hook={} wm_hotkey={}", NAMES[i], counts[0][i], counts[1][i]);
    }
    println!(
        "hook swallowed: down={} up={} injected_combos={} esc_passthru={}",
        SWALLOWED_DOWN.load(SeqCst), SWALLOWED_UP.load(SeqCst), injected, passthru
    );
}
