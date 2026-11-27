//! `spike_stop` (T088, quickstart 0.13): do dung khi loi goi treo (plan D-1) tren `bench_target`.
//!
//! Chay (bench_target phai dang chay, cua so khong bi che):
//!   cargo run -p teach-core --example bench_target            (cua so 1)
//!   cargo run -p teach-core --example spike_stop -- --case invoke --runs 5 --auto-stop-after 1000
//!
//! Tham so: --case uia_read|invoke|setvalue|sendinput  --runs N (5)  --hang-ms (8000)
//!          --auto-stop-after <ms> (dung tu luong hen gio thay vi phim; thieu thi bam Ctrl+Alt+Esc)
//!          --uia-timeout <ms>  dat IUIAutomation2 ConnectionTimeout/TransactionTimeout
//! Ca `sendinput` di chuot THAT toi nut Luu (cua so bench_target duoc dua len truoc).
//! Moi lan: reset -> worker tim phan tu/pattern -> `hang_ui` -> worker goi lien tuc qua Gate ->
//! dung (hook LL + RegisterHotKey tren cua so message-only, hoac timer) -> do state sau khi het treo.

use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use uiautomation::patterns::{UIInvokePattern, UIValuePattern};
use uiautomation::types::{TreeScope, UIProperty};
use uiautomation::variants::Variant;
use uiautomation::{UIAutomation, UIElement};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};
use windows::Win32::UI::Accessibility::{CUIAutomation8, IUIAutomation, IUIAutomation2};
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

const TEACH_MAGIC: usize = 0x7EAC_4A5C;
const PIPE: &str = r"\\.\pipe\teach-bench-target";
const TITLE: &str = "Teach Bench Target";

// ---------------------------------------------------------------- Gate (ban thu cua platform/input.rs)

#[derive(Default)]
struct GateState {
    stopped: bool,
    flag_at: Option<Instant>, // luc co dung duoc bat, ghi duoi mutex
    press_at: Option<Instant>, // luc nhan phim / timer
    late_effect_possible: bool,
    sent: Vec<(&'static str, Instant)>,
    held: HashSet<u16>,
    in_flight: Option<&'static str>,
}

struct Gate {
    inner: Mutex<GateState>,
}

enum Dispatch<R> {
    NotSent,
    Sent(R),
}

impl Gate {
    fn new() -> Self {
        Gate { inner: Mutex::new(GateState::default()) }
    }

    /// Kiem co dung -> ghi `sent` + `in_flight` duoi mutex -> nha mutex -> goi `f`.
    fn dispatch<R>(&self, op: &'static str, f: impl FnOnce() -> R) -> Dispatch<R> {
        {
            let mut g = self.inner.lock().unwrap();
            if g.stopped {
                return Dispatch::NotSent;
            }
            g.sent.push((op, Instant::now()));
            g.in_flight = Some(op);
        }
        let r = f();
        self.inner.lock().unwrap().in_flight = None;
        Dispatch::Sent(r)
    }

    fn stop(&self, press: Instant) {
        let mut g = self.inner.lock().unwrap();
        if g.stopped {
            return;
        }
        g.stopped = true;
        g.flag_at = Some(Instant::now());
        g.press_at = Some(press);
        g.late_effect_possible = g.in_flight.is_some();
    }

    fn hold(&self, vk: u16) {
        self.inner.lock().unwrap().held.insert(vk);
    }
    fn unhold(&self, vk: u16) {
        self.inner.lock().unwrap().held.remove(&vk);
    }

    /// Goi duoc tu luong hook: nha moi phim/nut dang giu bang SendInput key-up.
    fn release_all_held(&self) -> usize {
        let held: Vec<u16> = self.inner.lock().unwrap().held.drain().collect();
        for &vk in &held {
            send_inputs(&[if vk == 1 { mouse_input(MOUSEEVENTF_LEFTUP, 0, 0) } else { key_input(vk, true) }]);
        }
        held.len()
    }
}

// ---------------------------------------------------------------- SendInput

fn key_input(vk: u16, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: 0,
                dwFlags: if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) },
                time: 0,
                dwExtraInfo: TEACH_MAGIC,
            },
        },
    }
}

fn mouse_input(flags: MOUSE_EVENT_FLAGS, x: i32, y: i32) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT { dx: x, dy: y, mouseData: 0, dwFlags: flags, time: 0, dwExtraInfo: TEACH_MAGIC },
        },
    }
}

fn send_inputs(v: &[INPUT]) {
    unsafe { SendInput(v, std::mem::size_of::<INPUT>() as i32) };
}

/// Click chuot trai tai toa do man hinh (px vat ly; tien trinh DPI-aware V2).
fn click(gate: &Gate, x: i32, y: i32) {
    let (sw, sh) = unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
    let (ax, ay) = (x * 65535 / (sw - 1), y * 65535 / (sh - 1));
    send_inputs(&[
        mouse_input(MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE, ax, ay),
        mouse_input(MOUSEEVENTF_LEFTDOWN, 0, 0),
    ]);
    gate.hold(1);
    std::thread::sleep(Duration::from_millis(10));
    send_inputs(&[mouse_input(MOUSEEVENTF_LEFTUP, 0, 0)]);
    gate.unhold(1);
}

// ---------------------------------------------------------------- duong dung: hook LL + hotkey

static GATE: Mutex<Option<Arc<Gate>>> = Mutex::new(None);
static STOP_SRC: Mutex<&'static str> = Mutex::new("");

fn trigger_stop(src: &'static str) {
    let press = Instant::now();
    let g = GATE.lock().unwrap().clone();
    if let Some(g) = g {
        g.stop(press);
        let n = g.release_all_held();
        let mut s = STOP_SRC.lock().unwrap();
        if s.is_empty() {
            *s = src;
            println!("  [stop] source={src} released_held={n}");
        }
    }
}

unsafe extern "system" fn ll_proc(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code >= 0 && (wp.0 as u32 == WM_KEYDOWN || wp.0 as u32 == WM_SYSKEYDOWN) {
        let kb = unsafe { &*(lp.0 as *const KBDLLHOOKSTRUCT) };
        let down = |vk: VIRTUAL_KEY| unsafe { GetAsyncKeyState(vk.0 as i32) } as u16 & 0x8000 != 0;
        if kb.vkCode == VK_ESCAPE.0 as u32 && kb.dwExtraInfo != TEACH_MAGIC && down(VK_CONTROL) && down(VK_MENU) {
            trigger_stop("hook");
            return LRESULT(1);
        }
    }
    unsafe { CallNextHookEx(None, code, wp, lp) }
}

fn spawn_stop_paths() {
    let (tx, rx) = mpsc::channel::<String>();
    std::thread::spawn(move || unsafe {
        let hi = GetModuleHandleW(None).unwrap();
        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(ll_proc), Some(hi.into()), 0);
        let cls = [b'S' as u16, b'T' as u16, b'A' as u16, b'T' as u16, b'I' as u16, b'C' as u16, 0];
        let win = CreateWindowExW(
            WINDOW_EX_STYLE(0), PCWSTR(cls.as_ptr()), PCWSTR::null(), WINDOW_STYLE(0),
            0, 0, 0, 0, Some(HWND_MESSAGE), None, Some(hi.into()), None,
        );
        let hot = match &win {
            Ok(w) => RegisterHotKey(Some(*w), 1, MOD_CONTROL | MOD_ALT | MOD_NOREPEAT, VK_ESCAPE.0 as u32),
            Err(e) => Err(e.clone()),
        };
        let _ = tx.send(format!("hook={} hotkey={}", hook.is_ok(), hot.is_ok()));
        let mut m = MSG::default();
        while GetMessageW(&mut m, None, 0, 0).as_bool() {
            if m.message == WM_HOTKEY {
                trigger_stop("hotkey");
            }
            let _ = TranslateMessage(&m);
            DispatchMessageW(&m);
        }
    });
    println!("stop paths: {}", rx.recv().unwrap());
}

// ---------------------------------------------------------------- pipe toi bench_target

struct Pipe(BufReader<File>);

impl Pipe {
    fn connect() -> Pipe {
        let t0 = Instant::now();
        loop {
            match OpenOptions::new().read(true).write(true).open(PIPE) {
                Ok(f) => return Pipe(BufReader::new(f)),
                Err(e) if t0.elapsed() < Duration::from_secs(5) => {
                    let _ = e;
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(e) => panic!("khong mo duoc {PIPE}: {e} (bench_target dang chay?)"),
            }
        }
    }
    fn cmd(&mut self, line: &str) -> String {
        writeln!(self.0.get_mut(), "{line}").unwrap();
        let mut s = String::new();
        self.0.read_line(&mut s).unwrap();
        s.trim().to_string()
    }
    fn state(&mut self) -> (u64, String, String) {
        let v: serde_json::Value = serde_json::from_str(&self.cmd("state")).unwrap();
        (v["save_count"].as_u64().unwrap(), v["last_saved"].as_str().unwrap().into(), v["edit"].as_str().unwrap().into())
    }
}

// ---------------------------------------------------------------- worker

struct Call {
    dur_ms: u128,
    res: String,
}

fn find(auto: &UIAutomation, win: &UIElement, id: &str) -> Result<UIElement, String> {
    let c = auto.create_property_condition(UIProperty::AutomationId, Variant::from(id), None).map_err(|e| e.to_string())?;
    win.find_first(TreeScope::Descendants, &c).map_err(|e| format!("find {id}: {e}"))
}

fn worker(
    case: String,
    gate: Arc<Gate>,
    uia_timeout: Option<u32>,
    ready: mpsc::Sender<Result<String, String>>,
    go: mpsc::Receiver<()>,
) -> Vec<Call> {
    let mut calls = Vec::new();
    let prep = (|| -> Result<(UIAutomation, UIElement, String), String> {
        // CUIAutomation8 (khong phai CUIAutomation cua crate) moi co IUIAutomation2.
        let raw: IUIAutomation = unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            CoCreateInstance(&CUIAutomation8, None, CLSCTX_ALL).map_err(|e| e.to_string())?
        };
        let auto = UIAutomation::from(raw.clone());
        let a2: IUIAutomation2 = windows::core::Interface::cast(&raw).map_err(|e| e.to_string())?;
        let mut info = unsafe { format!("default conn={:?} trans={:?}", a2.ConnectionTimeout(), a2.TransactionTimeout()) };
        if let Some(ms) = uia_timeout {
            unsafe {
                a2.SetConnectionTimeout(ms).map_err(|e| e.to_string())?;
                a2.SetTransactionTimeout(ms).map_err(|e| e.to_string())?;
                info = format!("set {ms} ms -> conn={:?} trans={:?}", a2.ConnectionTimeout(), a2.TransactionTimeout());
            }
        }
        let t: Vec<u16> = TITLE.encode_utf16().chain(Some(0)).collect();
        let hwnd = unsafe { FindWindowW(PCWSTR::null(), PCWSTR(t.as_ptr())) }.map_err(|e| format!("FindWindow: {e}"))?;
        let win = auto.element_from_handle(hwnd.into()).map_err(|e| e.to_string())?;
        Ok((auto, win, info))
    })();
    let (auto, win, info) = match prep {
        Ok(x) => x,
        Err(e) => {
            let _ = ready.send(Err(e));
            return calls;
        }
    };
    // Pre-resolve truoc khi treo: tim phan tu + pattern.
    enum Op {
        Read(Vec<UIElement>),
        Invoke(UIInvokePattern),
        SetValue(UIValuePattern),
        Click(i32, i32),
    }
    let op = (|| -> Result<Op, String> {
        Ok(match case.as_str() {
            "uia_read" => Op::Read(vec![find(&auto, &win, "1001")?, find(&auto, &win, "1002")?, find(&auto, &win, "1004")?]),
            "invoke" => Op::Invoke(find(&auto, &win, "1002")?.get_pattern::<UIInvokePattern>().map_err(|e| e.to_string())?),
            "setvalue" => Op::SetValue(find(&auto, &win, "1001")?.get_pattern::<UIValuePattern>().map_err(|e| e.to_string())?),
            "sendinput" => {
                let r = find(&auto, &win, "1002")?.get_bounding_rectangle().map_err(|e| e.to_string())?;
                Op::Click((r.get_left() + r.get_right()) / 2, (r.get_top() + r.get_bottom()) / 2)
            }
            c => return Err(format!("case la {c}")),
        })
    })();
    let op = match op {
        Ok(o) => o,
        Err(e) => {
            let _ = ready.send(Err(e));
            return calls;
        }
    };
    let _ = ready.send(Ok(info));
    if go.recv().is_err() {
        return calls;
    }
    loop {
        let r = match &op {
            Op::Read(els) => gate.dispatch("uia_read", || {
                let t = Instant::now();
                let res = els.iter().map(|e| e.get_name().map_err(|e| e.to_string())).collect::<Result<Vec<_>, _>>();
                (t, res.map(|v| format!("{v:?}")).unwrap_or_else(|e| format!("ERR {e}")))
            }),
            Op::Invoke(p) => gate.dispatch("invoke", || {
                let t = Instant::now();
                (t, p.invoke().map(|_| "ok".to_string()).unwrap_or_else(|e| format!("ERR {e}")))
            }),
            Op::SetValue(p) => gate.dispatch("setvalue", || {
                let t = Instant::now();
                (t, p.set_value("TEACH").map(|_| "ok".to_string()).unwrap_or_else(|e| format!("ERR {e}")))
            }),
            Op::Click(x, y) => gate.dispatch("sendinput", || {
                let t = Instant::now();
                click(&gate, *x, *y);
                (t, "ok".to_string())
            }),
        };
        match r {
            Dispatch::NotSent => break,
            Dispatch::Sent((t, res)) => calls.push(Call { dur_ms: t.elapsed().as_millis(), res }),
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    calls
}

// ---------------------------------------------------------------- main

fn bring_front() -> bool {
    let t: Vec<u16> = TITLE.encode_utf16().chain(Some(0)).collect();
    unsafe {
        let Ok(h) = FindWindowW(PCWSTR::null(), PCWSTR(t.as_ptr())) else { return false };
        let _ = ShowWindow(h, SW_RESTORE);
        // thao tac ALT de Windows cho phep doi foreground
        send_inputs(&[key_input(VK_MENU.0, false), key_input(VK_MENU.0, true)]);
        let _ = SetForegroundWindow(h);
        std::thread::sleep(Duration::from_millis(150));
        GetForegroundWindow() == h
    }
}

fn ms(a: Instant, b: Instant) -> i64 {
    if a >= b { a.duration_since(b).as_millis() as i64 } else { -(b.duration_since(a).as_millis() as i64) }
}

fn arg<T: std::str::FromStr>(args: &[String], name: &str) -> Option<T> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok())
}

fn main() {
    unsafe { let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2); }
    let args: Vec<String> = std::env::args().collect();
    let case: String = arg(&args, "--case").expect("--case uia_read|invoke|setvalue|sendinput");
    let runs: usize = arg(&args, "--runs").unwrap_or(5);
    let hang_ms: u64 = arg(&args, "--hang-ms").unwrap_or(8000);
    let auto_stop: Option<u64> = arg(&args, "--auto-stop-after");
    let uia_timeout: Option<u32> = arg(&args, "--uia-timeout");

    spawn_stop_paths();
    let mut pipe = Pipe::connect();
    println!("case={case} runs={runs} hang_ms={hang_ms} auto_stop={auto_stop:?} uia_timeout={uia_timeout:?}");
    if auto_stop.is_none() {
        println!("Bam Ctrl+Alt+Esc de dung moi run.");
    }
    println!("run | press->last_sent_ms | sent_after_flag | in_flight(late_possible) | n_sent | first_call_ms | first_call_res | save_delta | edit_changed | effect_delay_ms(from press) | flag-press_ms | held_released");

    let (mut pass, mut late, mut max_delay) = (0usize, false, 0i64);
    for run in 1..=runs {
        // het treo cua run truoc
        let t0 = Instant::now();
        while pipe.cmd("ui_alive") != "alive" && t0.elapsed() < Duration::from_secs(15) {
            std::thread::sleep(Duration::from_millis(100));
        }
        pipe.cmd("reset");
        std::thread::sleep(Duration::from_millis(300));
        let base = pipe.state();
        *STOP_SRC.lock().unwrap() = "";
        let gate = Arc::new(Gate::new());
        *GATE.lock().unwrap() = Some(gate.clone());

        let (rtx, rrx) = mpsc::channel();
        let (gtx, grx) = mpsc::channel();
        let (c, g) = (case.clone(), gate.clone());
        let h = std::thread::spawn(move || worker(c, g, uia_timeout, rtx, grx));
        match rrx.recv_timeout(Duration::from_secs(20)) {
            Ok(Ok(info)) if run == 1 => println!("  uia timeouts: {info}"),
            Ok(Ok(_)) => {}
            other => {
                println!("{run} | LOI chuan bi worker: {other:?}");
                continue;
            }
        }
        if case == "sendinput" {
            let fg = bring_front();
            println!("  foreground bench_target: {}", if fg { "OK" } else { "KHONG LAY DUOC" });
        }
        let hang_start = Instant::now();
        pipe.cmd(&format!("hang_ui {hang_ms}"));
        std::thread::sleep(Duration::from_millis(150));
        let _ = gtx.send(());
        if let Some(a) = auto_stop {
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(a));
                trigger_stop("timer");
            });
        }
        // do state den het treo + 1.5 s
        let deadline = hang_start + Duration::from_millis(hang_ms + 1500);
        let mut change_at: Option<Instant> = None;
        let mut last = base.clone();
        while Instant::now() < deadline {
            let s = pipe.state();
            if s != base && change_at.is_none() {
                change_at = Some(Instant::now());
            }
            last = s;
            std::thread::sleep(Duration::from_millis(10));
        }
        let t_join = Instant::now();
        while !h.is_finished() && t_join.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(50));
        }
        let finished = h.is_finished();
        let calls = if finished { h.join().unwrap() } else { Vec::new() };

        let st = gate.inner.lock().unwrap();
        let (Some(flag), Some(press)) = (st.flag_at, st.press_at) else {
            println!("{run} | khong co lenh dung (flag chua bat)");
            continue;
        };
        let last_sent = st.sent.last().map(|s| s.1);
        let after = st.sent.iter().filter(|s| s.1 > flag).count();
        let (first_ms, first_res) = calls.first().map(|c| (c.dur_ms.to_string(), c.res.clone())).unwrap_or(("-".into(), if finished { "-".into() } else { "worker chua thoat".into() }));
        let save_delta = last.0 as i64 - base.0 as i64;
        let edit_changed = last.2 != base.2;
        let delay = change_at.map(|t| ms(t, press));
        if after == 0 && finished {
            pass += 1;
        }
        if let Some(d) = delay {
            if save_delta != 0 || edit_changed {
                late = true;
                max_delay = max_delay.max(d);
            }
        }
        println!(
            "{run} | {} | {after} | {} | {} | {first_ms} | {first_res} | {save_delta} | {edit_changed} | {} | {} | -",
            last_sent.map(|t| ms(t, press).to_string()).unwrap_or("-".into()),
            st.late_effect_possible,
            st.sent.len(),
            delay.map(|d| d.to_string()).unwrap_or("-".into()),
            ms(flag, press),
        );
    }
    println!("0.13 {case}: {pass}/{runs} pass (no dispatch begins > 0 ms after stop), late_effect={} max_delay_ms={max_delay}", if late { "yes" } else { "no" });
}
