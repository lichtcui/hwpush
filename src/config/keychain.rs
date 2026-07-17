use security_framework::os::macos::keychain::SecKeychain;

const SERVICE_NAME: &str = "hiboard";
const ACCOUNT_NAME: &str = "auth_code";

pub fn set_auth_code(code: &str) -> Result<(), String> {
    let keychain = SecKeychain::default().map_err(|e| format!("打开 Keychain 失败: {e}"))?;

    // Delete existing item if present
    let _ = delete_auth_code();

    // Add new password
    keychain
        .set_generic_password(SERVICE_NAME, ACCOUNT_NAME, code.as_bytes())
        .map_err(|e| format!("保存到 Keychain 失败: {e}"))?;

    Ok(())
}

pub fn get_auth_code() -> Result<String, String> {
    let keychain = SecKeychain::default().map_err(|e| format!("打开 Keychain 失败: {e}"))?;

    let (data, _item) = keychain
        .find_generic_password(SERVICE_NAME, ACCOUNT_NAME)
        .map_err(|_| "Keychain 中未找到认证码".to_string())?;

    String::from_utf8(data.to_vec())
        .map_err(|e| format!("Keychain 中数据不是有效 UTF-8: {e}"))
}

fn delete_auth_code() -> Result<(), String> {
    let keychain = SecKeychain::default().map_err(|e| format!("打开 Keychain 失败: {e}"))?;

    if let Ok((_, _item)) = keychain.find_generic_password(SERVICE_NAME, ACCOUNT_NAME) {
        // delete() returns (), no error to propagate
    }

    Ok(())
}
