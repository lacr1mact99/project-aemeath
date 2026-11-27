//! Skill/Step/Selector/Condition — contracts/skill-format.md, data-model §Skill.
//! File là đầu vào không tin cậy (FR-047): trường lạ → lỗi, `status` không bao giờ đọc từ file.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Skill {
    pub version: u32,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub source_trace: Option<String>,
    #[serde(default)]
    pub confirmed: Option<Confirmed>,
    #[serde(default)]
    pub inputs: Vec<Input>,
    pub steps: Vec<Step>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Confirmed {
    pub run: String,
    pub fingerprint: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub example: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Click,
    Type,
    Hotkey,
    Scroll,
    Wait,
    Launch,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub id: u32,
    pub intent: String,
    pub action: Action,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<WindowMatch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<Vec<Selector>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keys: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scroll: Option<Scroll>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wait_ms: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch: Option<Launch>,
    #[serde(default)]
    pub expect: Vec<Condition>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub approval: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub repeat_safe: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skip_if: Option<Condition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WindowMatch {
    pub process: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_contains: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process_path: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Scroll {
    pub dx: i32,
    pub dy: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Launch {
    pub path: String,
    #[serde(default)]
    pub args: Vec<String>,
}

/// Một selector: đúng một dạng chính (`validate` kiểm). Phẳng để YAML giống contracts/skill-format.md.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(deny_unknown_fields)]
pub struct Selector {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ocr_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xy_in_window: Option<[f64; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<At>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub patch: Option<Patch>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct At {
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Patch {
    pub id: String,
    pub run: String,
    pub pending_review: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    WindowTitleContains(String),
    ElementExists(Selector),
    ElementAbsent(Selector),
    TargetValueEquals(String),
    FocusedProcess(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Draft,
    Ready,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectorForm {
    AutomationId,
    RoleName,
    OcrText,
    Xy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldError {
    /// Đường dẫn trường, ví dụ `steps[2].target[0]`.
    pub field: String,
    /// Mã cố định cho giao diện/i18n, ví dụ `required`, `not_allowed`, `selector_form`.
    pub code: &'static str,
}

impl Selector {
    /// Dạng chính, hoặc `None` khi không đúng một dạng.
    pub fn form(&self) -> Option<SelectorForm> {
        let s = self;
        match (
            s.automation_id.is_some(),
            s.role.is_some(),
            s.name.is_some(),
            s.ocr_text.is_some(),
            s.xy_in_window.is_some(),
        ) {
            (true, _, false, false, false) => Some(SelectorForm::AutomationId),
            (false, true, true, false, false) => Some(SelectorForm::RoleName),
            (false, false, false, true, false) => Some(SelectorForm::OcrText),
            (false, false, false, false, true) => Some(SelectorForm::Xy),
            _ => None,
        }
    }

    fn strings(&self) -> impl Iterator<Item = &str> {
        let at = self.at.as_ref();
        [
            self.automation_id.as_deref(),
            self.role.as_deref(),
            self.name.as_deref(),
            self.ocr_text.as_deref(),
            at.map(|a| a.role.as_str()),
            at.and_then(|a| a.name.as_deref()),
        ]
        .into_iter()
        .flatten()
    }
}

impl Condition {
    fn strings(&self) -> Vec<&str> {
        match self {
            Condition::WindowTitleContains(s) | Condition::TargetValueEquals(s) | Condition::FocusedProcess(s) => {
                vec![s]
            }
            Condition::ElementExists(sel) | Condition::ElementAbsent(sel) => sel.strings().collect(),
        }
    }
    fn selector(&self) -> Option<&Selector> {
        match self {
            Condition::ElementExists(s) | Condition::ElementAbsent(s) => Some(s),
            _ => None,
        }
    }
}

/// Phần giữ chỗ trong chuỗi.
#[derive(Debug, PartialEq, Eq)]
pub enum Placeholder<'a> {
    Input(&'a str),
    Secret(&'a str),
}

/// Tách mọi `{{...}}`. `Err` khi có `{{` không đóng hoặc nội dung sai cú pháp (quy tắc 4, 6).
pub fn placeholders(s: &str) -> Result<Vec<Placeholder<'_>>, ()> {
    let mut out = Vec::new();
    let mut rest = s;
    while let Some(i) = rest.find("{{") {
        let after = &rest[i + 2..];
        let j = after.find("}}").ok_or(())?;
        let inner = &after[..j];
        if let Some(k) = inner.strip_prefix("secret:") {
            if k.is_empty() || !k.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_') {
                return Err(());
            }
            out.push(Placeholder::Secret(k));
        } else if valid_input_name(inner) {
            out.push(Placeholder::Input(inner));
        } else {
            return Err(());
        }
        rest = &after[j + 2..];
    }
    Ok(out)
}

fn valid_input_name(n: &str) -> bool {
    let mut b = n.bytes();
    matches!(b.next(), Some(c) if c.is_ascii_lowercase() || c == b'_')
        && b.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
}

fn valid_skill_name(n: &str) -> bool {
    (1..=64).contains(&n.len()) && n.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}

/// Thay `{{name}}` bằng giá trị input; giữ nguyên `{{secret:k}}` (runner đọc bí mật ngay trước khi gõ).
/// Input thiếu giá trị được giữ nguyên chữ — `start_run` đã từ chối trường hợp này trước đó.
pub fn substitute(s: &str, inputs: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find("{{") {
        out.push_str(&rest[..i]);
        let after = &rest[i + 2..];
        let Some(j) = after.find("}}") else {
            out.push_str(&rest[i..]);
            return out;
        };
        let inner = &after[..j];
        match inputs.get(inner) {
            Some(v) if !inner.starts_with("secret:") => out.push_str(v),
            _ => {
                out.push_str("{{");
                out.push_str(inner);
                out.push_str("}}");
            }
        }
        rest = &after[j + 2..];
    }
    out.push_str(rest);
    out
}

/// Tên nút gợi ý bật `approval` (FR-021), so khớp sau `fold`.
const APPROVAL_NAMES: &[&str] = &[
    "Gửi", "Send", "Submit", "Xoá", "Xóa", "Delete", "Remove", "Thanh toán", "Pay", "Đặt hàng", "Order", "Xác nhận",
    "Confirm",
];
const APPROVAL_KEYS: &[&str] = &["delete", "shift+delete", "ctrl+enter", "alt+f4"];

fn norm_keys(k: &str) -> String {
    k.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_lowercase()
}

impl Skill {
    pub fn from_yaml(s: &str) -> Result<Skill, String> {
        serde_saphyr::from_str(s).map_err(|e| e.to_string())
    }

    pub fn to_yaml(&self) -> Result<String, String> {
        serde_saphyr::to_string(self).map_err(|e| e.to_string())
    }

    /// Dấu vân tay thực thi (data-model, FR-017): bỏ `name`, `description`, `intent`, `warnings`,
    /// `source_trace`, `confirmed` và selector có `patch`.
    pub fn exec_fingerprint(&self) -> String {
        #[derive(Serialize)]
        struct FpStep<'a> {
            id: u32,
            action: Action,
            window: &'a Option<WindowMatch>,
            target: Option<Vec<&'a Selector>>,
            text: &'a Option<String>,
            keys: &'a Option<String>,
            scroll: &'a Option<Scroll>,
            wait_ms: Option<u32>,
            launch: &'a Option<Launch>,
            expect: &'a [Condition],
            approval: bool,
            repeat_safe: bool,
            skip_if: &'a Option<Condition>,
        }
        #[derive(Serialize)]
        struct Fp<'a> {
            inputs: Vec<&'a str>,
            steps: Vec<FpStep<'a>>,
        }
        let fp = Fp {
            inputs: self.inputs.iter().map(|i| i.name.as_str()).collect(),
            steps: self
                .steps
                .iter()
                .map(|s| FpStep {
                    id: s.id,
                    action: s.action,
                    window: &s.window,
                    target: s.target.as_ref().map(|t| t.iter().filter(|x| x.patch.is_none()).collect()),
                    text: &s.text,
                    keys: &s.keys,
                    scroll: &s.scroll,
                    wait_ms: s.wait_ms,
                    launch: &s.launch,
                    expect: &s.expect,
                    approval: s.approval,
                    repeat_safe: s.repeat_safe,
                    skip_if: &s.skip_if,
                })
                .collect(),
        };
        let json = serde_json::to_vec(&fp).expect("serialize fingerprint view");
        format!("{:016x}", crate::fnv1a64(&json))
    }

    pub fn status(&self) -> Status {
        match &self.confirmed {
            Some(c) if c.fingerprint == self.exec_fingerprint() => Status::Ready,
            _ => Status::Draft,
        }
    }

    /// Quy tắc hợp lệ 1–8 của contracts/skill-format.md (quy tắc 1 về trường lạ do serde bắt khi tải).
    pub fn validate(&self) -> Vec<FieldError> {
        let mut errs = Vec::new();
        let mut err = |field: String, code: &'static str| errs.push(FieldError { field, code });

        if self.version != 1 {
            err("version".into(), "unsupported_version");
        }
        if !valid_skill_name(&self.name) {
            err("name".into(), "invalid_name");
        }
        let mut input_names = HashSet::new();
        for (i, inp) in self.inputs.iter().enumerate() {
            if !valid_input_name(&inp.name) || inp.name == "secret" {
                err(format!("inputs[{i}].name"), "invalid_name");
            }
            if !input_names.insert(inp.name.as_str()) {
                err(format!("inputs[{i}].name"), "duplicate");
            }
        }
        if self.steps.is_empty() {
            err("steps".into(), "required");
        }
        let mut ids = HashSet::new();
        let mut patch_ids = HashSet::new();
        for (si, s) in self.steps.iter().enumerate() {
            let p = format!("steps[{si}]");
            if !ids.insert(s.id) {
                err(format!("{p}.id"), "duplicate");
            }
            if s.intent.contains(['\n', '\r']) || s.intent.chars().count() > 120 {
                err(format!("{p}.intent"), "invalid_intent");
            }

            // Quy tắc 2: trường bắt buộc / không thuộc action.
            use Action::*;
            let (need_window, need_target, need_text, need_keys, need_scroll, need_wait, need_launch) = match s.action {
                Click => (true, true, false, false, false, false, false),
                Type => (true, true, true, false, false, false, false),
                Hotkey => (true, false, false, true, false, false, false),
                Scroll => (true, true, false, false, true, false, false),
                Wait => (false, false, false, false, false, true, false),
                Launch => (true, false, false, false, false, false, true),
            };
            let fields: [(&str, bool, bool); 7] = [
                ("window", need_window, s.window.is_some()),
                ("target", need_target, s.target.is_some()),
                ("text", need_text, s.text.is_some()),
                ("keys", need_keys, s.keys.is_some()),
                ("scroll", need_scroll, s.scroll.is_some()),
                ("wait_ms", need_wait, s.wait_ms.is_some()),
                ("launch", need_launch, s.launch.is_some()),
            ];
            for (name, need, has) in fields {
                // `window` được phép (không bắt buộc) ở `wait`.
                let optional = name == "window" && s.action == Wait;
                if need && !has {
                    err(format!("{p}.{name}"), "required");
                } else if !need && has && !optional {
                    err(format!("{p}.{name}"), "not_allowed");
                }
            }
            if s.wait_ms.is_some_and(|w| w > 60_000) {
                err(format!("{p}.wait_ms"), "out_of_range");
            }
            if s.window.as_ref().is_some_and(|w| w.process.trim().is_empty()) {
                err(format!("{p}.window.process"), "required");
            }

            // Quy tắc 3: selector.
            if let Some(target) = &s.target {
                if target.is_empty() {
                    err(format!("{p}.target"), "required");
                }
                let mut seen_patch = false;
                let mut seen_xy = false;
                for (ti, sel) in target.iter().enumerate() {
                    let sp = format!("{p}.target[{ti}]");
                    let form = sel.form();
                    if form.is_none() {
                        err(sp.clone(), "selector_form");
                    }
                    if sel.at.is_some() && form != Some(SelectorForm::Xy) {
                        err(format!("{sp}.at"), "not_allowed");
                    }
                    if let Some([fx, fy]) = sel.xy_in_window {
                        if !(0.0..=1.0).contains(&fx) || !(0.0..=1.0).contains(&fy) {
                            err(format!("{sp}.xy_in_window"), "out_of_range");
                        }
                    }
                    match &sel.patch {
                        Some(pa) => {
                            seen_patch = true;
                            if form == Some(SelectorForm::Xy) {
                                err(format!("{sp}.patch"), "not_allowed");
                            }
                            if !patch_ids.insert(pa.id.clone()) {
                                err(format!("{sp}.patch.id"), "duplicate");
                            }
                        }
                        None => {
                            if seen_patch || seen_xy {
                                // gốc sau selector có patch, hoặc gốc sau xy_in_window
                                err(sp.clone(), "selector_order");
                            }
                            if form == Some(SelectorForm::Xy) {
                                seen_xy = true;
                            }
                        }
                    }
                }
            }

            // Quy tắc 8.
            if s.repeat_safe && !matches!(s.action, Click | Hotkey | Type) {
                err(format!("{p}.repeat_safe"), "not_allowed");
            }
            if s.repeat_safe && s.approval {
                err(format!("{p}.repeat_safe"), "conflicts_approval");
            }

            // Quy tắc 4, 6: giữ chỗ trong `text`.
            if let Some(text) = &s.text {
                match placeholders(text) {
                    Err(()) => err(format!("{p}.text"), "bad_placeholder"),
                    Ok(list) => {
                        for ph in list {
                            if let Placeholder::Input(n) = ph {
                                if !input_names.contains(n) {
                                    err(format!("{p}.text"), "unknown_input");
                                }
                            }
                        }
                    }
                }
            }
            // Quy tắc 5: `expect`/`skip_if` không bí mật, không xy.
            let conds = s.expect.iter().enumerate().map(|(i, c)| (format!("{p}.expect[{i}]"), c));
            let conds = conds.chain(s.skip_if.iter().map(|c| (format!("{p}.skip_if"), c)));
            for (cp, c) in conds {
                if let Some(sel) = c.selector() {
                    if sel.xy_in_window.is_some() || sel.form().is_none() || sel.patch.is_some() || sel.at.is_some() {
                        err(cp.clone(), "selector_form");
                    }
                }
                for st in c.strings() {
                    match placeholders(st) {
                        Err(()) => err(cp.clone(), "bad_placeholder"),
                        Ok(list) => {
                            for ph in list {
                                match ph {
                                    Placeholder::Secret(_) => err(cp.clone(), "secret_not_allowed"),
                                    Placeholder::Input(n) if !input_names.contains(n) => {
                                        err(cp.clone(), "unknown_input")
                                    }
                                    Placeholder::Input(_) => {}
                                }
                            }
                        }
                    }
                }
            }
        }
        errs
    }

    /// Cảnh báo hiển thị (data-model Step.warnings), tính từ nội dung skill. Không bao giờ bật `approval`.
    pub fn compute_warnings(&self) -> Vec<Vec<&'static str>> {
        let mut copied = false;
        self.steps
            .iter()
            .map(|s| {
                let mut w = Vec::new();
                if let Some(t) = &s.target {
                    if !t.is_empty() && t.iter().all(|x| x.xy_in_window.is_some()) {
                        w.push("coordinates_only");
                    }
                }
                if s.action == Action::Type && s.text.as_deref().is_some_and(str::is_empty) {
                    w.push("text_not_recorded");
                }
                if s.expect.is_empty() {
                    w.push("unverifiable");
                }
                let keys = s.keys.as_deref().map(norm_keys).unwrap_or_default();
                if s.action == Action::Hotkey {
                    if keys == "ctrl+c" || keys == "ctrl+x" {
                        copied = true;
                    } else if keys == "ctrl+v" && !copied {
                        w.push("clipboard_external");
                    }
                }
                if !s.approval && suggests_approval(s, &keys) {
                    w.push("suggest_approval");
                }
                w
            })
            .collect()
    }

    /// Ghi `compute_warnings()` vào từng bước (gọi trước khi lưu).
    pub fn refresh_warnings(&mut self) {
        let all = self.compute_warnings();
        for (s, w) in self.steps.iter_mut().zip(all) {
            s.warnings = w.into_iter().map(String::from).collect();
        }
    }
}

fn suggests_approval(s: &Step, keys: &str) -> bool {
    match s.action {
        Action::Hotkey => APPROVAL_KEYS.contains(&keys),
        Action::Click => {
            let names = APPROVAL_NAMES.iter().map(|n| crate::fold(n)).collect::<Vec<_>>();
            s.target.iter().flatten().any(|sel| {
                let at_name = sel.at.as_ref().and_then(|a| a.name.as_deref());
                // ponytail: so khớp bằng nguyên tên sau fold; "Gửi email" không được gợi ý — chỉ là gợi ý, không phải chốt an toàn.
                [sel.name.as_deref(), sel.ocr_text.as_deref(), at_name]
                    .into_iter()
                    .flatten()
                    .any(|n| names.contains(&crate::fold(n)))
            })
        }
        _ => false,
    }
}
