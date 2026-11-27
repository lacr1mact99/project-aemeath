//! T090 / quickstart M0 row 0.11 — spike: session change + secure desktop detection.
//!
//! Run: cargo run --example spike_session -- [--simulate run|record] [--secs N]
//!   (--simulate only labels the output; --secs 0 (default) = run until Ctrl+C)
//!
//! Operator procedure for 0.11 (do each case 5 times, once with `--simulate record`, once with `--simulate run`):
//!   1. Lock (Win+L) / unlock x5.
//!   2. Trigger UAC x5 (e.g. start an elevated app / `Start-Process cmd -Verb RunAs`), then accept/cancel.
//!   3. RDP: disconnect / reconnect x5 (run this example INSIDE the RDP session).
//!   4. Minimize the RDP client window x5 — must be done from the OTHER machine (the client side).
//! Read off per case: WM_WTSSESSION_CHANGE reason + timestamp, desktop transitions (Default <-> Winlogon /
//! ERR) with latency vs the nearest WTS event, SM_REMOTESESSION changes. Pass = every lock/UAC/RDP-disconnect
//! is detected (by WTS event or desktop poll) before the next action; minimized-RDP case: just record what shows.

use std::sync::atomic::{AtomicIsize, AtomicU64, Ordering::SeqCst};
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, Instant};

use windows::core::{w, PWSTR};
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::RemoteDesktop::{
    WTSRegisterSessionNotification, WTSUnRegisterSessionNotification, NOTIFY_FOR_THIS_SESSION,
};
use windows::Win32::System::StationsAndDesktops::{
    CloseDesktop, GetUserObjectInformationW, OpenInputDesktop, DESKTOP_CONTROL_FLAGS, DESKTOP_READOBJECTS,
    UOI_NAME,
};
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::WindowsAndMessaging::*;

static START: OnceLock<Instant> = OnceLock::new();
static LAST_WTS_MS: AtomicU64 = AtomicU64::new(0);
static LAST_DESK_MS: AtomicU64 = AtomicU64::new(0);
static LABEL: OnceLock<String> = OnceLock::new();
static HWND_RAW: AtomicIsize = AtomicIsize::new(0);

fn ms() -> u64 {
    START.get().unwrap().elapsed().as_millis() as u64
}

fn log(s: String) {
    println!("[{}] [{:>6}ms] {s}", LABEL.get().unwrap(), ms());
}

fn reason(w: usize) -> &'static str {
    match w {
        1 => "WTS_CONSOLE_CONNECT",
        2 => "WTS_CONSOLE_DISCONNECT",
        3 => "WTS_REMOTE_CONNECT",
        4 => "WTS_REMOTE_DISCONNECT",
        5 => "WTS_SESSION_LOGON",
        6 => "WTS_SESSION_LOGOFF",
        7 => "WTS_SESSION_LOCK",
        8 => "WTS_SESSION_UNLOCK",
        9 => "WTS_SESSION_REMOTE_CONTROL",
        10 => "WTS_SESSION_CREATE",
        11 => "WTS_SESSION_TERMINATE",
        _ => "UNKNOWN",
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_WTSSESSION_CHANGE => {
            let now = ms();
            LAST_WTS_MS.store(now, SeqCst);
            let d = now.saturating_sub(LAST_DESK_MS.load(SeqCst));
            log(format!("WM_WTSSESSION_CHANGE {} ({}) session_id={} (+{d}ms after last desktop transition)", reason(wp.0), wp.0, lp.0));
            LRESULT(0)
        }
        WM_CLOSE => {
            let _ = unsafe { DestroyWindow(hwnd) };
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe {
                let _ = WTSUnRegisterSessionNotification(hwnd);
                PostQuitMessage(0);
            }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wp, lp) },
    }
}

/// Name of the current input desktop, or "ERR(code)" when it cannot be opened (typically Winlogon/secure desktop).
fn input_desktop() -> String {
    unsafe {
        match OpenInputDesktop(DESKTOP_CONTROL_FLAGS(0), false, DESKTOP_READOBJECTS) {
            Err(e) => format!("ERR({:#x})", e.code().0),
            Ok(d) => {
                let mut buf = [0u16; 128];
                let mut need = 0u32;
                let r = GetUserObjectInformationW(
                    HANDLE(d.0),
                    UOI_NAME,
                    Some(buf.as_mut_ptr() as *mut _),
                    (buf.len() * 2) as u32,
                    Some(&mut need),
                );
                let _ = CloseDesktop(d);
                match r {
                    Ok(()) => PWSTR(buf.as_mut_ptr()).to_string().unwrap_or_default(),
                    Err(e) => format!("NAME_ERR({:#x})", e.code().0),
                }
            }
        }
    }
}

fn poll_thread() {
    let mut desk = input_desktop();
    let mut remote = unsafe { GetSystemMetrics(SM_REMOTESESSION) };
    log(format!("initial desktop={desk} SM_REMOTESESSION={remote}"));
    loop {
        thread::sleep(Duration::from_millis(250));
        let now = ms();
        let d = input_desktop();
        if d != desk {
            let since = now.saturating_sub(LAST_WTS_MS.load(SeqCst));
            log(format!("desktop {desk} -> {d} (poll 250ms; +{since}ms after last WTS event)"));
            desk = d;
            LAST_DESK_MS.store(now, SeqCst);
        }
        let r = unsafe { GetSystemMetrics(SM_REMOTESESSION) };
        if r != remote {
            log(format!("SM_REMOTESESSION {remote} -> {r}"));
            remote = r;
        }
    }
}

fn main() {
    START.set(Instant::now()).ok();
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    let args: Vec<String> = std::env::args().collect();
    let val = |n: &str| args.iter().position(|a| a == n).and_then(|i| args.get(i + 1)).cloned();
    LABEL.set(val("--simulate").unwrap_or_else(|| "unlabeled".into())).ok();
    let secs: u64 = val("--secs").and_then(|s| s.parse().ok()).unwrap_or(0);

    unsafe {
        let hinst: HINSTANCE = GetModuleHandleW(None).unwrap().into();
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinst,
            lpszClassName: w!("spike_session"),
            ..Default::default()
        };
        RegisterClassW(&wc);
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0), w!("spike_session"), w!(""), WINDOW_STYLE(0),
            0, 0, 0, 0, Some(HWND_MESSAGE), None, Some(hinst), None,
        )
        .unwrap();
        HWND_RAW.store(hwnd.0 as isize, SeqCst);
        match WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION) {
            Ok(()) => log("WTSRegisterSessionNotification OK".into()),
            Err(e) => log(format!("WTSRegisterSessionNotification FAILED: {e}")),
        }
        thread::spawn(poll_thread);
        if secs > 0 {
            thread::spawn(move || {
                thread::sleep(Duration::from_secs(secs));
                let _ = PostMessageW(Some(HWND(HWND_RAW.load(SeqCst) as *mut _)), WM_CLOSE, WPARAM(0), LPARAM(0));
            });
        }
        let mut m = MSG::default();
        while GetMessageW(&mut m, None, 0, 0).as_bool() {
            DispatchMessageW(&m);
        }
    }
    log("exit".into());
}
