//! Bí mật và API key trong Windows Credential Manager (data-model §Secret reference).
//!
//! Gọi `CredWriteW` trực tiếp với `CRED_PERSIST_LOCAL_MACHINE`: M0 0.9 (T011) đo thấy `keyring` 4 lưu
//! `CRED_PERSIST_ENTERPRISE` (đi theo roaming profile) và không cho chọn mức lưu.
//! Ra ngoài lõi chỉ có `set`/`delete`/`exists`; `get` là `pub(crate)` cho runner/model (Constitution V).

use windows::Win32::Foundation::ERROR_NOT_FOUND;
use windows::Win32::Security::Credentials::{
    CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC, CREDENTIALW, CredDeleteW, CredFree, CredReadW, CredWriteW,
};
use windows::core::{PCWSTR, PWSTR};

const SERVICE: &str = "teach";

/// Tên tài khoản trong Credential Manager.
#[derive(Clone, Copy, Debug)]
pub enum Account<'a> {
    /// `{{secret:<key>}}`
    Secret(&'a str),
    ModelApiKey,
}

fn target(acc: Account) -> Vec<u16> {
    let user = match acc {
        Account::Secret(k) => format!("secret:{k}"),
        Account::ModelApiKey => "model-api-key".to_string(),
    };
    format!("{SERVICE}:{user}").encode_utf16().chain(Some(0)).collect()
}

/// Lỗi không bao giờ chứa giá trị bí mật.
#[derive(Debug)]
pub struct SecretError(pub String);

pub fn set(acc: Account, value: &str) -> Result<(), SecretError> {
    let t = target(acc);
    let mut blob = value.as_bytes().to_vec();
    let cred = CREDENTIALW {
        Type: CRED_TYPE_GENERIC,
        TargetName: PWSTR(t.as_ptr() as *mut u16),
        CredentialBlobSize: blob.len() as u32,
        CredentialBlob: blob.as_mut_ptr(),
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        ..Default::default()
    };
    // SAFETY: mọi con trỏ trỏ vào bộ nhớ sống tới hết lời gọi.
    let r = unsafe { CredWriteW(&cred, 0) };
    blob.fill(0);
    r.map_err(|e| SecretError(format!("CredWriteW: 0x{:08x}", e.code().0)))
}

pub fn delete(acc: Account) -> Result<(), SecretError> {
    let t = target(acc);
    // SAFETY: `t` kết thúc bằng NUL.
    match unsafe { CredDeleteW(PCWSTR(t.as_ptr()), CRED_TYPE_GENERIC, None) } {
        Ok(()) => Ok(()),
        Err(e) if e.code() == ERROR_NOT_FOUND.to_hresult() => Ok(()),
        Err(e) => Err(SecretError(format!("CredDeleteW: 0x{:08x}", e.code().0))),
    }
}

pub fn exists(acc: Account) -> Result<bool, SecretError> {
    Ok(get(acc)?.is_some())
}

/// Chỉ runner (ngay trước khi gõ) và mô-đun model (header Authorization) gọi.
pub(crate) fn get(acc: Account) -> Result<Option<String>, SecretError> {
    let t = target(acc);
    let mut p: *mut CREDENTIALW = std::ptr::null_mut();
    // SAFETY: `p` do hệ thống cấp phát, giải phóng bằng CredFree ngay sau khi sao chép.
    unsafe {
        match CredReadW(PCWSTR(t.as_ptr()), CRED_TYPE_GENERIC, None, &mut p) {
            Ok(()) => {}
            Err(e) if e.code() == ERROR_NOT_FOUND.to_hresult() => return Ok(None),
            Err(e) => return Err(SecretError(format!("CredReadW: 0x{:08x}", e.code().0))),
        }
        let c = &*p;
        let bytes = std::slice::from_raw_parts(c.CredentialBlob, c.CredentialBlobSize as usize);
        let v = String::from_utf8(bytes.to_vec()).map_err(|_| SecretError("not utf-8".into()));
        CredFree(p as *const _);
        v.map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_local_machine() {
        let acc = Account::Secret("teach_unit_test_tmp");
        set(acc, "S3cret!xyz").unwrap();
        assert!(exists(acc).unwrap());
        assert_eq!(get(acc).unwrap().as_deref(), Some("S3cret!xyz"));
        // mức lưu thực tế
        let t = target(acc);
        let mut p: *mut CREDENTIALW = std::ptr::null_mut();
        unsafe {
            CredReadW(PCWSTR(t.as_ptr()), CRED_TYPE_GENERIC, None, &mut p).unwrap();
            assert_eq!((*p).Persist, CRED_PERSIST_LOCAL_MACHINE);
            CredFree(p as *const _);
        }
        delete(acc).unwrap();
        assert!(!exists(acc).unwrap());
        delete(acc).unwrap(); // xoá lần hai không lỗi
    }
}
