//! Account credentials live in macOS Keychain, never in the mailbox or app bundle.
use std::{ffi::c_void, ptr};
const SERVICE: &[u8] = b"com.begenius.sparky.accounts";
/// One sign-in entry for every build of Sparky on this Mac, so `cargo run`
/// and the installed app share connected accounts.
const KEY: &str = "accounts";
const NOT_FOUND: i32 = -25300;
#[link(name = "Security", kind = "framework")]
unsafe extern "C" {
    fn SecKeychainFindGenericPassword(keychain: *const c_void, service_len: u32, service: *const u8, account_len: u32, account: *const u8, len: *mut u32, data: *mut *mut c_void, item: *mut *mut c_void) -> i32;
    fn SecKeychainAddGenericPassword(keychain: *const c_void, service_len: u32, service: *const u8, account_len: u32, account: *const u8, len: u32, data: *const c_void, item: *mut *mut c_void) -> i32;
    fn SecKeychainItemModifyAttributesAndData(item: *mut c_void, attributes: *const c_void, len: u32, data: *const c_void) -> i32;
    fn SecKeychainItemFreeContent(attributes: *mut c_void, data: *mut c_void) -> i32;
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" { fn CFRelease(item: *const c_void); }
/// Older builds keyed the entry by data folder: the checkout for `cargo run`,
/// Application Support for the installed app. Read once, then moved to `KEY`.
fn legacy_keys() -> Vec<String> {
    let mut keys = Vec::new();
    if let Ok(dir) = crate::data_directory() {
        keys.push(dir.to_string_lossy().into_owned());
    }
    let checkout = env!("CARGO_MANIFEST_DIR").to_owned();
    if !keys.contains(&checkout) {
        keys.push(checkout);
    }
    keys
}
fn failure(code: i32) -> String {
    match code {
        // errSecUserCanceled / errSecAuthFailed: the access prompt was dismissed or denied.
        -128 => "macOS Keychain access was cancelled.".into(),
        -25293 => "macOS Keychain access was denied.".into(),
        _ => format!("macOS Keychain returned error {code}; make sure your login keychain is unlocked."),
    }
}
fn find(key: &str) -> Result<Option<Vec<u8>>, String> {
    let mut len = 0;
    let mut data = ptr::null_mut();
    let code = unsafe { SecKeychainFindGenericPassword(ptr::null(), SERVICE.len() as u32, SERVICE.as_ptr(), key.len() as u32, key.as_ptr(), &mut len, &mut data, ptr::null_mut()) };
    if code == NOT_FOUND { return Ok(None); }
    if code != 0 { return Err(failure(code)); }
    let bytes = unsafe { std::slice::from_raw_parts(data as *const u8, len as usize) }.to_vec();
    unsafe { SecKeychainItemFreeContent(ptr::null_mut(), data); }
    Ok(Some(bytes))
}
pub fn load() -> Result<Vec<crate::sync::Account>, String> {
    // Tests must never read or overwrite the developer's real sign-ins.
    if cfg!(test) {
        return Ok(Vec::new());
    }
    if let Some(data) = find(KEY)? {
        return crate::sync::restore_accounts(&data);
    }
    for key in legacy_keys() {
        if let Some(data) = find(&key)? {
            let accounts = crate::sync::restore_accounts(&data)?;
            if !accounts.is_empty() {
                // Move to the shared entry; the old one is left untouched.
                save(&accounts)?;
                return Ok(accounts);
            }
        }
    }
    Ok(Vec::new())
}
pub fn save(accounts: &[crate::sync::Account]) -> Result<(), String> {
    if cfg!(test) {
        return Ok(());
    }
    let data = crate::sync::encode_accounts(accounts)?;
    let mut item = ptr::null_mut();
    let code = unsafe { SecKeychainFindGenericPassword(ptr::null(), SERVICE.len() as u32, SERVICE.as_ptr(), KEY.len() as u32, KEY.as_ptr(), ptr::null_mut(), ptr::null_mut(), &mut item) };
    let code = if code == NOT_FOUND {
        unsafe { SecKeychainAddGenericPassword(ptr::null(), SERVICE.len() as u32, SERVICE.as_ptr(), KEY.len() as u32, KEY.as_ptr(), data.len() as u32, data.as_ptr().cast(), ptr::null_mut()) }
    } else if code == 0 {
        let updated = unsafe { SecKeychainItemModifyAttributesAndData(item, ptr::null(), data.len() as u32, data.as_ptr().cast()) };
        unsafe { CFRelease(item); }
        updated
    } else { code };
    if code == 0 { Ok(()) } else { Err(failure(code)) }
}
