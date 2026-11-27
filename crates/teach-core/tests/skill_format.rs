//! T017: round-trip ví dụ của contracts/skill-format.md, mỗi quy tắc hợp lệ một ca sai, dấu vân tay thực thi.
use teach_core::skill::*;

const EXAMPLE: &str = include_str!("fixtures/skills/format_example.yaml");

fn example() -> Skill {
    Skill::from_yaml(EXAMPLE).expect("ví dụ trong contract phải tải được")
}

fn codes(s: &Skill) -> Vec<(String, &'static str)> {
    s.validate().into_iter().map(|e| (e.field, e.code)).collect()
}

fn has(s: &Skill, field: &str, code: &str) -> bool {
    codes(s).iter().any(|(f, c)| f == field && *c == code)
}

#[test]
fn example_valid_and_round_trips() {
    let s = example();
    assert!(s.validate().is_empty(), "{:?}", s.validate());
    let back = Skill::from_yaml(&s.to_yaml().unwrap()).unwrap();
    assert_eq!(s, back);
    // `confirmed.fingerprint` trong ví dụ là giá trị minh hoạ → draft.
    assert_eq!(s.status(), Status::Draft);
}

#[test]
fn rule1_unknown_fields_and_status_rejected() {
    assert!(Skill::from_yaml(&EXAMPLE.replace("version: 1\n", "version: 1\nstatus: ready\n")).is_err());
    assert!(Skill::from_yaml(&EXAMPLE.replace("version: 1\n", "version: 1\nlast_confirmed_run: x\n")).is_err());
    assert!(Skill::from_yaml(&EXAMPLE.replace("    intent: Mở Excel\n", "    intent: Mở Excel\n    foo: 1\n")).is_err());
    let mut s = example();
    s.version = 2;
    assert!(has(&s, "version", "unsupported_version"));
}

#[test]
fn rule2_fields_per_action() {
    let mut s = example();
    s.steps[1].text = Some("x".into()); // click có text
    assert!(has(&s, "steps[1].text", "not_allowed"));
    let mut s = example();
    s.steps[2].target = None; // type thiếu target
    assert!(has(&s, "steps[2].target", "required"));
    let mut s = example();
    s.steps[0].action = Action::Wait;
    s.steps[0].launch = None;
    s.steps[0].wait_ms = Some(60_001);
    assert!(has(&s, "steps[0].wait_ms", "out_of_range"));
}

#[test]
fn rule3_selector_form_and_order() {
    let mut s = example();
    let t = s.steps[1].target.as_mut().unwrap();
    t[0].name = Some("Lưu".into()); // automation_id + name = hai dạng
    assert!(has(&s, "steps[1].target[0]", "selector_form"));

    let mut s = example();
    let t = s.steps[1].target.as_mut().unwrap();
    t[0].at = Some(At { role: "Button".into(), name: None });
    assert!(has(&s, "steps[1].target[0].at", "not_allowed"));

    let mut s = example();
    let t = s.steps[1].target.as_mut().unwrap();
    t.swap(3, 4); // selector có patch đứng trước selector gốc xy
    assert!(has(&s, "steps[1].target[4]", "selector_order"));

    let mut s = example();
    let t = s.steps[1].target.as_mut().unwrap();
    t.swap(2, 3); // gốc đứng sau xy_in_window
    assert!(has(&s, "steps[1].target[3]", "selector_order"));

    let mut s = example();
    let t = s.steps[1].target.as_mut().unwrap();
    t[3].xy_in_window = Some([1.2, 0.5]);
    assert!(has(&s, "steps[1].target[3].xy_in_window", "out_of_range"));

    let mut s = example();
    let t = s.steps[1].target.as_mut().unwrap();
    t[3].patch = t[4].patch.clone();
    assert!(has(&s, "steps[1].target[3].patch", "not_allowed"));
}

#[test]
fn rule4_placeholders() {
    let mut s = example();
    s.steps[2].text = Some("BaoCao-{{khong_co}}.pdf".into());
    assert!(has(&s, "steps[2].text", "unknown_input"));
    let mut s = example();
    s.steps[2].text = Some("{{secret:Hoa}}".into());
    assert!(has(&s, "steps[2].text", "bad_placeholder"));
}

#[test]
fn rule5_expect_no_secret_no_xy() {
    let mut s = example();
    s.steps[2].expect = vec![Condition::TargetValueEquals("{{secret:cong_bao_cao}}".into())];
    assert!(has(&s, "steps[2].expect[0]", "secret_not_allowed"));
    let mut s = example();
    s.steps[1].skip_if = Some(Condition::ElementExists(Selector { xy_in_window: Some([0.1, 0.1]), ..Default::default() }));
    assert!(has(&s, "steps[1].skip_if", "selector_form"));
}

#[test]
fn rule6_no_escape() {
    let mut s = example();
    s.steps[2].text = Some("a {{ b".into());
    assert!(has(&s, "steps[2].text", "bad_placeholder"));
}

#[test]
fn rule8_repeat_safe() {
    let mut s = example();
    s.steps[3].repeat_safe = true; // bước có approval: true
    assert!(has(&s, "steps[3].repeat_safe", "conflicts_approval"));
    let mut s = example();
    s.steps[0].repeat_safe = true; // launch
    assert!(has(&s, "steps[0].repeat_safe", "not_allowed"));
}

#[test]
fn names() {
    let mut s = example();
    s.name = "Xuat Bao Cao".into();
    assert!(has(&s, "name", "invalid_name"));
    let mut s = example();
    s.inputs[0].name = "secret".into();
    assert!(has(&s, "inputs[0].name", "invalid_name"));
    let mut s = example();
    s.steps[0].intent = "a\nb".into();
    assert!(has(&s, "steps[0].intent", "invalid_intent"));
}

#[test]
fn warnings() {
    let mut s = example();
    s.steps[3].approval = false;
    let w = s.compute_warnings();
    assert!(w[3].contains(&"unverifiable"), "expect: [] → unverifiable");
    assert!(!w[1].contains(&"coordinates_only"));

    s.steps[1].target = Some(vec![Selector { xy_in_window: Some([0.5, 0.5]), ..Default::default() }]);
    assert!(s.compute_warnings()[1].contains(&"coordinates_only"));

    // suggest_approval: nút "Gửi" (so khớp bỏ dấu), không tự bật approval.
    s.steps[1].target = Some(vec![Selector { role: Some("Button".into()), name: Some("GUI".into()), ..Default::default() }]);
    assert!(s.compute_warnings()[1].contains(&"suggest_approval"));
    s.refresh_warnings();
    assert!(!s.steps[1].approval);

    // clipboard_external: Ctrl+V không có Ctrl+C trước.
    let mut paste = s.steps[0].clone();
    paste.action = Action::Hotkey;
    paste.launch = None;
    paste.keys = Some("Ctrl + V".into());
    let mut copy = paste.clone();
    copy.keys = Some("Ctrl+C".into());
    s.steps = vec![paste.clone()];
    assert!(s.compute_warnings()[0].contains(&"clipboard_external"));
    s.steps = vec![copy, paste];
    assert!(!s.compute_warnings()[1].contains(&"clipboard_external"));

    let mut del = s.steps[0].clone();
    del.keys = Some("Shift+Delete".into());
    s.steps = vec![del];
    assert!(s.compute_warnings()[0].contains(&"suggest_approval"));
}

#[test]
fn fingerprint_stable_for_non_exec_changes() {
    let base = example();
    let fp = base.exec_fingerprint();
    assert_eq!(fp.len(), 16);

    let mut s = base.clone();
    s.steps[1].intent = "khác".into();
    s.description = "khác".into();
    s.steps[1].warnings = vec!["unverifiable".into()];
    assert_eq!(s.exec_fingerprint(), fp, "intent/description/warnings");

    let mut s = base.clone();
    let t = s.steps[1].target.as_mut().unwrap();
    t.push(Selector {
        role: Some("Button".into()),
        name: Some("Save".into()),
        patch: Some(Patch { id: "p-x-3".into(), run: "x".into(), pending_review: true }),
        ..Default::default()
    });
    assert_eq!(s.exec_fingerprint(), fp, "thêm selector tự vá");
    s.steps[1].target.as_mut().unwrap()[4].patch.as_mut().unwrap().pending_review = false;
    assert_eq!(s.exec_fingerprint(), fp, "Giữ tự vá");
    s.steps[1].target.as_mut().unwrap().remove(4);
    assert_eq!(s.exec_fingerprint(), fp, "Hoàn tác tự vá");
}

#[test]
fn fingerprint_changes_for_exec_changes() {
    let base = example();
    let fp = base.exec_fingerprint();
    let changes: Vec<(&str, Box<dyn Fn(&mut Skill)>)> = vec![
        ("text", Box::new(|s| s.steps[2].text = Some("x".into()))),
        ("expect", Box::new(|s| s.steps[1].expect.clear())),
        ("approval", Box::new(|s| s.steps[1].approval = true)),
        ("repeat_safe", Box::new(|s| s.steps[1].repeat_safe = false)),
        ("order", Box::new(|s| s.steps.swap(1, 2))),
        ("orig selector", Box::new(|s| s.steps[1].target.as_mut().unwrap()[0].automation_id = Some("X".into()))),
    ];
    for (what, f) in changes {
        let mut s = base.clone();
        f(&mut s);
        assert_ne!(s.exec_fingerprint(), fp, "{what}");
    }
}

#[test]
fn ready_iff_fingerprint_matches() {
    let mut s = example();
    s.confirmed = Some(Confirmed { run: "r".into(), fingerprint: s.exec_fingerprint() });
    assert_eq!(s.status(), Status::Ready);
    s.steps[1].intent = "sửa nhãn".into();
    assert_eq!(s.status(), Status::Ready);
    s.steps[2].text = Some("khác".into());
    assert_eq!(s.status(), Status::Draft);
}

#[test]
fn substitute_keeps_secrets() {
    let mut m = std::collections::HashMap::new();
    m.insert("tuan".to_string(), "2026-W47".to_string());
    assert_eq!(substitute("BaoCao-{{tuan}}.pdf {{secret:k}}", &m), "BaoCao-2026-W47.pdf {{secret:k}}");
}
