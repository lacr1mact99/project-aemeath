//! `bench_target` (T087): app thu co kiem soat cho spike dung khan cap (T088) va bench che do 3.
//!
//! Chay: `cargo run -p teach-core --example bench_target` (hoac chay thang exe trong
//! `target\debug\examples\bench_target.exe`). Cua so "Teach Bench Target": Edit 1001 ("Ten"),
//! Button 1002 ("Luu"), Button 1003 ("Gui"), Static 1004 (bo dem Luu + gia tri luu cuoi).
//! UIA hien control ID cua Win32 thanh AutomationId.
//!
//! Named pipe `\\.\pipe\teach-bench-target`: mot dong lenh, mot dong tra loi.
//!   hang_ui <ms>    treo luong UI <ms> (WM_APP+1 -> Sleep tren luong UI)
//!   dialog <title>  bat hop thoai modal cua chinh tien trinh (Cancel / OK / Don't Save)
//!   reset           xoa bo dem, xoa Edit, dong hop thoai
//!   state           JSON {"save_count":n,"last_saved":"..","edit":"..","dialog_open":bool}
//!                   doc tu trang thai that cua app (khong qua UIA, tra loi duoc ca khi UI treo)
//!   ui_alive        "alive" neu luong UI tra loi WM_NULL trong 200 ms, nguoc lai "hung"
//! Moi lan Luu in ra stdout: `save #n t_ms=<tu luc chay> unix_ms=<SystemTime>`.

use std::ffi::c_void;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::os::windows::io::FromRawHandle;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use windows::core::PCWSTR;
use windows::Win32::Foundation::{HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{GetStockObject, COLOR_WINDOW, DEFAULT_GUI_FONT, HBRUSH};
use windows::Win32::Storage::FileSystem::PIPE_ACCESS_DUPLEX;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE,
    PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};
use windows::Win32::System::Threading::Sleep;
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows::Win32::UI::WindowsAndMessaging::*;

const WM_HANG: u32 = WM_APP + 1;
const WM_DIALOG: u32 = WM_APP + 2;
const WM_RESET: u32 = WM_APP + 3;
const ID_EDIT: usize = 1001;
const ID_SAVE: usize = 1002;
const ID_SEND: usize = 1003;
const ID_LABEL: usize = 1004;

#[derive(Default)]
struct AppState {
    save_count: u32,
    send_count: u32,
    last_saved: String,
    edit: String,
    dialog_open: bool,
}

static STATE: Mutex<Option<AppState>> = Mutex::new(None);
static START: OnceLock<Instant> = OnceLock::new();
static MAIN: AtomicIsize = AtomicIsize::new(0);
static EDIT: AtomicIsize = AtomicIsize::new(0);
static LABEL: AtomicIsize = AtomicIsize::new(0);
static DLG: AtomicIsize = AtomicIsize::new(0);

fn hwnd(a: &AtomicIsize) -> HWND {
    HWND(a.load(Ordering::SeqCst) as *mut c_void)
}

fn w(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

fn with_state<R>(f: impl FnOnce(&mut AppState) -> R) -> R {
    let mut g = STATE.lock().unwrap();
    f(g.get_or_insert_with(AppState::default))
}

fn stamp() -> String {
    let unix = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    format!("t_ms={} unix_ms={}", START.get().unwrap().elapsed().as_millis(), unix)
}

fn window_text(h: HWND) -> String {
    let mut buf = [0u16; 512];
    let n = unsafe { GetWindowTextW(h, &mut buf) };
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

fn refresh_label() {
    let text = with_state(|s| {
        format!("Lưu: {} lần | Đã lưu: {} | Gửi: {}", s.save_count, s.last_saved, s.send_count)
    });
    let t = w(&text);
    unsafe {
        let _ = SetWindowTextW(hwnd(&LABEL), PCWSTR(t.as_ptr()));
    }
}

fn child(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    style: WINDOW_STYLE,
    (x, y, cw, ch): (i32, i32, i32, i32),
) -> HWND {
    let (c, t) = (w(class), w(text));
    unsafe {
        let h = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR(c.as_ptr()),
            PCWSTR(t.as_ptr()),
            WS_CHILD | WS_VISIBLE | style,
            x,
            y,
            cw,
            ch,
            Some(parent),
            Some(HMENU(id as *mut c_void)),
            None,
            None,
        )
        .expect("CreateWindowExW child");
        let font = GetStockObject(DEFAULT_GUI_FONT);
        SendMessageW(h, WM_SETFONT, Some(WPARAM(font.0 as usize)), Some(LPARAM(1)));
        h
    }
}

unsafe extern "system" fn main_proc(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            child(h, "STATIC", "Tên", 1000, WINDOW_STYLE(0), (20, 20, 60, 22));
            let e = child(h, "EDIT", "", ID_EDIT, WS_BORDER | WS_TABSTOP | WINDOW_STYLE(ES_AUTOHSCROLL as u32), (90, 18, 260, 24));
            EDIT.store(e.0 as isize, Ordering::SeqCst);
            child(h, "BUTTON", "Lưu", ID_SAVE, WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as u32), (90, 60, 100, 30));
            child(h, "BUTTON", "Gửi", ID_SEND, WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as u32), (200, 60, 100, 30));
            let l = child(h, "STATIC", "", ID_LABEL, WINDOW_STYLE(0), (20, 110, 370, 60));
            LABEL.store(l.0 as isize, Ordering::SeqCst);
            refresh_label();
            LRESULT(0)
        }
        WM_COMMAND => {
            let (id, note) = (wp.0 & 0xffff, (wp.0 >> 16) & 0xffff);
            if id == ID_EDIT && note == EN_CHANGE as usize {
                let text = window_text(hwnd(&EDIT));
                println!("edit {} value={:?}", stamp(), text);
                with_state(|s| s.edit = text);
            } else if id == ID_SAVE && note == BN_CLICKED as usize {
                let text = window_text(hwnd(&EDIT));
                let n = with_state(|s| {
                    s.save_count += 1;
                    s.last_saved = text;
                    s.save_count
                });
                println!("save #{} {}", n, stamp());
                refresh_label();
            } else if id == ID_SEND && note == BN_CLICKED as usize {
                with_state(|s| s.send_count += 1);
                println!("send {}", stamp());
                refresh_label();
            }
            LRESULT(0)
        }
        WM_HANG => {
            println!("hang_ui begin {} ms={}", stamp(), wp.0);
            unsafe { Sleep(wp.0 as u32) };
            println!("hang_ui end {}", stamp());
            LRESULT(0)
        }
        WM_DIALOG => {
            let title = unsafe { *Box::from_raw(lp.0 as *mut String) };
            run_dialog(h, &title);
            LRESULT(0)
        }
        WM_RESET => {
            with_state(|s| *s = AppState { dialog_open: s.dialog_open, ..Default::default() });
            let d = hwnd(&DLG);
            if !d.0.is_null() {
                unsafe { let _ = DestroyWindow(d); }
            }
            let empty = w("");
            unsafe { let _ = SetWindowTextW(hwnd(&EDIT), PCWSTR(empty.as_ptr())); }
            with_state(|s| s.edit.clear());
            refresh_label();
            println!("reset {}", stamp());
            LRESULT(0)
        }
        WM_DESTROY => {
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(h, msg, wp, lp) },
    }
}

unsafe extern "system" fn dlg_proc(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND | WM_CLOSE => {
            unsafe { let _ = DestroyWindow(h); }
            LRESULT(0)
        }
        // danh thuc vong lap modal dang cho GetMessage
        WM_DESTROY => {
            unsafe { let _ = PostMessageW(Some(hwnd(&MAIN)), WM_NULL, WPARAM(0), LPARAM(0)); }
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(h, msg, wp, lp) },
    }
}

/// Hop thoai modal: chu so huu = cua so chinh, vong lap thong diep long nhau tren luong UI.
fn run_dialog(owner: HWND, title: &str) {
    if !hwnd(&DLG).0.is_null() {
        return;
    }
    let (cls, t) = (w("TeachBenchDialog"), w(title));
    unsafe {
        let d = CreateWindowExW(
            WS_EX_DLGMODALFRAME,
            PCWSTR(cls.as_ptr()),
            PCWSTR(t.as_ptr()),
            WS_POPUP | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            200,
            200,
            380,
            130,
            Some(owner),
            None,
            None,
            None,
        )
        .expect("CreateWindowExW dialog");
        DLG.store(d.0 as isize, Ordering::SeqCst);
        child(d, "BUTTON", "Cancel", 2001, WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as u32), (10, 20, 100, 30));
        child(d, "BUTTON", "OK", 2002, WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as u32), (120, 20, 100, 30));
        child(d, "BUTTON", "Don't Save", 2003, WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as u32), (230, 20, 120, 30));
        with_state(|s| s.dialog_open = true);
        let _ = EnableWindow(owner, false);
        let _ = SetForegroundWindow(d);
        println!("dialog open {} title={:?}", stamp(), title);
        let mut m = MSG::default();
        while IsWindow(Some(d)).as_bool() {
            let r = GetMessageW(&mut m, None, 0, 0);
            if r.0 <= 0 {
                if r.0 == 0 {
                    PostQuitMessage(m.wParam.0 as i32);
                }
                break;
            }
            let _ = TranslateMessage(&m);
            DispatchMessageW(&m);
        }
        DLG.store(0, Ordering::SeqCst);
        with_state(|s| s.dialog_open = false);
        let _ = EnableWindow(owner, true);
        let _ = SetForegroundWindow(owner);
        println!("dialog closed {}", stamp());
    }
}

fn handle_command(line: &str) -> String {
    let (cmd, arg) = line.split_once(' ').unwrap_or((line, ""));
    let main = hwnd(&MAIN);
    let post = |m: u32, wp: usize, lp: isize| unsafe {
        let _ = PostMessageW(Some(main), m, WPARAM(wp), LPARAM(lp));
    };
    match cmd {
        "hang_ui" => match arg.trim().parse::<usize>() {
            Ok(ms) => {
                post(WM_HANG, ms, 0);
                "ok".into()
            }
            Err(_) => "err bad ms".into(),
        },
        "dialog" => {
            let b = Box::into_raw(Box::new(arg.to_string()));
            post(WM_DIALOG, 0, b as isize);
            "ok".into()
        }
        "reset" => {
            post(WM_RESET, 0, 0);
            "ok".into()
        }
        "state" => with_state(|s| {
            serde_json::json!({
                "save_count": s.save_count,
                "last_saved": s.last_saved,
                "edit": s.edit,
                "dialog_open": s.dialog_open,
            })
            .to_string()
        }),
        "ui_alive" => {
            let r = unsafe {
                SendMessageTimeoutW(main, WM_NULL, WPARAM(0), LPARAM(0), SMTO_NORMAL, 200, None)
            };
            if r.0 != 0 { "alive" } else { "hung" }.into()
        }
        _ => "err unknown command".into(),
    }
}

fn pipe_server() {
    let name = w(r"\\.\pipe\teach-bench-target");
    let pipe = unsafe {
        CreateNamedPipeW(
            PCWSTR(name.as_ptr()),
            PIPE_ACCESS_DUPLEX,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
            PIPE_UNLIMITED_INSTANCES,
            4096,
            4096,
            0,
            None,
        )
    };
    let raw: HANDLE = pipe;
    // ponytail: mot instance dung lai cho moi client (Disconnect roi Connect), khong dong handle
    let file = std::mem::ManuallyDrop::new(unsafe { File::from_raw_handle(raw.0) });
    loop {
        let _ = unsafe { ConnectNamedPipe(raw, None) }; // ERROR_PIPE_CONNECTED cung chap nhan
        let mut reader = BufReader::new(&*file);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let reply = handle_command(line.trim());
                    let mut f: &File = &file;
                    if f.write_all(format!("{reply}\n").as_bytes()).is_err() {
                        break;
                    }
                }
            }
        }
        unsafe { let _ = DisconnectNamedPipe(raw); }
    }
}

fn main() {
    START.set(Instant::now()).ok();
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let hi: HINSTANCE = GetModuleHandleW(None).expect("GetModuleHandleW").into();
        let bg = HBRUSH((COLOR_WINDOW.0 + 1) as isize as *mut c_void);
        let arrow = LoadCursorW(None, IDC_ARROW).ok().unwrap_or_default();
        let (mc, dc) = (w("TeachBenchTarget"), w("TeachBenchDialog"));
        let mut wc = WNDCLASSW {
            lpfnWndProc: Some(main_proc),
            hInstance: hi,
            lpszClassName: PCWSTR(mc.as_ptr()),
            hbrBackground: bg,
            hCursor: arrow,
            ..Default::default()
        };
        assert!(RegisterClassW(&wc) != 0, "RegisterClassW main");
        wc.lpfnWndProc = Some(dlg_proc);
        wc.lpszClassName = PCWSTR(dc.as_ptr());
        assert!(RegisterClassW(&wc) != 0, "RegisterClassW dialog");

        let title = w("Teach Bench Target");
        let main = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            PCWSTR(mc.as_ptr()),
            PCWSTR(title.as_ptr()),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_VISIBLE,
            100,
            100,
            420,
            260,
            None,
            None,
            Some(hi),
            None,
        )
        .expect("CreateWindowExW main");
        MAIN.store(main.0 as isize, Ordering::SeqCst);
        let _ = SetForegroundWindow(main);
        println!("bench_target ready pid={} {}", std::process::id(), stamp());

        std::thread::spawn(pipe_server);

        let mut m = MSG::default();
        while GetMessageW(&mut m, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&m);
            DispatchMessageW(&m);
        }
    }
}
