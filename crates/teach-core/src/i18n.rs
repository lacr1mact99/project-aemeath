//! T020: khoá thông điệp + bảng `vi`/`en` (FR-048). Nhãn bước là tất định: cùng đầu vào → cùng chuỗi.
use serde::{Deserialize, Serialize};
use windows::Win32::Globalization::GetUserDefaultUILanguage;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    Vi,
    En,
}

impl Lang {
    /// Theo ngôn ngữ giao diện Windows: tiếng Anh → `En`, còn lại (kể cả tiếng Việt) → `Vi`.
    pub fn default_from_windows() -> Lang {
        // SAFETY: hàm Win32 không tham số, không có tiền điều kiện.
        let id = unsafe { GetUserDefaultUILanguage() };
        Lang::from_lang_id(id)
    }

    /// `LANGID`: 10 bit thấp là ngôn ngữ chính (0x09 = English, 0x2A = Vietnamese).
    pub fn from_lang_id(id: u16) -> Lang {
        match id & 0x3ff {
            0x09 => Lang::En,
            _ => Lang::Vi,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pause {
    RepairBudget,
    StepTimeout,
    TargetUnresponsive,
    FocusMismatchSecret,
    UnverifiableStep,
    ModelDeclined,
    ApprovalRetryBlocked,
    RetryUnsafe,
    NotDiscriminating,
    InputGap,
    FocusMismatch,
    WindowAmbiguous,
    SecretStep,
    TargetElevated,
}

impl Pause {
    pub const ALL: [Pause; 14] = [
        Pause::RepairBudget,
        Pause::StepTimeout,
        Pause::TargetUnresponsive,
        Pause::FocusMismatchSecret,
        Pause::UnverifiableStep,
        Pause::ModelDeclined,
        Pause::ApprovalRetryBlocked,
        Pause::RetryUnsafe,
        Pause::NotDiscriminating,
        Pause::InputGap,
        Pause::FocusMismatch,
        Pause::WindowAmbiguous,
        Pause::SecretStep,
        Pause::TargetElevated,
    ];

    /// Mã ổn định dùng trong log/bề mặt điều khiển.
    pub fn code(self) -> &'static str {
        match self {
            Pause::RepairBudget => "repair_budget",
            Pause::StepTimeout => "step_timeout",
            Pause::TargetUnresponsive => "target_unresponsive",
            Pause::FocusMismatchSecret => "focus_mismatch_secret",
            Pause::UnverifiableStep => "unverifiable_step",
            Pause::ModelDeclined => "model_declined",
            Pause::ApprovalRetryBlocked => "approval_retry_blocked",
            Pause::RetryUnsafe => "retry_unsafe",
            Pause::NotDiscriminating => "not_discriminating",
            Pause::InputGap => "input_gap",
            Pause::FocusMismatch => "focus_mismatch",
            Pause::WindowAmbiguous => "window_ambiguous",
            Pause::SecretStep => "secret_step",
            Pause::TargetElevated => "target_elevated",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    Invalid,
    Busy,
    NotFound,
    NotReady,
    Conflict,
    /// Nhóm `model_*`.
    Model,
    Io,
}

impl ErrorCode {
    pub const ALL: [ErrorCode; 7] = [
        ErrorCode::Invalid,
        ErrorCode::Busy,
        ErrorCode::NotFound,
        ErrorCode::NotReady,
        ErrorCode::Conflict,
        ErrorCode::Model,
        ErrorCode::Io,
    ];
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Msg {
    /// `Bấm "X" (Role)` / `Click "X" (Role)`
    Click { name: String, role: String },
    /// `Gõ vào ô "X"` / `Type into "X"`
    TypeInto { name: String },
    /// `Nhấn K` / `Press K`
    Press { keys: String },
    Pause(Pause),
    Error(ErrorCode),
}

pub fn text(lang: Lang, msg: &Msg) -> String {
    let vi = lang == Lang::Vi;
    let pick = |v: &str, e: &str| (if vi { v } else { e }).to_string();
    match msg {
        Msg::Click { name, role } => {
            if vi {
                format!("Bấm \"{name}\" ({role})")
            } else {
                format!("Click \"{name}\" ({role})")
            }
        }
        Msg::TypeInto { name } => {
            if vi {
                format!("Gõ vào ô \"{name}\"")
            } else {
                format!("Type into \"{name}\"")
            }
        }
        Msg::Press { keys } => {
            if vi {
                format!("Nhấn {keys}")
            } else {
                format!("Press {keys}")
            }
        }
        Msg::Pause(p) => match p {
            Pause::RepairBudget => pick("Đã hết số lần tự sửa cho bước này.", "The repair budget for this step is used up."),
            Pause::StepTimeout => pick("Bước này chạy quá thời gian cho phép.", "This step exceeded its time limit."),
            Pause::TargetUnresponsive => pick("Ứng dụng đích không phản hồi.", "The target application is not responding."),
            Pause::FocusMismatchSecret => pick(
                "Tiêu điểm không đúng ô cần gõ bí mật; đã dừng để không gõ nhầm chỗ.",
                "Focus is not on the field for the secret; paused so nothing is typed in the wrong place.",
            ),
            Pause::UnverifiableStep => pick(
                "Không thể kiểm chứng bước này; cần bạn xác nhận.",
                "This step cannot be verified; your confirmation is needed.",
            ),
            Pause::ModelDeclined => pick("Mô hình từ chối chọn phần tử cho bước này.", "The model declined to pick an element for this step."),
            Pause::ApprovalRetryBlocked => pick(
                "Bước cần duyệt không được tự thử lại.",
                "A step that needs approval is not retried automatically.",
            ),
            Pause::RetryUnsafe => pick(
                "Không an toàn để thử lại bước này tự động.",
                "It is not safe to retry this step automatically.",
            ),
            Pause::NotDiscriminating => pick(
                "Không phân biệt được phần tử đích với phần tử khác.",
                "The target cannot be told apart from other elements.",
            ),
            Pause::InputGap => pick("Thiếu giá trị đầu vào cho bước này.", "A required input value is missing for this step."),
            Pause::FocusMismatch => pick("Cửa sổ đang được chọn không phải cửa sổ của bước.", "The focused window is not the one this step expects."),
            Pause::WindowAmbiguous => pick("Có nhiều cửa sổ khớp; không rõ cửa sổ nào đúng.", "Several windows match; it is unclear which one is right."),
            Pause::SecretStep => pick("Bước nhập bí mật; cần bạn gõ trực tiếp.", "This is a secret-entry step; please type it yourself."),
            Pause::TargetElevated => pick(
                "Ứng dụng đích chạy với quyền cao hơn; Teach không thể điều khiển.",
                "The target application runs elevated; Teach cannot control it.",
            ),
        },
        Msg::Error(c) => match c {
            ErrorCode::Invalid => pick("Yêu cầu không hợp lệ.", "Invalid request."),
            ErrorCode::Busy => pick("Đang bận với việc khác.", "Busy with another task."),
            ErrorCode::NotFound => pick("Không tìm thấy.", "Not found."),
            ErrorCode::NotReady => pick("Chưa sẵn sàng (cần chạy thử và xác nhận trước).", "Not ready (a confirmed trial run is needed first)."),
            ErrorCode::Conflict => pick("Xung đột: nội dung đã bị thay đổi ở nơi khác.", "Conflict: the content was changed elsewhere."),
            ErrorCode::Model => pick("Lỗi khi gọi mô hình.", "Model call failed."),
            ErrorCode::Io => pick("Lỗi đọc/ghi tệp.", "File read/write error."),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn samples() -> Vec<Msg> {
        let mut v = vec![
            Msg::Click { name: "OK".into(), role: "Button".into() },
            Msg::TypeInto { name: "Tên".into() },
            Msg::Press { keys: "Ctrl+S".into() },
        ];
        v.extend(Pause::ALL.map(Msg::Pause));
        v.extend(ErrorCode::ALL.map(Msg::Error));
        v
    }

    #[test]
    fn every_msg_has_vi_and_en() {
        for m in samples() {
            let (vi, en) = (text(Lang::Vi, &m), text(Lang::En, &m));
            assert!(!vi.is_empty() && !en.is_empty(), "{m:?}");
            if matches!(m, Msg::Pause(_) | Msg::Error(_)) {
                assert_ne!(vi, en, "{m:?}");
            }
        }
    }

    #[test]
    fn labels_are_deterministic() {
        let c = Msg::Click { name: "Lưu".into(), role: "Button".into() };
        assert_eq!(text(Lang::Vi, &c), "Bấm \"Lưu\" (Button)");
        assert_eq!(text(Lang::En, &c), "Click \"Lưu\" (Button)");
        assert_eq!(text(Lang::En, &Msg::Press { keys: "F5".into() }), "Press F5");
    }

    #[test]
    fn lang_id_mapping() {
        assert_eq!(Lang::from_lang_id(0x0409), Lang::En);
        assert_eq!(Lang::from_lang_id(0x042A), Lang::Vi);
        assert_eq!(Lang::from_lang_id(0x040C), Lang::Vi);
    }
}
