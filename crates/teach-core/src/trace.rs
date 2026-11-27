//! Bản ghi `events.jsonl` + `meta.yaml` — contracts/trace-format.md, data-model §Trace.

use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WindowInfo {
    pub title: String,
    pub process: String,
    #[serde(default)]
    pub process_path: String,
    /// `[x, y, w, h]`, pixel vật lý.
    pub rect: [i32; 4],
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ElementInfo {
    #[serde(default)]
    pub name: String,
    pub role: String,
    #[serde(default)]
    pub automation_id: String,
    #[serde(default)]
    pub class_name: String,
    #[serde(default)]
    pub path: Vec<String>,
    #[serde(default)]
    pub rect: Option<[i32; 4]>,
    /// `None` = không xác định (research R-4: coi như bí mật).
    #[serde(default)]
    pub is_password: Option<bool>,
}

/// Phần tử mới trong `state` — không bao giờ có Value của ô nhập.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NewElement {
    #[serde(default)]
    pub name: String,
    pub role: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub automation_id: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Uncertain {
    Element,
    ShotBefore,
    State,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClipOp {
    Copy,
    Paste,
    Cut,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClipSource {
    Hotkey,
    Menu,
    Unknown,
}

pub const MAX_NEW_ELEMENTS: usize = 50;

/// Trường chung có ở hành động gắn với phần tử.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct Common {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element: Option<ElementInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ocr_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shot_before: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shot_after: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub uncertain: Vec<Uncertain>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EventKind {
    Click {
        button: String,
        x: i32,
        y: i32,
        #[serde(default)]
        double: bool,
        #[serde(flatten)]
        c: Common,
    },
    Scroll {
        dx: i32,
        dy: i32,
        x: i32,
        y: i32,
        #[serde(flatten)]
        c: Common,
    },
    /// `secret: true` ⇒ không có `text` (FR-006) — `Event::check` bảo đảm.
    Type {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        secret: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        length: Option<u32>,
        #[serde(flatten)]
        c: Common,
    },
    Hotkey {
        keys: String,
        #[serde(flatten)]
        c: Common,
    },
    WindowFocus {},
    /// Chỉ thao tác, không bao giờ nội dung bộ nhớ tạm.
    Clipboard { op: ClipOp, source: ClipSource, cause_t_ms: u64 },
    State {
        after_t_ms: u64,
        focused_title: String,
        #[serde(default)]
        new_elements: Vec<NewElement>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        uncertain: Vec<Uncertain>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Event {
    pub t_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<WindowInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lag_ms: Option<u64>,
    #[serde(flatten)]
    pub kind: EventKind,
}

impl Event {
    /// Quy tắc bắt buộc của contract mà kiểu không tự bảo đảm.
    pub fn check(&self) -> Result<(), &'static str> {
        match &self.kind {
            EventKind::Type { text, secret, c, .. } => {
                let pw = c.element.as_ref().map(|e| e.is_password);
                // is_password true hoặc không xác định (kể cả không có element) ⇒ phải là secret.
                let must_secret = !matches!(pw, Some(Some(false)));
                if must_secret && !secret {
                    return Err("type_must_be_secret");
                }
                if *secret && text.is_some() {
                    return Err("secret_with_text");
                }
                if *secret && c.element.as_ref().and_then(|e| e.rect).is_none() && (c.shot_before.is_some() || c.shot_after.is_some()) {
                    return Err("secret_shot_without_rect");
                }
                Ok(())
            }
            EventKind::State { new_elements, .. } if new_elements.len() > MAX_NEW_ELEMENTS => Err("too_many_new_elements"),
            _ => Ok(()),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    User,
    TimeLimit,
    DiskFull,
    InputGuardLost,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Screen {
    pub width: u32,
    pub height: u32,
    pub scale: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Meta {
    pub id: String,
    pub goal: String,
    pub started_at: String,
    pub ended_at: String,
    #[serde(default)]
    pub paused_ms: u64,
    pub stop_reason: StopReason,
    pub screen: Screen,
}

/// Đọc `events.jsonl`; dòng nào vi phạm contract → lỗi kèm số dòng.
pub fn read_events(path: &Path) -> Result<Vec<Event>, String> {
    let f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for (i, line) in std::io::BufReader::new(f).lines().enumerate() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let ev: Event = serde_json::from_str(&line).map_err(|e| format!("dòng {}: {e}", i + 1))?;
        ev.check().map_err(|e| format!("dòng {}: {e}", i + 1))?;
        out.push(ev);
    }
    Ok(out)
}

/// Ghi thêm một sự kiện + flush (FR-003: không mất sự kiện khi tiến trình chết).
pub fn append_event(w: &mut impl Write, ev: &Event) -> std::io::Result<()> {
    ev.check().map_err(std::io::Error::other)?;
    serde_json::to_writer(&mut *w, ev)?;
    w.write_all(b"\n")?;
    w.flush()
}

pub fn read_meta(path: &Path) -> Result<Meta, String> {
    let s = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_saphyr::from_str(&s).map_err(|e| e.to_string())
}

pub fn meta_to_yaml(m: &Meta) -> Result<String, String> {
    serde_saphyr::to_string(m).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_contract_examples() {
        let lines = [
            r#"{"t_ms":4210,"kind":"click","button":"left","x":812,"y":440,"double":false,"window":{"title":"Hoá đơn - Excel","process":"EXCEL.EXE","process_path":"C:\\x\\EXCEL.EXE","rect":[0,0,1920,1040]},"element":{"name":"Lưu","role":"Button","automation_id":"FileSave","class_name":"NetUIRibbonButton","path":["Window","Pane","ToolBar","Button"],"rect":[790,428,44,24],"is_password":false},"ocr_text":null,"shot_before":"0007a.png","shot_after":"0007b.png"}"#,
            r#"{"t_ms":6100,"kind":"type","secret":true,"length":null,"element":{"name":"","role":"Edit","is_password":true}}"#,
            r#"{"t_ms":7000,"kind":"hotkey","keys":"Ctrl+S"}"#,
            r#"{"t_ms":8000,"kind":"window_focus","window":{"title":"a","process":"b.exe","rect":[0,0,1,1]}}"#,
            r#"{"t_ms":8100,"kind":"clipboard","op":"copy","source":"hotkey","cause_t_ms":8090}"#,
            r#"{"t_ms":8300,"kind":"state","after_t_ms":8000,"focused_title":"Save As","new_elements":[{"name":"Save","role":"Button"}]}"#,
        ];
        for l in lines {
            let ev: Event = serde_json::from_str(l).unwrap_or_else(|e| panic!("{l}: {e}"));
            ev.check().unwrap();
            let back: Event = serde_json::from_str(&serde_json::to_string(&ev).unwrap()).unwrap();
            assert_eq!(ev, back);
        }
    }

    #[test]
    fn rejects_privacy_violations() {
        let bad = [
            // ô mật khẩu mà có text
            r#"{"t_ms":1,"kind":"type","text":"abc","element":{"role":"Edit","is_password":true}}"#,
            // không xác định is_password mà không secret
            r#"{"t_ms":1,"kind":"type","text":"abc","element":{"role":"Edit"}}"#,
            // secret không có rect mà vẫn có ảnh
            r#"{"t_ms":1,"kind":"type","secret":true,"shot_before":"1a.png"}"#,
        ];
        for l in bad {
            let ev: Event = serde_json::from_str(l).unwrap();
            assert!(ev.check().is_err(), "{l}");
        }
        // clipboard có nội dung → trường lạ
        assert!(serde_json::from_str::<Event>(r#"{"t_ms":1,"kind":"clipboard","op":"copy","source":"menu","cause_t_ms":0,"text":"x"}"#).is_err());
    }
}
