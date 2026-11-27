//! T014: thư mục dữ liệu, ghi nguyên tử, khoá liên tiến trình theo skill, dọn log, cài đặt (FR-007/013/026/038/052).
//! Bố cục thư mục: data-model.md §Bố cục thư mục.
use crate::i18n::Lang;
use crate::skill::Skill;
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::os::windows::ffi::OsStrExt;
use std::path::{Component, Path, PathBuf, Prefix};
use std::sync::atomic::{AtomicU64, Ordering};
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0};
use windows::Win32::Storage::FileSystem::{
    GetDiskFreeSpaceExW, GetDriveTypeW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
};
use windows::Win32::System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject};
use windows::core::PCWSTR;

/// FR-052: cần ≥ 500 MB trống trên ổ chứa thư mục dữ liệu.
pub const MIN_FREE_BYTES: u64 = 500 * 1024 * 1024;
const DRIVE_REMOTE: u32 = 4; // winbase.h; không có trong feature đang bật của `windows`
const LOCK_TIMEOUT_MS: u32 = 30_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoreError {
    NotFound,
    Conflict,
    Io(String),
    Invalid(String),
    DiskFull,
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::NotFound => write!(f, "not_found"),
            StoreError::Conflict => write!(f, "conflict"),
            StoreError::Io(m) => write!(f, "io: {m}"),
            StoreError::Invalid(m) => write!(f, "invalid: {m}"),
            StoreError::DiskFull => write!(f, "disk_full"),
        }
    }
}

impl std::error::Error for StoreError {}

fn io_err(e: io::Error) -> StoreError {
    match e.kind() {
        io::ErrorKind::NotFound => StoreError::NotFound,
        _ => match e.raw_os_error() {
            Some(112) | Some(39) => StoreError::DiskFull, // ERROR_DISK_FULL / ERROR_HANDLE_DISK_FULL
            _ => StoreError::Io(e.to_string()),
        },
    }
}

/// Lỗi của `update_skill`: lỗi kho hoặc lỗi do closure của người gọi trả về.
#[derive(Debug, PartialEq, Eq)]
pub enum UpdateError<E> {
    Store(StoreError),
    Apply(E),
}

impl<E> From<StoreError> for UpdateError<E> {
    fn from(e: StoreError) -> Self {
        UpdateError::Store(e)
    }
}

fn wide(s: &OsStr) -> Vec<u16> {
    s.encode_wide().chain(Some(0)).collect()
}

/// `revision` = FNV-1a nội dung file (data-model §Ghi đồng thời).
pub fn revision_of(bytes: &[u8]) -> String {
    format!("{:016x}", crate::fnv1a64(bytes))
}

/// Tên skill = tên thư mục: `[a-z0-9-]`, 1–64 ký tự (cũng chặn `..` và dấu `\`).
pub fn valid_skill_name(name: &str) -> bool {
    (1..=64).contains(&name.len()) && name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn check_name(name: &str) -> Result<(), StoreError> {
    if valid_skill_name(name) { Ok(()) } else { Err(StoreError::Invalid("skill name".into())) }
}

/// Mutex có tên liên tiến trình; thả khi drop.
struct SkillLock(HANDLE);

impl SkillLock {
    fn acquire(name: &str) -> Result<SkillLock, StoreError> {
        let n = wide(OsStr::new(&format!("Local\\teach-skill-{name}")));
        // SAFETY: `n` kết thúc bằng NUL và sống qua lời gọi.
        let h = unsafe { CreateMutexW(None, false, PCWSTR(n.as_ptr())) }.map_err(|e| StoreError::Io(e.to_string()))?;
        // SAFETY: `h` là handle mutex hợp lệ vừa tạo.
        let r = unsafe { WaitForSingleObject(h, LOCK_TIMEOUT_MS) };
        if r == WAIT_OBJECT_0 || r == WAIT_ABANDONED {
            // WAIT_ABANDONED: tiến trình giữ trước đã chết; ta vẫn sở hữu mutex, file ghi nguyên tử nên nhất quán.
            Ok(SkillLock(h))
        } else {
            // SAFETY: đóng handle ta vừa mở.
            unsafe { _ = CloseHandle(h) };
            Err(StoreError::Io("lock timeout".into()))
        }
    }
}

impl Drop for SkillLock {
    fn drop(&mut self) {
        // SAFETY: handle do ta sở hữu, drop trên cùng luồng đã acquire.
        unsafe {
            _ = ReleaseMutex(self.0);
            _ = CloseHandle(self.0);
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Location {
    data_dir: String,
}

#[derive(Clone, Debug)]
pub struct Store {
    root: PathBuf,
    min_free: u64,
}

static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

impl Store {
    /// Gốc dữ liệu tường minh (dùng cho test).
    pub fn open(root: impl Into<PathBuf>) -> Store {
        Store { root: root.into(), min_free: MIN_FREE_BYTES }
    }

    /// `%LOCALAPPDATA%\teach\`, theo con trỏ `location.yaml` nếu có (FR-007).
    pub fn open_default() -> Result<Store, StoreError> {
        let base = std::env::var_os("LOCALAPPDATA").ok_or_else(|| StoreError::Io("LOCALAPPDATA not set".into()))?;
        Store::open_with_pointer(&Path::new(&base).join("teach"))
    }

    /// `default_root` chứa `location.yaml`; chưa có → dùng chính `default_root`.
    pub fn open_with_pointer(default_root: &Path) -> Result<Store, StoreError> {
        match fs::read_to_string(default_root.join("location.yaml")) {
            Ok(s) => {
                let l: Location = serde_saphyr::from_str(&s).map_err(|_| StoreError::Invalid("location.yaml".into()))?;
                Ok(Store::open(l.data_dir))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Store::open(default_root)),
            Err(e) => Err(io_err(e)),
        }
    }

    /// Đổi thư mục dữ liệu: kiểm tra FR-038 rồi ghi con trỏ `location.yaml` ở `default_root`. Không di chuyển dữ liệu cũ.
    pub fn write_location(default_root: &Path, new_dir: &Path) -> Result<(), StoreError> {
        validate_data_dir(new_dir).map_err(|r| StoreError::Invalid(r.reason))?;
        let l = Location { data_dir: new_dir.to_string_lossy().into_owned() };
        let y = serde_saphyr::to_string(&l).map_err(|e| StoreError::Invalid(e.to_string()))?;
        let s = Store::open(default_root);
        s.write_text(&default_root.join("location.yaml"), &y)
    }

    /// Ngưỡng dung lượng trống (test).
    pub fn with_min_free_bytes(mut self, n: u64) -> Store {
        self.min_free = n;
        self
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    // --- bố cục ---
    pub fn settings_path(&self) -> PathBuf {
        self.root.join("settings.yaml")
    }
    pub fn schedules_path(&self) -> PathBuf {
        self.root.join("schedules.yaml")
    }
    pub fn schedule_history_path(&self) -> PathBuf {
        self.root.join("schedule_history.jsonl")
    }
    pub fn trace_dir(&self, trace_id: &str) -> PathBuf {
        self.root.join("traces").join(trace_id)
    }
    pub fn skills_dir(&self) -> PathBuf {
        self.root.join("skills")
    }
    pub fn skill_dir(&self, name: &str) -> PathBuf {
        self.skills_dir().join(name)
    }
    pub fn skill_path(&self, name: &str) -> PathBuf {
        self.skill_dir(name).join("skill.yaml")
    }
    pub fn label_log_path(&self, name: &str) -> PathBuf {
        self.skill_dir(name).join("label_log.jsonl")
    }
    pub fn runs_dir(&self, skill: &str) -> PathBuf {
        self.root.join("runs").join(skill)
    }
    pub fn run_dir(&self, skill: &str, run_id: &str) -> PathBuf {
        self.runs_dir(skill).join(run_id)
    }

    /// FR-052: ≥ ngưỡng trống trên ổ chứa thư mục dữ liệu. Không đọc được → `false`.
    pub fn free_space_ok(&self) -> bool {
        let mut p = self.root.as_path();
        while !p.exists() {
            match p.parent() {
                Some(x) => p = x,
                None => break,
            }
        }
        let w = wide(p.as_os_str());
        let mut avail = 0u64;
        // SAFETY: `w` kết thúc NUL; `avail` là con trỏ ghi hợp lệ.
        let ok = unsafe { GetDiskFreeSpaceExW(PCWSTR(w.as_ptr()), Some(&mut avail), None, None) };
        ok.is_ok() && avail >= self.min_free
    }

    // --- ghi ---
    /// Ghi nguyên tử: file tạm cùng thư mục → `MoveFileExW(REPLACE_EXISTING|WRITE_THROUGH)`. Không tạo thư mục.
    pub fn write_atomic(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        if !self.free_space_ok() {
            return Err(StoreError::DiskFull);
        }
        let dir = path.parent().ok_or_else(|| StoreError::Invalid("path".into()))?;
        let fname = path.file_name().ok_or_else(|| StoreError::Invalid("path".into()))?.to_string_lossy();
        let tmp = dir.join(format!(".{fname}.{}.{}.tmp", std::process::id(), TMP_SEQ.fetch_add(1, Ordering::Relaxed)));
        let res = (|| -> Result<(), StoreError> {
            let mut f = fs::File::create(&tmp).map_err(io_err)?;
            f.write_all(bytes).map_err(io_err)?;
            f.sync_all().map_err(io_err)?;
            drop(f);
            let (a, b) = (wide(tmp.as_os_str()), wide(path.as_os_str()));
            // SAFETY: cả hai chuỗi kết thúc NUL và sống qua lời gọi.
            unsafe {
                MoveFileExW(PCWSTR(a.as_ptr()), PCWSTR(b.as_ptr()), MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)
            }
            .map_err(|e| io_err(io::Error::from_raw_os_error(e.code().0 & 0xFFFF)))
        })();
        if res.is_err() {
            _ = fs::remove_file(&tmp);
        }
        res
    }

    /// Như `write_atomic` nhưng tạo thư mục cha (cho settings, schedules, location…).
    pub fn write_text(&self, path: &Path, text: &str) -> Result<(), StoreError> {
        if let Some(d) = path.parent() {
            fs::create_dir_all(d).map_err(io_err)?;
        }
        self.write_atomic(path, text.as_bytes())
    }

    /// Ghi thêm một dòng JSON rồi flush; tạo thư mục cha.
    pub fn append_jsonl<T: Serialize>(&self, path: &Path, value: &T) -> Result<(), StoreError> {
        let mut line = serde_json::to_string(value).map_err(|e| StoreError::Invalid(e.to_string()))?;
        line.push('\n');
        if let Some(d) = path.parent() {
            fs::create_dir_all(d).map_err(io_err)?;
        }
        let mut f = fs::OpenOptions::new().create(true).append(true).open(path).map_err(io_err)?;
        f.write_all(line.as_bytes()).map_err(io_err)?;
        f.flush().map_err(io_err)
    }

    // --- skill ---
    fn read_skill(&self, name: &str) -> Result<(Skill, String), StoreError> {
        let bytes = fs::read(self.skill_path(name)).map_err(io_err)?;
        let text = std::str::from_utf8(&bytes).map_err(|_| StoreError::Invalid("skill.yaml: utf-8".into()))?;
        let skill = Skill::from_yaml(text).map_err(StoreError::Invalid)?;
        Ok((skill, revision_of(&bytes)))
    }

    fn write_skill(&self, name: &str, skill: &Skill) -> Result<String, StoreError> {
        let y = skill.to_yaml().map_err(StoreError::Invalid)?;
        self.write_atomic(&self.skill_path(name), y.as_bytes())?;
        Ok(revision_of(y.as_bytes()))
    }

    /// Tải skill và `revision` hiện tại của file.
    pub fn load_skill(&self, name: &str) -> Result<(Skill, String), StoreError> {
        check_name(name)?;
        self.read_skill(name)
    }

    /// Tạo skill mới (thất bại `Conflict` nếu đã có). Trả `revision`.
    pub fn create_skill(&self, name: &str, skill: &Skill) -> Result<String, StoreError> {
        check_name(name)?;
        let _lock = SkillLock::acquire(name)?;
        if self.skill_path(name).exists() {
            return Err(StoreError::Conflict);
        }
        fs::create_dir_all(self.skill_dir(name)).map_err(io_err)?;
        self.write_skill(name, skill)
    }

    /// Đường ghi duy nhất cho `skill.yaml` của skill đã có: khoá liên tiến trình, đọc lại, áp, ghi nguyên tử.
    /// Skill không còn → `NotFound`, không tạo lại thư mục. Trả `revision` mới.
    pub fn update_skill<E>(
        &self,
        name: &str,
        f: impl FnOnce(&mut Skill) -> Result<(), E>,
    ) -> Result<String, UpdateError<E>> {
        check_name(name)?;
        let _lock = SkillLock::acquire(name)?;
        let (mut skill, _) = self.read_skill(name)?;
        f(&mut skill).map_err(UpdateError::Apply)?;
        Ok(self.write_skill(name, &skill)?)
    }

    /// `save_skill` của người dùng: ghi `skill` nếu `revision` hiện tại còn bằng `expected`, ngược lại `Conflict`.
    pub fn save_skill(&self, name: &str, skill: &Skill, expected: &str) -> Result<String, StoreError> {
        check_name(name)?;
        let _lock = SkillLock::acquire(name)?;
        let (_, cur) = self.read_skill(name)?;
        if cur != expected {
            return Err(StoreError::Conflict);
        }
        self.write_skill(name, skill)
    }

    /// FR-026: giữ `keep` thư mục run mới nhất (tên = thời điểm nên sắp theo tên), không xoá run trong `active_run_ids`.
    /// Chỉ đụng `runs\<skill>\`. Trả số thư mục đã xoá.
    pub fn prune_runs(&self, skill: &str, keep: usize, active_run_ids: &[String]) -> Result<usize, StoreError> {
        check_name(skill)?;
        let rd = match fs::read_dir(self.runs_dir(skill)) {
            Ok(r) => r,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(0),
            Err(e) => return Err(io_err(e)),
        };
        let mut ids: Vec<String> = Vec::new();
        for e in rd {
            let e = e.map_err(io_err)?;
            if e.file_type().map_err(io_err)?.is_dir() {
                ids.push(e.file_name().to_string_lossy().into_owned());
            }
        }
        ids.sort_unstable_by(|a, b| b.cmp(a)); // mới nhất trước; hậu tố `-2` sắp sau nên mới hơn
        let mut n = 0;
        for id in ids.into_iter().skip(keep).filter(|id| !active_run_ids.contains(id)) {
            fs::remove_dir_all(self.run_dir(skill, &id)).map_err(io_err)?;
            n += 1;
        }
        Ok(n)
    }

    // --- settings ---
    /// Chưa có file → mặc định. Trường lạ/sai kiểu → `Invalid` với thông điệp cố định (không lặp lại giá trị trong file).
    pub fn load_settings(&self) -> Result<Settings, StoreError> {
        let text = match fs::read_to_string(self.settings_path()) {
            Ok(t) => t,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Settings::default()),
            Err(e) => return Err(io_err(e)),
        };
        let s: Settings = serde_saphyr::from_str(&text).map_err(|_| StoreError::Invalid("settings.yaml: unknown or invalid field".into()))?;
        if s.version != 1 {
            return Err(StoreError::Invalid("settings.yaml: unsupported version".into()));
        }
        Ok(s)
    }

    pub fn save_settings(&self, s: &Settings) -> Result<(), StoreError> {
        let y = serde_saphyr::to_string(s).map_err(|e| StoreError::Invalid(e.to_string()))?;
        self.write_text(&self.settings_path(), &y)
    }
}

// --- Settings (data-model §Settings; không có API key) ---

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct Settings {
    pub version: u32,
    pub language: Lang,
    pub model: ModelSettings,
    pub hotkeys: Hotkeys,
    pub budget: Budget,
    pub schedule: ScheduleSettings,
    pub keep_runs_per_skill: u32,
    pub start_with_windows: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct ModelSettings {
    pub base_url: String,
    pub name: String,
    pub timeout_seconds: u32,
    pub data_notice_accepted: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct Hotkeys {
    pub emergency_stop: String,
    pub record_pause: String,
    pub record_stop: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct Budget {
    pub max_minutes: u32,
    pub max_model_calls: u32,
    pub max_repairs_per_step: u32,
    pub max_step_minutes: u32,
    pub manual_prompt_timeout_minutes: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct ScheduleSettings {
    pub idle_minutes: u32,
    pub ask_timeout_minutes: u32,
    pub approval_timeout_minutes: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            version: 1,
            language: Lang::default_from_windows(),
            model: ModelSettings::default(),
            hotkeys: Hotkeys::default(),
            budget: Budget::default(),
            schedule: ScheduleSettings::default(),
            keep_runs_per_skill: 50,
            start_with_windows: false,
        }
    }
}

impl Default for ModelSettings {
    fn default() -> Self {
        ModelSettings { base_url: String::new(), name: String::new(), timeout_seconds: 30, data_notice_accepted: false }
    }
}

impl Default for Hotkeys {
    fn default() -> Self {
        Hotkeys {
            emergency_stop: "Ctrl+Alt+Esc".into(),
            record_pause: "Ctrl+Alt+P".into(),
            record_stop: "Ctrl+Alt+S".into(),
        }
    }
}

impl Default for Budget {
    fn default() -> Self {
        Budget {
            max_minutes: 30,
            max_model_calls: 20,
            max_repairs_per_step: 2,
            max_step_minutes: 3,
            manual_prompt_timeout_minutes: 60,
        }
    }
}

impl Default for ScheduleSettings {
    fn default() -> Self {
        ScheduleSettings { idle_minutes: 3, ask_timeout_minutes: 5, approval_timeout_minutes: 30 }
    }
}

// --- FR-038: thư mục dữ liệu hợp lệ ---

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataDirRejection {
    pub reason: String,
}

fn reject<T>(r: &str) -> Result<T, DataDirRejection> {
    Err(DataDirRejection { reason: r.into() })
}

fn real_drive_type(letter: char) -> u32 {
    let w = wide(OsStr::new(&format!("{letter}:\\")));
    // SAFETY: `w` kết thúc NUL.
    unsafe { GetDriveTypeW(PCWSTR(w.as_ptr())) }
}

fn norm(s: &str) -> String {
    s.replace('/', "\\").trim_end_matches('\\').to_lowercase()
}

/// Từ chối: UNC, ổ mạng/ánh xạ, thư mục OneDrive/đồng bộ đám mây đã biết, đường dẫn (kể cả sau khi phân giải
/// junction/symlink) trỏ lên máy chủ — thư mục cá nhân chuyển hướng lên server rơi vào hai ca trước.
pub fn validate_data_dir(path: &Path) -> Result<(), DataDirRejection> {
    validate_data_dir_with(path, &|k| std::env::var(k).ok(), &real_drive_type)
}

/// Bản có tiêm môi trường và loại ổ (cho test).
pub fn validate_data_dir_with(
    path: &Path,
    env: &dyn Fn(&str) -> Option<String>,
    drive_type: &dyn Fn(char) -> u32,
) -> Result<(), DataDirRejection> {
    check_one(path, env, drive_type)?;
    // Phân giải junction/symlink ở phần đã tồn tại sâu nhất.
    let mut p = path;
    while !p.exists() {
        match p.parent() {
            Some(x) => p = x,
            None => return Ok(()),
        }
    }
    match fs::canonicalize(p) {
        Ok(c) => check_one(&c, env, drive_type),
        Err(_) => Ok(()),
    }
}

fn check_one(
    path: &Path,
    env: &dyn Fn(&str) -> Option<String>,
    drive_type: &dyn Fn(char) -> u32,
) -> Result<(), DataDirRejection> {
    let mut comps = path.components();
    let letter = match comps.next() {
        Some(Component::Prefix(p)) => match p.kind() {
            Prefix::UNC(..) | Prefix::VerbatimUNC(..) => return reject("network share (UNC) path"),
            Prefix::Disk(d) | Prefix::VerbatimDisk(d) => (d as char).to_ascii_uppercase(),
            _ => return reject("device path not allowed"),
        },
        _ => return reject("path must be absolute with a drive letter"),
    };
    if !matches!(comps.next(), Some(Component::RootDir)) {
        return reject("path must be absolute with a drive letter");
    }
    if drive_type(letter) == DRIVE_REMOTE {
        return reject("network drive");
    }
    let mut lower = norm(&path.to_string_lossy());
    if let Some(s) = lower.strip_prefix("\\\\?\\") {
        lower = s.to_string();
    }
    for var in ["OneDrive", "OneDriveCommercial", "OneDriveConsumer"] {
        if let Some(v) = env(var).filter(|v| !v.is_empty()) {
            let v = norm(&v);
            if lower == v || lower.starts_with(&format!("{v}\\")) {
                return reject("OneDrive synced folder");
            }
        }
    }
    const CLOUD: [&str; 5] = ["onedrive", "dropbox", "google drive", "googledrive", "iclouddrive"];
    if lower.split('\\').any(|c| CLOUD.iter().any(|k| c.starts_with(k))) {
        return reject("cloud-synced folder");
    }
    Ok(())
}
