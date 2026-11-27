//! T089 / quickstart M0 row 0.14 — spike: LL hook loss + recorder lag_ms (plan D-2).
//!
//! Run (unattended, injects F24 key presses; F24 is used by no app):
//!   cargo run --example spike_hookloss -- --mode force [--duration 15] [--threshold-ms 250]
//!   cargo run --example spike_hookloss -- --mode lag --synthetic-clicks 40 --rate 20 [--uia-ms 200] [--real-uia]
//!
//! Do NOT touch keyboard/mouse during the run (it would pollute counts). Registry is read only.
//!
//! `force`: hook thread (message loop + message-only window) installs WH_KEYBOARD_LL + WH_MOUSE_LL and
//!   RegisterHotKey(Ctrl+Alt+Esc). After 3 s the hook proc sleeps (LowLevelHooksTimeout + 500 ms) on
//!   the next 3 F24 events so Windows drops the hook. Monitor (100 ms) compares GetLastInputInfo with
//!   the last hook event tick (mismatch > threshold = "hook lost"). Reinstaller re-installs the hooks
//!   every 1 s. While lost, the injector sends Ctrl+Alt+Esc; WM_HOTKEY must still arrive.
//!   Read off: detect_ms (forced sleep start -> detection), recover_ms (detection -> first hook event),
//!   hotkey_during_loss=k/n (want n/n), missed events.
//! `lag`: hook pushes events to a channel; a processing thread does a fake slow UIA lookup (--uia-ms,
//!   default 200; --real-uia also calls uiautomation element_from_point). lag_ms = processing start -
//!   event time. Read off: p50/p90/p99/max, `uncertain` counts at limits 100/250/500/1000 ms, focus
//!   changes -> choose RECORD_LAG_LIMIT_MS.
//! Last line: `0.14: detect_ms=.. recover_ms=.. hotkey_during_loss=k/n lag p99=.. suggest RECORD_LAG_LIMIT_MS=..`

use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, AtomicU64, Ordering::SeqCst};
use std::sync::{mpsc, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use windows::core::w;
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemInformation::GetTickCount;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

const VK_F24_: u16 = 0x87;
const WM_REINSTALL: u32 = WM_APP + 1;

struct Ev {
    t: Instant,
    fg: isize,
    x: i32,
    y: i32,
}

static START: OnceLock<Instant> = OnceLock::new();
static TX: OnceLock<mpsc::Sender<Ev>> = OnceLock::new();
static LAST_HOOK_MS: AtomicU64 = AtomicU64::new(0);
static LAST_HOOK_TICK: AtomicU32 = AtomicU32::new(0);
static HOOK_EVENTS: AtomicU64 = AtomicU64::new(0);
static F24_SEEN: AtomicU64 = AtomicU64::new(0);
static F24_SENT: AtomicU64 = AtomicU64::new(0);
static FORCE_REMAIN: AtomicU32 = AtomicU32::new(0);
static FORCE_SLEEP_MS: AtomicU32 = AtomicU32::new(0);
static FORCED_START_MS: AtomicU64 = AtomicU64::new(0);
static HK_TOTAL: AtomicU64 = AtomicU64::new(0);
static HK_LOSS: AtomicU64 = AtomicU64::new(0);
static HK_SENT_LOSS: AtomicU64 = AtomicU64::new(0);
static LOST: AtomicBool = AtomicBool::new(false);
static KB_HOOK: AtomicIsize = AtomicIsize::new(0);
static MS_HOOK: AtomicIsize = AtomicIsize::new(0);

fn ms() -> u64 {
    START.get().unwrap().elapsed().as_millis() as u64
}

fn note_event(down: bool, f24: bool) {
    LAST_HOOK_MS.store(ms(), SeqCst);
    LAST_HOOK_TICK.store(unsafe { GetTickCount() }, SeqCst);
    HOOK_EVENTS.fetch_add(1, SeqCst);
    if down {
        if f24 {
            F24_SEEN.fetch_add(1, SeqCst);
        }
        if let Some(tx) = TX.get() {
            let mut p = POINT::default();
            unsafe {
                let _ = GetCursorPos(&mut p);
                let _ = tx.send(Ev { t: Instant::now(), fg: GetForegroundWindow().0 as isize, x: p.x, y: p.y });
            }
        }
    }
    if down
        && f24
        && FORCE_REMAIN.try_update(SeqCst, SeqCst, |n| n.checked_sub(1)).is_ok()
    {
        FORCED_START_MS.store(ms(), SeqCst);
        thread::sleep(Duration::from_millis(FORCE_SLEEP_MS.load(SeqCst) as u64));
    }
}

unsafe extern "system" fn kb_proc(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code >= 0 {
        let k = unsafe { &*(lp.0 as *const KBDLLHOOKSTRUCT) };
        let down = matches!(wp.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
        note_event(down, k.vkCode == VK_F24_ as u32);
    }
    unsafe { CallNextHookEx(None, code, wp, lp) }
}

unsafe extern "system" fn ms_proc(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code >= 0 {
        let down = matches!(wp.0 as u32, WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN);
        note_event(down, false);
    }
    unsafe { CallNextHookEx(None, code, wp, lp) }
}

fn install_hooks() {
    unsafe {
        let h = GetModuleHandleW(None).unwrap();
        let k = SetWindowsHookExW(WH_KEYBOARD_LL, Some(kb_proc), Some(h.into()), 0);
        let m = SetWindowsHookExW(WH_MOUSE_LL, Some(ms_proc), Some(h.into()), 0);
        KB_HOOK.store(k.as_ref().map(|x| x.0 as isize).unwrap_or(0), SeqCst);
        MS_HOOK.store(m.as_ref().map(|x| x.0 as isize).unwrap_or(0), SeqCst);
        if k.is_err() || m.is_err() {
            println!("[{}ms] SetWindowsHookExW failed: kb={:?} ms={:?}", ms(), k.err(), m.err());
        }
    }
}

fn uninstall_hooks() {
    unsafe {
        for a in [&KB_HOOK, &MS_HOOK] {
            let h = a.swap(0, SeqCst);
            if h != 0 {
                let _ = UnhookWindowsHookEx(HHOOK(h as *mut _)); // fails silently if Windows already removed it
            }
        }
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_HOTKEY => {
            let n = HK_TOTAL.fetch_add(1, SeqCst) + 1;
            let lost = LOST.load(SeqCst);
            if lost {
                HK_LOSS.fetch_add(1, SeqCst);
            }
            println!("[{}ms] WM_HOTKEY #{n} (hook_lost={lost})", ms());
            LRESULT(0)
        }
        WM_REINSTALL => {
            uninstall_hooks();
            install_hooks();
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wp, lp) },
    }
}

fn hook_thread(ready: mpsc::Sender<(u32, isize)>) {
    unsafe {
        let hinst: HINSTANCE = GetModuleHandleW(None).unwrap().into();
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinst,
            lpszClassName: w!("spike_hookloss"),
            ..Default::default()
        };
        RegisterClassW(&wc);
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0), w!("spike_hookloss"), w!(""), WINDOW_STYLE(0),
            0, 0, 0, 0, Some(HWND_MESSAGE), None, Some(hinst), None,
        )
        .unwrap();
        if let Err(e) = RegisterHotKey(Some(hwnd), 1, MOD_CONTROL | MOD_ALT | MOD_NOREPEAT, VK_ESCAPE.0 as u32) {
            println!("WARNING RegisterHotKey Ctrl+Alt+Esc failed: {e}");
        }
        install_hooks();
        ready.send((GetCurrentThreadId(), hwnd.0 as isize)).unwrap();
        let mut m = MSG::default();
        while GetMessageW(&mut m, None, 0, 0).as_bool() {
            DispatchMessageW(&m);
        }
        uninstall_hooks();
        let _ = UnregisterHotKey(Some(hwnd), 1);
        let _ = DestroyWindow(hwnd);
    }
}

fn key(vk: u16, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: 0,
                dwFlags: if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn send(v: &[INPUT]) {
    unsafe { SendInput(v, std::mem::size_of::<INPUT>() as i32) };
}

fn send_f24() {
    send(&[key(VK_F24_, false), key(VK_F24_, true)]);
    F24_SENT.fetch_add(1, SeqCst);
}

fn send_hotkey() {
    send(&[
        key(VK_CONTROL.0, false), key(VK_MENU.0, false), key(VK_ESCAPE.0, false),
        key(VK_ESCAPE.0, true), key(VK_MENU.0, true), key(VK_CONTROL.0, true),
    ]);
}

fn last_input_tick() -> u32 {
    let mut li = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
    unsafe {
        let _ = GetLastInputInfo(&mut li);
    }
    li.dwTime
}

fn hooks_timeout_ms() -> u32 {
    let out = std::process::Command::new("reg")
        .args(["query", r"HKCU\Control Panel\Desktop", "/v", "LowLevelHooksTimeout"])
        .output();
    let parsed = out.ok().and_then(|o| {
        let s = String::from_utf8_lossy(&o.stdout).to_string();
        let tok = s.split_whitespace().last()?.to_string();
        match tok.strip_prefix("0x") {
            Some(h) => u32::from_str_radix(h, 16).ok(),
            None => tok.parse().ok(),
        }
    });
    match parsed {
        Some(v) => {
            println!("LowLevelHooksTimeout = {v} ms (HKCU, read only)");
            v
        }
        None => {
            println!("LowLevelHooksTimeout absent -> assume default 300 ms");
            300
        }
    }
}

fn arg<T: std::str::FromStr>(args: &[String], name: &str, default: T) -> T {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn pct(v: &[u64], p: f64) -> u64 {
    if v.is_empty() {
        return 0;
    }
    v[((p * v.len() as f64).ceil() as usize).clamp(1, v.len()) - 1]
}

/// Returns (max detect_ms, max recover_ms, cycles).
fn run_force(args: &[String], hwnd: isize, timeout: u32) -> (u64, u64, u32) {
    let duration: u64 = arg(args, "--duration", 15);
    let thr: i32 = arg(args, "--threshold-ms", 250);
    FORCE_SLEEP_MS.store(timeout + 500, SeqCst);
    let stop = std::sync::Arc::new(AtomicBool::new(false));

    // injector: F24 at 10/s; Ctrl+Alt+Esc every 250 ms while hook is lost; arms the forced sleeps at 3 s
    let s = stop.clone();
    let inj = thread::spawn(move || {
        let mut last_hk = Instant::now();
        let mut armed = false;
        while !s.load(SeqCst) {
            if !armed && ms() >= 3000 {
                armed = true;
                FORCE_REMAIN.store(3, SeqCst);
                println!("[{}ms] armed: next 3 F24 events will stall the hook proc {} ms", ms(), timeout + 500);
            }
            send_f24();
            if LOST.load(SeqCst) && last_hk.elapsed() >= Duration::from_millis(250) {
                last_hk = Instant::now();
                HK_SENT_LOSS.fetch_add(1, SeqCst);
                send_hotkey();
            }
            thread::sleep(Duration::from_millis(100));
        }
    });
    // reinstaller: every 1 s, executed on the hook thread via posted message
    let s = stop.clone();
    let rein = thread::spawn(move || {
        while !s.load(SeqCst) {
            thread::sleep(Duration::from_secs(1));
            unsafe {
                let _ = PostMessageW(Some(HWND(hwnd as *mut _)), WM_REINSTALL, WPARAM(0), LPARAM(0));
            }
        }
    });
    // monitor
    let s = stop.clone();
    let mon = thread::spawn(move || {
        let (mut det_max, mut rec_max, mut cycles) = (0u64, 0u64, 0u32);
        let (mut lost_at, mut det) = (0u64, 0u64);
        let (mut prev_li, mut prev_ev, mut missed_polls) = (last_input_tick(), HOOK_EVENTS.load(SeqCst), 0u64);
        while !s.load(SeqCst) {
            thread::sleep(Duration::from_millis(100));
            let li = last_input_tick();
            let ev = HOOK_EVENTS.load(SeqCst);
            let lost = LOST.load(SeqCst);
            if li != prev_li && ev == prev_ev {
                missed_polls += 1;
            }
            prev_li = li;
            prev_ev = ev;
            let diff = li.wrapping_sub(LAST_HOOK_TICK.load(SeqCst)) as i32;
            if !lost && diff > thr {
                det = ms().saturating_sub(FORCED_START_MS.load(SeqCst));
                lost_at = ms();
                LOST.store(true, SeqCst);
                println!("[{}ms] hook lost detected (input-hook tick diff {diff} ms): detect_ms={det}", lost_at);
            } else if lost && LAST_HOOK_MS.load(SeqCst) > lost_at {
                let rec = LAST_HOOK_MS.load(SeqCst) - lost_at;
                LOST.store(false, SeqCst);
                cycles += 1;
                det_max = det_max.max(det);
                rec_max = rec_max.max(rec);
                println!("[{}ms] hook recovered: recover_ms={rec} (detect_ms={det})", ms());
            }
        }
        println!("polls with input change but no hook event: {missed_polls}");
        (det_max, rec_max, cycles)
    });

    thread::sleep(Duration::from_secs(duration));
    stop.store(true, SeqCst);
    inj.join().unwrap();
    rein.join().unwrap();
    let r = mon.join().unwrap();
    println!(
        "F24 injected={} seen_by_hook={} missed={}",
        F24_SENT.load(SeqCst), F24_SEEN.load(SeqCst),
        F24_SENT.load(SeqCst).saturating_sub(F24_SEEN.load(SeqCst))
    );
    r
}

/// Returns p99.
fn run_lag(args: &[String]) -> u64 {
    let n: u32 = arg(args, "--synthetic-clicks", 40);
    let rate: f64 = arg(args, "--rate", 20.0);
    let uia_ms: u64 = arg(args, "--uia-ms", 200);
    let real = args.iter().any(|a| a == "--real-uia");
    // TX must be set before hooks see events; hook thread is already up but F24 is only sent below
    let (tx, rx) = mpsc::channel::<Ev>();
    TX.set(tx).ok();
    let done = std::sync::Arc::new(AtomicBool::new(false));
    let d = done.clone();
    let proc_t = thread::spawn(move || {
        let auto = if real { uiautomation::UIAutomation::new().ok() } else { None };
        let (mut lags, mut focus) = (Vec::<u64>::new(), 0u32);
        loop {
            match rx.recv_timeout(Duration::from_millis(300)) {
                Ok(ev) => {
                    lags.push(ev.t.elapsed().as_millis() as u64);
                    if unsafe { GetForegroundWindow().0 as isize } != ev.fg {
                        focus += 1;
                    }
                    if let Some(a) = &auto {
                        let _ = a.element_from_point(uiautomation::types::Point::new(ev.x, ev.y));
                    }
                    thread::sleep(Duration::from_millis(uia_ms));
                }
                Err(_) if d.load(SeqCst) => break,
                Err(_) => {}
            }
        }
        (lags, focus)
    });
    let t0 = Instant::now();
    for i in 0..n {
        send_f24();
        let next = t0 + Duration::from_secs_f64((i + 1) as f64 / rate);
        thread::sleep(next.saturating_duration_since(Instant::now()));
    }
    thread::sleep(Duration::from_millis(300));
    done.store(true, SeqCst);
    let (mut lags, focus) = proc_t.join().unwrap();
    lags.sort();
    println!("sent={} hook_seen={} processed={} uia_ms={uia_ms} real_uia={real}", F24_SENT.load(SeqCst), F24_SEEN.load(SeqCst), lags.len());
    let (p99, mx) = (pct(&lags, 0.99), lags.last().copied().unwrap_or(0));
    println!("lag_ms p50={} p90={} p99={p99} max={mx}", pct(&lags, 0.5), pct(&lags, 0.9));
    let mut suggest = None;
    for lim in [100u64, 250, 500, 1000] {
        let u = lags.iter().filter(|&&l| l > lim).count();
        println!("limit {lim:>4} ms: uncertain {u}/{} ({:.0}%)", lags.len(), 100.0 * u as f64 / lags.len().max(1) as f64);
        if suggest.is_none() && u * 20 <= lags.len() {
            suggest = Some(lim); // ponytail: first candidate with <=5% uncertain; judgement left to the operator
        }
    }
    println!("focus changed between event and processing: {focus}");
    LAG_SUGGEST.store(suggest.unwrap_or(0), SeqCst);
    p99
}

static LAG_SUGGEST: AtomicU64 = AtomicU64::new(0);

fn main() {
    START.set(Instant::now()).ok();
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    let args: Vec<String> = std::env::args().collect();
    let mode: String = arg(&args, "--mode", "force".to_string());
    let timeout = hooks_timeout_ms();

    let (rtx, rrx) = mpsc::channel();
    let ht = thread::spawn(move || hook_thread(rtx));
    let (tid, hwnd) = rrx.recv().unwrap();
    println!("[{}ms] hook thread up, mode={mode}", ms());

    let (mut det, mut rec, mut p99) = (None, None, None);
    match mode.as_str() {
        "force" => {
            let (d, r, c) = run_force(&args, hwnd, timeout);
            println!("cycles={c}");
            if c > 0 {
                det = Some(d);
                rec = Some(r);
            }
        }
        "lag" => p99 = Some(run_lag(&args)),
        m => println!("unknown --mode {m} (force|lag)"),
    }
    unsafe {
        let _ = PostThreadMessageW(tid, WM_QUIT, WPARAM(0), LPARAM(0));
    }
    ht.join().unwrap();
    let f = |o: Option<u64>| o.map_or("n/a".to_string(), |v| v.to_string());
    let sug = match (p99, LAG_SUGGEST.load(SeqCst)) {
        (Some(_), 0) => ">1000".to_string(),
        (Some(_), s) => s.to_string(),
        _ => "n/a".to_string(),
    };
    println!(
        "0.14: detect_ms={} recover_ms={} hotkey_during_loss={}/{} lag p99={} suggest RECORD_LAG_LIMIT_MS={sug}",
        f(det), f(rec), HK_LOSS.load(SeqCst), HK_SENT_LOSS.load(SeqCst), f(p99)
    );
}
