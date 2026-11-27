//! T010 spike: phím tắt toàn cục qua hook LL (nuốt tổ hợp) + RegisterHotKey. Chỉ đo, không phải code production.
//!
//! Đo quickstart §M0 0.8: nhận 20/20 lần Ctrl+Alt+Esc, Ctrl+Alt+P, Ctrl+Alt+S khi cửa sổ khác
//! đang focus (kể cả ứng dụng toàn màn hình) và tổ hợp không lọt vào ứng dụng đích.
//!
//! Chạy: `cargo run --example spike_hotkeys -- [giây] [--no-hotkey] [--self-test N]`
//! (giây mặc định 0 = chạy tới Ctrl+C).
//!
//! Người đo: focus Notepad / game / video toàn màn hình, bấm mỗi tổ hợp 20 lần, xem bộ đếm
//! mỗi dòng và `summary` khi thoát. Kiểm tra bằng mắt: ứng dụng đích không nhận phím nào
//! (Notepad không gõ ra "p"/"s", không mở menu/Start). Tổ hợp bị nuốt cả key down lẫn key up.
//!   - đường `hook`: hook LL. `wm_hotkey`: RegisterHotKey trên cửa sổ message-only ở LUỒNG RIÊNG
//!     (plan D-2: hook kẹt không được kéo đường dừng thứ hai kẹt theo).
//!   - Hook LL chạy TRƯỚC RegisterHotKey, nên khi RegisterHotKey thành công hook KHÔNG nuốt
//!     Ctrl+Alt+Esc (chỉ quan sát); hệ thống tự giữ phím tắt đã đăng ký khỏi ứng dụng đích.
//!     RegisterHotKey thất bại (hoặc `--no-hotkey`) → hook nuốt như P/S. Mỗi lần nhấn phải tăng
//!     cả `hook` và `wm_hotkey`; người đo kiểm Esc không lọt vào ứng dụng đích ở cả hai chế độ.
//!   - `--self-test N`: tự gửi N lần Ctrl+Alt+Esc bằng SendInput (cửa sổ đang focus nhận Esc nếu
//!     lọt) rồi in `self-test: hook=a/N wm_hotkey=b/N`.

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

fn send_combo() {
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    let key = |vk: VIRTUAL_KEY, up: bool| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT { wVk: vk, dwFlags: if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) }, ..Default::default() },
        },
    };
    let seq = [
        key(VK_CONTROL, false), key(VK_MENU, false), key(VK_ESCAPE, false),
        key(VK_ESCAPE, true), key(VK_MENU, true), key(VK_CONTROL, true),
    ];
    unsafe { SendInput(&seq, size_of::<INPUT>() as i32) };
}

unsafe extern "system" fn on_ctrl(_: u32) -> BOOL {
    STOP.store(true, SeqCst);
    BOOL(1)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let secs: u64 = args.first().and_then(|a| a.parse().ok()).unwrap_or(0);
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let _ = SetConsoleCtrlHandler(Some(on_ctrl), true);
    }
    now_us();
    let (tx, rx) = mpsc::channel();
    TX.set(tx).ok();

    let no_hotkey = args.iter().any(|a| a == "--no-hotkey");
    let self_test: u32 = args.windows(2).find(|w| w[0] == "--self-test").and_then(|w| w[1].parse().ok()).unwrap_or(0);

    // Luồng phím tắt: cửa sổ message-only + RegisterHotKey, tách khỏi luồng hook (D-2).
    let (hk_tx, hk_rx) = mpsc::channel::<(u32, bool)>();
    let hk_t = std::thread::spawn(move || unsafe {
        let hinst: Option<HINSTANCE> = GetModuleHandleW(None).ok().map(Into::into);
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
            Ok(_) if no_hotkey => false,
            Ok(h) => RegisterHotKey(Some(*h), 1, MOD_CONTROL | MOD_ALT | MOD_NOREPEAT, VK_ESCAPE.0 as u32)
                .map_err(|e| eprintln!("CẢNH BÁO: RegisterHotKey(Ctrl+Alt+Esc) lỗi: {e}; hook sẽ nuốt tổ hợp"))
                .is_ok(),
            Err(e) => {
                eprintln!("CẢNH BÁO: CreateWindowExW(HWND_MESSAGE) lỗi: {e}; không có đường wm_hotkey");
                false
            }
        };
        let _ = hk_tx.send((GetCurrentThreadId(), registered));
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
    });
    let (hk_tid, registered) = hk_rx.recv().expect("luồng phím tắt");
    // Đã đăng ký → hệ thống giữ tổ hợp khỏi ứng dụng đích; hook chỉ quan sát để WM_HOTKEY tới được.
    PASSTHRU_ESC.store(registered, SeqCst);
    let passthru = registered;

    let (tid_tx, tid_rx) = mpsc::channel::<Result<u32, String>>();
    let hook_t = std::thread::spawn(move || unsafe {
        let hinst: Option<HINSTANCE> = GetModuleHandleW(None).ok().map(Into::into);
        let hook = match SetWindowsHookExW(WH_KEYBOARD_LL, Some(key_proc), hinst, 0) {
            Ok(h) => h,
            Err(e) => return drop(tid_tx.send(Err(e.to_string()))),
        };
        let _ = tid_tx.send(Ok(GetCurrentThreadId()));
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
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
    if self_test > 0 {
        std::thread::spawn(move || {
            for _ in 0..self_test {
                std::thread::sleep(Duration::from_millis(150));
                send_combo();
            }
        });
    }
    eprintln!("sẵn sàng (hotkey_registered={registered}). Bấm Ctrl+Alt+Esc/P/S; Ctrl+C để dừng.");

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
    unsafe {
        let _ = PostThreadMessageW(hk_tid, WM_QUIT, WPARAM(0), LPARAM(0));
    }
    let _ = hk_t.join();
    println!("--- summary ---");
    for i in 0..3 {
        println!("{:<12} hook={} wm_hotkey={}", NAMES[i], counts[0][i], counts[1][i]);
    }
    println!(
        "hook swallowed: down={} up={} injected_combos={} esc_passthru={}",
        SWALLOWED_DOWN.load(SeqCst), SWALLOWED_UP.load(SeqCst), injected, passthru
    );
    if self_test > 0 {
        println!("self-test: hook={}/{self_test} wm_hotkey={}/{self_test}", counts[0][0], counts[1][0]);
    }
}
