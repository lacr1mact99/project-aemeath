//! M0 spike T011 / quickstart row 0.9: Windows Credential Manager persistence of
//! secrets stored through `keyring` 4 (data-model.md "Secret reference").
//!
//! Measures: with service `teach` and users `secret:spike_test` / `model-api-key`
//! (dummy value) - keyring write/read/delete round trip, the TargetName keyring
//! really used (found via CredEnumerateW, all credentials, name contains `teach`), and the `Persist` value read back
//! with CredReadW (1=SESSION, 2=LOCAL_MACHINE, 3=ENTERPRISE). If Persist is not 2, it
//! repeats write/read/delete with CredWriteW and Persist=LOCAL_MACHINE.
//! Everything it creates is deleted before exit.
//!
//! Run: `cargo run -p teach-core --example spike_secret`
//! Operator reads off: the final line `0.9: PASS|FAIL persist=<n> method=<keyring|CredWriteW>`
//! (PASS = persist is 2 or 3, i.e. survives logoff/reboot; method says who achieved it).
//! Run it on the company machine (profile/GPO may change the result).

use windows::Win32::Security::Credentials::*;
use windows::core::{PCWSTR, PWSTR};

const SERVICE: &str = "teach";
const USERS: [&str; 2] = ["secret:spike_test", "model-api-key"];
const DUMMY: &str = "dummy-spike-value-0123";

fn w(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

/// All credentials whose TargetName matches `*teach*`: (target, persist).
fn enumerate() -> Vec<(String, u32)> {
    let (mut count, mut arr) = (0u32, std::ptr::null_mut());
    let mut out = Vec::new();
    unsafe {
        if CredEnumerateW(PCWSTR::null(), Some(CRED_ENUMERATE_ALL_CREDENTIALS), &mut count, &mut arr).is_ok() {
            for i in 0..count as usize {
                let c = &**arr.add(i);
                let t = c.TargetName.to_string().unwrap_or_default();
                // CredEnumerateW filter only supports a trailing "*", so match "*teach*" here
                if t.to_lowercase().contains("teach") {
                    out.push((t, c.Persist.0));
                }
            }
            CredFree(arr as *const _);
        }
    }
    out
}

/// Persist value of a generic credential, None if missing.
fn read_persist(target: &str) -> Option<u32> {
    let t = w(target);
    let mut p = std::ptr::null_mut();
    unsafe {
        CredReadW(PCWSTR(t.as_ptr()), CRED_TYPE_GENERIC, None, &mut p).ok()?;
        let persist = (*p).Persist.0;
        CredFree(p as *const _);
        Some(persist)
    }
}

fn main() {
    unsafe {
        use windows::Win32::UI::HiDpi::*;
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    println!("before: {:?}", enumerate());

    // keyring path
    let mut persist = 0u32;
    let mut roundtrip_ok = true;
    if let Err(e) = keyring::Entry::store_status() {
        println!("keyring store init FAILED: {e:?}");
        roundtrip_ok = false;
    }
    for user in USERS {
        let r = (|| -> keyring::Result<String> {
            let e = keyring::Entry::new(SERVICE, user)?;
            e.set_password(DUMMY)?;
            e.get_password()
        })();
        match &r {
            Ok(v) if v == DUMMY => println!("keyring write/read {SERVICE}/{user}: OK"),
            other => {
                println!("keyring write/read {SERVICE}/{user}: FAIL {other:?}");
                roundtrip_ok = false;
            }
        }
        // discover the TargetName keyring used: every credential matching *teach*
        let found = enumerate();
        println!("  CredEnumerateW *teach* after write: {found:?}");
        if let Some((t, p)) = found.iter().find(|(t, _)| t.contains(user)) {
            println!("  TargetName used = {t:?}; CredReadW Persist={:?}", read_persist(t));
            persist = *p;
        } else {
            println!("  TargetName not found by *teach* filter (user={user})");
        }
    }
    for user in USERS {
        match keyring::Entry::new(SERVICE, user).and_then(|e| e.delete_credential()) {
            Ok(()) => println!("keyring delete {user}: OK"),
            Err(e) => {
                println!("keyring delete {user}: FAIL {e:?}");
                roundtrip_ok = false;
            }
        }
    }

    let mut method = "keyring";
    if persist != 2 {
        println!("keyring Persist={persist} (not LOCAL_MACHINE); trying CredWriteW with Persist=2");
        method = "CredWriteW";
        let target = w("teach:spike_direct");
        let mut user = w("spike");
        let mut blob = DUMMY.as_bytes().to_vec();
        let cred = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: PWSTR(target.as_ptr() as *mut u16),
            UserName: PWSTR(user.as_mut_ptr()),
            CredentialBlobSize: blob.len() as u32,
            CredentialBlob: blob.as_mut_ptr(),
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            ..Default::default()
        };
        unsafe {
            match CredWriteW(&cred, 0) {
                Ok(()) => {
                    let p = read_persist("teach:spike_direct");
                    println!("CredWriteW OK; CredReadW Persist={p:?}");
                    persist = p.unwrap_or(0);
                }
                Err(e) => println!("CredWriteW FAIL {e:?}"),
            }
            match CredDeleteW(PCWSTR(target.as_ptr()), CRED_TYPE_GENERIC, None) {
                Ok(()) => println!("CredDeleteW OK"),
                Err(e) => println!("CredDeleteW FAIL {e:?}"),
            }
        }
    }

    let left = enumerate();
    println!("after cleanup (should be empty of ours): {left:?}");
    let pass = roundtrip_ok && (persist == 2 || persist == 3);
    println!(
        "0.9: {} persist={persist} method={method}",
        if pass { "PASS" } else { "FAIL" }
    );
}
