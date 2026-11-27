//! M0 spike T009 / quickstart row 0.7 / research R-9: window capture accuracy, xcap vs GDI BitBlt.
//!
//! Measures, for the top-level window of a process (plus its other visible top-level
//! windows, e.g. owned dialogs): DWM extended frame bounds, window DPI, monitor scale;
//! then captures with (a) xcap (per window) and (b) GDI BitBlt from the screen DC over
//! the union rectangle of the process windows. Pixels in the BitBlt image outside every
//! process window, and any rectangle covered by a window of a DIFFERENT process that is
//! above it in z-order, are painted black. The full screen is never captured or saved.
//! Long edge is downscaled to 1920 px. Prints xcap x/y/w/h vs DWM bounds vs PNG dims,
//! PNG bytes and capture time (ms) per method, and the occluder rects.
//!
//! Run: `cargo run -p teach-core --example spike_capture -- notepad.exe C:\temp\cap`
//!
//! Operator procedure for 0.7 (repeat each case, keep the printed output + PNGs):
//!  1. Set display scaling to 150%, open the target app, run the spike. Read: do
//!     `xcap` w/h equal the DWM w/h, or the PNG dims? Is the PNG content aligned with the
//!     window (no shifted/cropped edge, no shadow margin)?
//!  2. Two monitors with different DPI (e.g. 100% and 150%): run once with the window on
//!     each monitor, and once straddling both. Read `dpi`, `monitor scale`, dims again.
//!  3. Open another app's window over the target and run again: its rect must show up
//!     under `occluders` and be black in both PNGs.
//!  4. Decide xcap vs BitBlt by comparing the printed frames/dims and looking at the
//!     PNGs: the method whose dims match DWM bounds (physical px) at every DPI wins.

use std::{path::Path, time::Instant};

use image::{Rgba, RgbaImage, imageops::FilterType};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::HiDpi::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{BOOL, PWSTR};

const MAX_EDGE: u32 = 1920;

struct Win {
    hwnd: HWND,
    pid: u32,
    title: String,
    rect: RECT, // DWM extended frame bounds, physical px
}

fn pid_of(h: HWND) -> u32 {
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
    pid
}

fn exe_of(pid: u32) -> Option<String> {
    unsafe {
        let p = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut n = buf.len() as u32;
        let r = QueryFullProcessImageNameW(p, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut n);
        let _ = CloseHandle(p);
        r.ok()?;
        Some(String::from_utf16_lossy(&buf[..n as usize]))
    }
}

fn dwm_rect(h: HWND) -> Option<RECT> {
    let mut r = RECT::default();
    unsafe {
        DwmGetWindowAttribute(h, DWMWA_EXTENDED_FRAME_BOUNDS, &mut r as *mut _ as *mut _, size_of::<RECT>() as u32)
            .ok()?;
    }
    Some(r)
}

/// Visible, non-minimized, non-cloaked window with a non-empty frame.
fn info(h: HWND) -> Option<Win> {
    unsafe {
        if !IsWindowVisible(h).as_bool() || IsIconic(h).as_bool() {
            return None;
        }
        let mut cloaked = 0u32;
        let _ = DwmGetWindowAttribute(h, DWMWA_CLOAKED, &mut cloaked as *mut _ as *mut _, 4);
        let rect = dwm_rect(h)?;
        if cloaked != 0 || rect.right <= rect.left || rect.bottom <= rect.top {
            return None;
        }
        let mut t = [0u16; 512];
        let n = GetWindowTextW(h, &mut t) as usize;
        Some(Win { hwnd: h, pid: pid_of(h), title: String::from_utf16_lossy(&t[..n]), rect })
    }
}

fn all_windows() -> Vec<Win> {
    unsafe extern "system" fn cb(h: HWND, l: LPARAM) -> BOOL {
        unsafe { (*(l.0 as *mut Vec<HWND>)).push(h) };
        BOOL(1)
    }
    let mut v: Vec<HWND> = Vec::new();
    unsafe { EnumWindows(Some(cb), LPARAM(&mut v as *mut _ as isize)).ok() };
    v.into_iter().filter_map(info).collect()
}

fn inter(a: &RECT, b: &RECT) -> Option<RECT> {
    let r = RECT {
        left: a.left.max(b.left),
        top: a.top.max(b.top),
        right: a.right.min(b.right),
        bottom: a.bottom.min(b.bottom),
    };
    (r.right > r.left && r.bottom > r.top).then_some(r)
}

fn fmt(r: &RECT) -> String {
    format!("x={} y={} w={} h={}", r.left, r.top, r.right - r.left, r.bottom - r.top)
}

/// Paint `r` (screen coords) black on `img` whose top-left is at screen (ox, oy).
fn black(img: &mut RgbaImage, ox: i32, oy: i32, r: &RECT) {
    for y in r.top.max(oy)..r.bottom.min(oy + img.height() as i32) {
        for x in r.left.max(ox)..r.right.min(ox + img.width() as i32) {
            img.put_pixel((x - ox) as u32, (y - oy) as u32, Rgba([0, 0, 0, 255]));
        }
    }
}

fn bitblt(r: &RECT) -> Result<RgbaImage, String> {
    let (w, h) = ((r.right - r.left) as u32, (r.bottom - r.top) as u32);
    unsafe {
        let screen = GetDC(None);
        let mem = CreateCompatibleDC(Some(screen));
        let bmp = CreateCompatibleBitmap(screen, w as i32, h as i32);
        let old = SelectObject(mem, bmp.into());
        // CAPTUREBLT: include layered windows
        let ok = BitBlt(mem, 0, 0, w as i32, h as i32, Some(screen), r.left, r.top, SRCCOPY | CAPTUREBLT);
        let mut bi = BITMAPINFO::default();
        bi.bmiHeader = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: w as i32,
            biHeight: -(h as i32), // top-down
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        };
        let mut buf = vec![0u8; (w * h * 4) as usize];
        let lines = GetDIBits(mem, bmp, 0, h, Some(buf.as_mut_ptr() as *mut _), &mut bi, DIB_RGB_COLORS);
        SelectObject(mem, old);
        let _ = DeleteObject(bmp.into());
        let _ = DeleteDC(mem);
        ReleaseDC(None, screen);
        ok.map_err(|e| format!("BitBlt: {e}"))?;
        if lines == 0 {
            return Err("GetDIBits returned 0".into());
        }
        for px in buf.chunks_exact_mut(4) {
            px.swap(0, 2); // BGRA -> RGBA
            px[3] = 255;
        }
        RgbaImage::from_raw(w, h, buf).ok_or_else(|| "bad buffer".into())
    }
}

/// Downscale (long edge > 1920), save PNG, print dims / bytes.
fn save(img: RgbaImage, dir: &Path, name: &str, ms: u128) {
    let (w, h) = img.dimensions();
    let img = if w.max(h) > MAX_EDGE {
        let s = MAX_EDGE as f32 / w.max(h) as f32;
        image::imageops::resize(&img, (w as f32 * s).round() as u32, (h as f32 * s).round() as u32, FilterType::Triangle)
    } else {
        img
    };
    let path = dir.join(name);
    let t = Instant::now();
    let res = img.save(&path);
    let bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    println!(
        "  {name}: raw {w}x{h} -> PNG {}x{} bytes={bytes} capture_ms={ms} png_encode_ms={} save={:?}",
        img.width(),
        img.height(),
        t.elapsed().as_millis(),
        res.map(|_| path.display().to_string())
    );
}

fn main() {
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: spike_capture <process name e.g. notepad.exe> <output dir>");
        std::process::exit(2);
    }
    let dir = Path::new(&args[2]);
    std::fs::create_dir_all(dir).expect("create output dir");

    let wins = all_windows();
    let mine: Vec<&Win> = wins
        .iter()
        .filter(|w| {
            exe_of(w.pid)
                .and_then(|p| p.rsplit('\\').next().map(|n| n.eq_ignore_ascii_case(&args[1])))
                .unwrap_or(false)
        })
        .collect();
    // target = largest un-owned, titled window of the process
    let area = |w: &Win| (w.rect.right - w.rect.left) as i64 * (w.rect.bottom - w.rect.top) as i64;
    let Some(target) = mine
        .iter()
        .filter(|w| !w.title.is_empty() && unsafe { GetWindow(w.hwnd, GW_OWNER) }.is_err_or_null())
        .max_by_key(|w| area(w))
        .copied()
    else {
        eprintln!("no visible top-level window for {}", args[1]);
        std::process::exit(1);
    };
    let dpi = unsafe { GetDpiForWindow(target.hwnd) };
    println!("target: hwnd={:?} pid={} title={:?}", target.hwnd.0, target.pid, target.title);
    println!("DWM bounds: {}  window dpi={dpi} (scale {}%)", fmt(&target.rect), dpi * 100 / 96);

    // region list: target + other visible top-level windows of same process
    let mut bbox = target.rect;
    println!("regions:");
    for w in &mine {
        println!("  hwnd={:?} title={:?} {}", w.hwnd.0, w.title, fmt(&w.rect));
        bbox.left = bbox.left.min(w.rect.left);
        bbox.top = bbox.top.min(w.rect.top);
        bbox.right = bbox.right.max(w.rect.right);
        bbox.bottom = bbox.bottom.max(w.rect.bottom);
    }

    // occluders: other-process windows above any region window in z-order
    let mut occ: Vec<RECT> = Vec::new();
    for w in &mine {
        let mut h = w.hwnd;
        loop {
            h = unsafe { GetWindow(h, GW_HWNDPREV) }.unwrap_or_default();
            if h.0.is_null() {
                break;
            }
            if let Some(o) = info(h).filter(|o| o.pid != target.pid)
                && let Some(r) = inter(&o.rect, &bbox)
            {
                println!("  occluder pid={} {:?} {}", o.pid, o.title, fmt(&r));
                occ.push(r);
            }
        }
    }
    println!("occluders: {}", occ.len());

    // (a) xcap
    println!("xcap:");
    let xw = xcap::Window::all().ok().and_then(|v| {
        let same = |w: &&xcap::Window| w.pid().ok() == Some(target.pid);
        v.iter().find(|w| same(w) && w.title().ok().as_deref() == Some(&target.title)).or_else(|| v.iter().find(same)).cloned()
    });
    match xw {
        None => println!("  no xcap window matched pid {}", target.pid),
        Some(w) => {
            let (x, y) = (w.x().unwrap_or(0), w.y().unwrap_or(0));
            let (ww, wh) = (w.width().unwrap_or(0), w.height().unwrap_or(0));
            let sf = w.current_monitor().and_then(|m| m.scale_factor()).ok();
            println!(
                "  xcap reported x={x} y={y} w={ww} h={wh} | DWM {} | monitor scale_factor={sf:?}",
                fmt(&target.rect)
            );
            let t = Instant::now();
            match w.capture_image() {
                Ok(mut img) => {
                    let ms = t.elapsed().as_millis();
                    println!("  xcap image {}x{}", img.width(), img.height());
                    for r in &occ {
                        black(&mut img, x, y, r);
                    }
                    save(img, dir, "xcap.png", ms);
                }
                Err(e) => println!("  xcap capture FAILED: {e}"),
            }
        }
    }

    // (b) GDI BitBlt over bbox
    println!("bitblt (bbox {}):", fmt(&bbox));
    let t = Instant::now();
    match bitblt(&bbox) {
        Ok(mut img) => {
            let ms = t.elapsed().as_millis();
            // black out everything not in a process window, then occluders
            let mut keep = RgbaImage::from_pixel(img.width(), img.height(), Rgba([0, 0, 0, 255]));
            for w in &mine {
                for y in w.rect.top..w.rect.bottom {
                    for x in w.rect.left..w.rect.right {
                        let (px, py) = ((x - bbox.left) as u32, (y - bbox.top) as u32);
                        keep.put_pixel(px, py, *img.get_pixel(px, py));
                    }
                }
            }
            img = keep;
            for r in &occ {
                black(&mut img, bbox.left, bbox.top, r);
            }
            save(img, dir, "bitblt.png", ms);
        }
        Err(e) => println!("  bitblt FAILED: {e}"),
    }
}

trait NullWin {
    fn is_err_or_null(&self) -> bool;
}
impl NullWin for windows::core::Result<HWND> {
    fn is_err_or_null(&self) -> bool {
        self.as_ref().map(|h| h.0.is_null()).unwrap_or(true)
    }
}
