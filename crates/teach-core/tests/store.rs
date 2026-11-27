//! T015: kho dữ liệu — đồng thời (luồng + tiến trình), not_found, FR-038, ghi nguyên tử, settings, prune_runs (FR-026).
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use teach_core::skill::Skill;
use teach_core::store::*;

static SEQ: AtomicU32 = AtomicU32::new(0);

struct Tmp(PathBuf);
impl Tmp {
    fn new() -> Tmp {
        let p = std::env::temp_dir().join(format!(
            "teach-store-test-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&p).unwrap();
        Tmp(p)
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        _ = fs::remove_dir_all(&self.0);
    }
}

const SKILL: &str = "version: 1\nname: demo\ndescription: '0'\nsteps:\n  - id: 1\n    intent: wait\n    action: wait\n    wait_ms: 10\n";

fn skill() -> Skill {
    Skill::from_yaml(SKILL).unwrap()
}

fn bump_description(s: &mut Skill) -> Result<(), ()> {
    s.description = (s.description.parse::<u32>().unwrap() + 1).to_string();
    Ok(())
}

#[test]
fn threads_update_different_fields() {
    let t = Tmp::new();
    let st = Store::open(&t.0);
    st.create_skill("demo", &skill()).unwrap();
    std::thread::scope(|sc| {
        for _ in 0..2 {
            sc.spawn(|| {
                for _ in 0..25 {
                    st.update_skill("demo", bump_description).unwrap();
                }
            });
        }
        sc.spawn(|| {
            st.update_skill::<()>("demo", |s| {
                s.source_trace = Some("t1".into());
                Ok(())
            })
            .unwrap();
        });
    });
    let (s, _) = st.load_skill("demo").unwrap();
    assert_eq!(s.description, "50");
    assert_eq!(s.source_trace.as_deref(), Some("t1"));
}

#[test]
fn two_processes_do_not_lose_updates() {
    if let Ok(dir) = std::env::var("TEACH_STORE_CHILD_DIR") {
        let st = Store::open(dir);
        for _ in 0..50 {
            st.update_skill("demo", bump_description).unwrap();
        }
        return;
    }
    let t = Tmp::new();
    Store::open(&t.0).create_skill("demo", &skill()).unwrap();
    let exe = std::env::current_exe().unwrap();
    let kids: Vec<_> = (0..2)
        .map(|_| {
            Command::new(&exe)
                .args(["--exact", "two_processes_do_not_lose_updates", "--nocapture", "--test-threads=1"])
                .env("TEACH_STORE_CHILD_DIR", &t.0)
                .spawn()
                .unwrap()
        })
        .collect();
    for mut k in kids {
        assert!(k.wait().unwrap().success());
    }
    // Nếu con không chạy test nào, bộ đếm sẽ không đạt 100.
    let (s, _) = Store::open(&t.0).load_skill("demo").unwrap();
    assert_eq!(s.description, "100");
}

#[test]
fn update_missing_skill_is_not_found_and_does_not_recreate_dir() {
    let t = Tmp::new();
    let st = Store::open(&t.0);
    st.create_skill("demo", &skill()).unwrap();
    fs::remove_dir_all(st.skill_dir("demo")).unwrap();
    let r = st.update_skill("demo", bump_description);
    assert_eq!(r, Err(UpdateError::Store(StoreError::NotFound)));
    assert!(!st.skill_dir("demo").exists());
    assert_eq!(st.load_skill("demo").unwrap_err(), StoreError::NotFound);
}

#[test]
fn closure_error_leaves_file_untouched_and_names_validated() {
    let t = Tmp::new();
    let st = Store::open(&t.0);
    let rev = st.create_skill("demo", &skill()).unwrap();
    let r = st.update_skill("demo", |s| {
        s.description = "changed".into();
        Err("nope")
    });
    assert_eq!(r, Err(UpdateError::Apply("nope")));
    assert_eq!(st.load_skill("demo").unwrap().1, rev);
    assert!(matches!(st.load_skill("..\\x"), Err(StoreError::Invalid(_))));
    assert_eq!(st.create_skill("demo", &skill()), Err(StoreError::Conflict));
}

#[test]
fn revision_and_save_conflict() {
    let t = Tmp::new();
    let st = Store::open(&t.0);
    let rev = st.create_skill("demo", &skill()).unwrap();
    let bytes = fs::read(st.skill_path("demo")).unwrap();
    assert_eq!(rev, format!("{:016x}", teach_core::fnv1a64(&bytes)));
    let (mut s, rev0) = st.load_skill("demo").unwrap();
    st.update_skill("demo", bump_description).unwrap();
    s.description = "mine".into();
    assert_eq!(st.save_skill("demo", &s, &rev0), Err(StoreError::Conflict));
    assert_eq!(st.load_skill("demo").unwrap().0.description, "1");
    let (_, rev1) = st.load_skill("demo").unwrap();
    st.save_skill("demo", &s, &rev1).unwrap();
    assert_eq!(st.load_skill("demo").unwrap().0.description, "mine");
}

#[test]
fn data_dir_rejections() {
    let no_env = |_: &str| None;
    let fixed = |_: char| 3u32; // DRIVE_FIXED
    let v = |p: &str| validate_data_dir_with(Path::new(p), &no_env, &fixed);
    assert!(v(r"\\server\share\teach").unwrap_err().reason.contains("UNC"));
    assert!(v(r"\\?\UNC\server\share\teach").is_err());
    assert!(v(r"relative\dir").is_err());
    assert!(v(r"C:\Data\teach").is_ok());

    // ổ ánh xạ mạng (DRIVE_REMOTE = 4)
    let remote = |c: char| if c == 'Z' { 4u32 } else { 3 };
    let r = validate_data_dir_with(Path::new(r"Z:\teach"), &no_env, &remote);
    assert!(r.unwrap_err().reason.contains("network drive"));
    assert!(validate_data_dir_with(Path::new(r"C:\teach"), &no_env, &remote).is_ok());

    // OneDrive theo biến môi trường
    let env = |k: &str| (k == "OneDrive").then(|| r"C:\Users\Bob\OneDrive".to_string());
    assert!(validate_data_dir_with(Path::new(r"c:\users\bob\onedrive\teach"), &env, &fixed).is_err());
    assert!(validate_data_dir_with(Path::new(r"C:\Users\Bob\OneDrive"), &env, &fixed).is_err());
    assert!(validate_data_dir_with(Path::new(r"C:\Users\Bob\OneDrive2\teach"), &env, &fixed).is_err()); // thư mục tên OneDrive*
    assert!(validate_data_dir_with(Path::new(r"C:\Users\Bob\Documents\teach"), &env, &fixed).is_ok());
    let env2 = |k: &str| (k == "OneDriveCommercial").then(|| r"D:\Sync\Corp".to_string());
    assert!(validate_data_dir_with(Path::new(r"D:\Sync\Corp\x"), &env2, &fixed).is_err());
    assert!(validate_data_dir_with(Path::new(r"D:\Sync\Corporate"), &env2, &fixed).is_ok());

    // đường thật: thư mục tạm cục bộ phải được chấp nhận bằng hàm thật
    let t = Tmp::new();
    assert!(validate_data_dir(&t.0.join("teach")).is_ok());
}

#[test]
fn write_location_rejects_bad_dir_and_pointer_is_followed() {
    let t = Tmp::new();
    let default_root = t.0.join("default");
    assert!(matches!(Store::write_location(&default_root, Path::new(r"\\server\share")), Err(StoreError::Invalid(_))));
    assert!(!default_root.join("location.yaml").exists());
    assert_eq!(Store::open_with_pointer(&default_root).unwrap().root(), default_root);
    let target = t.0.join("elsewhere");
    Store::write_location(&default_root, &target).unwrap();
    assert_eq!(Store::open_with_pointer(&default_root).unwrap().root(), target);
}

#[test]
fn atomic_write_leaves_no_partial_files() {
    let t = Tmp::new();
    let st = Store::open(&t.0);
    let p = t.0.join("sub").join("f.yaml");
    st.write_text(&p, "a: 1\n").unwrap();
    st.write_text(&p, "a: 2\n").unwrap();
    assert_eq!(fs::read_to_string(&p).unwrap(), "a: 2\n");
    let names: Vec<_> = fs::read_dir(p.parent().unwrap()).unwrap().map(|e| e.unwrap().file_name()).collect();
    assert_eq!(names, vec![std::ffi::OsString::from("f.yaml")]);
    // write_atomic không tạo thư mục
    assert!(st.write_atomic(&t.0.join("nope").join("x"), b"x").is_err());
    assert!(!t.0.join("nope").exists());
}

#[test]
fn free_space_threshold() {
    let t = Tmp::new();
    let st = Store::open(t.0.join("not-yet-created"));
    assert!(st.clone().with_min_free_bytes(0).free_space_ok());
    let huge = st.with_min_free_bytes(u64::MAX);
    assert!(!huge.free_space_ok());
    assert_eq!(huge.write_text(&t.0.join("x.yaml"), "a"), Err(StoreError::DiskFull));
    assert!(!t.0.join("x.yaml").exists());
}

#[test]
fn jsonl_append() {
    let t = Tmp::new();
    let st = Store::open(&t.0);
    let p = st.run_dir("demo", "20260101-000000").join("run.jsonl");
    st.append_jsonl(&p, &serde_json::json!({"a": 1})).unwrap();
    st.append_jsonl(&p, &serde_json::json!({"b": "x"})).unwrap();
    assert_eq!(fs::read_to_string(&p).unwrap(), "{\"a\":1}\n{\"b\":\"x\"}\n");
}

#[test]
fn settings_defaults_roundtrip_and_unknown_field_rejected() {
    let t = Tmp::new();
    let st = Store::open(&t.0);
    let d = st.load_settings().unwrap(); // chưa có file
    assert_eq!(d.version, 1);
    assert_eq!((d.budget.max_minutes, d.budget.max_model_calls, d.budget.max_repairs_per_step), (30, 20, 2));
    assert_eq!(d.keep_runs_per_skill, 50);
    assert_eq!(d.hotkeys.emergency_stop, "Ctrl+Alt+Esc");
    let mut s = d.clone();
    s.model.name = "deepseek-chat".into();
    s.budget.max_minutes = 45;
    st.save_settings(&s).unwrap();
    assert_eq!(st.load_settings().unwrap(), s);
    assert!(!fs::read_to_string(st.settings_path()).unwrap().contains("api_key"));

    const SECRET: &str = "sk-SUPERSECRET123";
    for bad in [
        format!("version: 1\napi_key: {SECRET}\n"),
        format!("version: 1\nmodel: {{name: x, api_key: {SECRET}}}\n"),
    ] {
        fs::write(st.settings_path(), bad).unwrap();
        match st.load_settings() {
            Err(StoreError::Invalid(m)) => assert!(!m.contains(SECRET) && !m.contains("SUPERSECRET"), "{m}"),
            other => panic!("{other:?}"),
        }
    }
    fs::write(st.settings_path(), "version: 2\n").unwrap();
    assert!(matches!(st.load_settings(), Err(StoreError::Invalid(_))));
}

#[test]
fn prune_runs_keeps_newest_and_active_and_spares_other_files() {
    let t = Tmp::new();
    let st = Store::open(&t.0);
    st.create_skill("demo", &skill()).unwrap();
    st.write_text(&st.schedules_path(), "version: 1\nschedules: []\n").unwrap();
    st.append_jsonl(&st.schedule_history_path(), &serde_json::json!({"e": 1})).unwrap();
    let ids: Vec<String> = (0..51).map(|i| format!("20260101-0000{i:02}")).collect();
    for id in &ids {
        fs::create_dir_all(st.run_dir("demo", id).join("shots")).unwrap();
        fs::write(st.run_dir("demo", id).join("run.jsonl"), "{}\n").unwrap();
    }
    // run cũ nhất đang chạy → giữ
    let n = st.prune_runs("demo", 50, &[ids[0].clone()]).unwrap();
    assert_eq!(n, 0);
    assert!(st.run_dir("demo", &ids[0]).exists());

    // không còn run active: cũ nhất bị xoá, còn 50 mới nhất
    assert_eq!(st.prune_runs("demo", 50, &[]).unwrap(), 1);
    assert!(!st.run_dir("demo", &ids[0]).exists());
    assert_eq!(fs::read_dir(st.runs_dir("demo")).unwrap().count(), 50);
    assert!(st.run_dir("demo", &ids[50]).exists());

    // active ở giữa vùng bị xoá được giữ, các cái khác cũ hơn `keep` bị xoá
    assert_eq!(st.prune_runs("demo", 10, &[ids[5].clone()]).unwrap(), 39);
    assert!(st.run_dir("demo", &ids[5]).exists());
    assert_eq!(fs::read_dir(st.runs_dir("demo")).unwrap().count(), 11);

    assert!(st.skill_path("demo").exists());
    assert!(st.schedules_path().exists());
    assert!(st.schedule_history_path().exists());
    // skill chưa có run → 0
    assert_eq!(st.prune_runs("other", 50, &[]).unwrap(), 0);
}
