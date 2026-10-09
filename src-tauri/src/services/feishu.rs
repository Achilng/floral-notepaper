use super::notes::{AppError, Note, NoteStore};
use crate::json_io::write_json_atomic;
use chrono::Utc;
use keyring::{Entry, Error as KeyringError};
use reqwest::blocking::{Client, RequestBuilder};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};

const API_BASE: &str = "https://open.feishu.cn/open-apis";
const ACCOUNTS_BASE: &str = "https://accounts.feishu.cn";
const KEYRING_SERVICE: &str = "floral-notepaper";
const KEYRING_ACCOUNT: &str = "feishu-app-secret";
const KEYRING_USER_TOKENS_ACCOUNT: &str = "feishu-user-tokens";
const KEYRING_USER_ACCESS_ACCOUNT: &str = "feishu-user-access-token";
const KEYRING_USER_REFRESH_ACCOUNT: &str = "feishu-user-refresh-token";
const KEYRING_USER_EXPIRES_ACCOUNT: &str = "feishu-user-token-expires";
const SETTINGS_FILE: &str = "feishu.json";
const SYNC_FILE: &str = "feishu-sync.json";
const AUTH_DIAGNOSTIC_FILE: &str = "feishu-auth-diagnostic.json";
const USER_TOKENS_DPAPI_FILE: &str = "feishu-user-tokens.dpapi";
const SYNC_DEBOUNCE: Duration = Duration::from_secs(5);
const RETRY_DELAYS: [Duration; 2] = [Duration::from_secs(5), Duration::from_secs(30)];
const TOKEN_CACHE_LIFETIME: Duration = Duration::from_secs(90 * 60);
const MAX_BLOCKS_PER_REQUEST: usize = 50;
const MAX_TEXT_CHARS: usize = 1800;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct StoredSettings {
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    app_id: String,
    #[serde(default)]
    folder_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FeishuSettings {
    pub enabled: bool,
    pub app_id: String,
    pub folder_token: String,
    pub has_secret: bool,
    pub connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserTokens {
    access_token: String,
    refresh_token: String,
    expires_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceAuthorization {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceAuthorizationPoll {
    pub pending: bool,
    pub connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct SyncRecord {
    document_id: String,
    content_hash: String,
    status: String,
    last_synced_at: Option<String>,
    last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct SyncFile {
    #[serde(default)]
    notes: BTreeMap<String, SyncRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SyncOverview {
    pub enabled: bool,
    pub configured: bool,
    pub total: usize,
    pub synced: usize,
    pub pending: usize,
    pub failed: usize,
    pub syncing: bool,
    pub last_synced_at: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SyncSummary {
    pub total: usize,
    pub synced: usize,
    pub skipped: usize,
    pub failed: usize,
}

struct CachedToken {
    app_id: String,
    value: String,
    created_at: Instant,
}

#[derive(Default)]
pub struct FeishuSyncRuntime {
    generations: Mutex<HashMap<String, u64>>,
    sync_lock: Mutex<()>,
    token: Mutex<Option<CachedToken>>,
    active_jobs: Mutex<usize>,
}

impl FeishuSyncRuntime {
    fn next_generation(&self, note_id: &str) -> u64 {
        let mut generations = self.generations.lock().unwrap_or_else(|e| e.into_inner());
        let generation = generations.entry(note_id.to_string()).or_insert(0);
        *generation += 1;
        *generation
    }

    fn is_current(&self, note_id: &str, generation: u64) -> bool {
        self.generations
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(note_id)
            .is_some_and(|current| *current == generation)
    }

    pub(crate) fn set_active(&self, delta: isize) {
        let mut active = self.active_jobs.lock().unwrap_or_else(|e| e.into_inner());
        if delta >= 0 {
            *active += delta as usize;
        } else {
            *active = active.saturating_sub((-delta) as usize);
        }
    }

    fn is_active(&self) -> bool {
        *self.active_jobs.lock().unwrap_or_else(|e| e.into_inner()) > 0
    }

    fn tenant_token(&self, settings: &StoredSettings, secret: &str) -> Result<String, AppError> {
        {
            let token = self.token.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(cached) = token.as_ref() {
                if cached.app_id == settings.app_id
                    && cached.created_at.elapsed() < TOKEN_CACHE_LIFETIME
                {
                    return Ok(cached.value.clone());
                }
            }
        }

        let token = request_tenant_token(&settings.app_id, secret)?;
        *self.token.lock().unwrap_or_else(|e| e.into_inner()) = Some(CachedToken {
            app_id: settings.app_id.clone(),
            value: token.clone(),
            created_at: Instant::now(),
        });
        Ok(token)
    }

    pub fn clear_token(&self) {
        *self.token.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
}

fn app_error(code: &str, message: impl Into<String>) -> AppError {
    AppError {
        code: code.into(),
        message: message.into(),
        details: BTreeMap::new(),
    }
}

fn keyring_entry() -> Result<Entry, AppError> {
    Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT)
        .map_err(|error| app_error("feishuSecureStore", error.to_string()))
}

fn user_token_entry(account: &str) -> Result<Entry, AppError> {
    Entry::new(KEYRING_SERVICE, account)
        .map_err(|error| app_error("feishuSecureStore", error.to_string()))
}

fn get_secret() -> Result<Option<String>, AppError> {
    match keyring_entry()?.get_password() {
        Ok(secret) => Ok((!secret.trim().is_empty()).then(|| secret.trim().to_string())),
        Err(KeyringError::NoEntry) => Ok(None),
        Err(error) => Err(app_error("feishuSecureStore", error.to_string())),
    }
}

fn get_user_tokens() -> Result<Option<UserTokens>, AppError> {
    #[cfg(target_os = "windows")]
    {
        get_user_tokens_dpapi()
    }

    #[cfg(not(target_os = "windows"))]
    {
        let access_token = match user_token_entry(KEYRING_USER_ACCESS_ACCOUNT)?.get_password() {
            Ok(value) => value,
            Err(KeyringError::NoEntry) => return Ok(None),
            Err(error) => return Err(app_error("feishuSecureStore", error.to_string())),
        };
        let refresh_token = match user_token_entry(KEYRING_USER_REFRESH_ACCOUNT)?.get_password() {
            Ok(value) => value,
            Err(KeyringError::NoEntry) => String::new(),
            Err(error) => return Err(app_error("feishuSecureStore", error.to_string())),
        };
        let expires_at = match user_token_entry(KEYRING_USER_EXPIRES_ACCOUNT)?.get_password() {
            Ok(value) => value
                .parse::<i64>()
                .map_err(|error| app_error("feishuSecureStore", error.to_string()))?,
            Err(KeyringError::NoEntry) => 0,
            Err(error) => return Err(app_error("feishuSecureStore", error.to_string())),
        };
        Ok(Some(UserTokens {
            access_token,
            refresh_token,
            expires_at,
        }))
    }
}

fn save_user_tokens(tokens: &UserTokens) -> Result<(), AppError> {
    #[cfg(target_os = "windows")]
    {
        save_user_tokens_dpapi(tokens)
    }

    #[cfg(not(target_os = "windows"))]
    {
        user_token_entry(KEYRING_USER_ACCESS_ACCOUNT)?
            .set_password(&tokens.access_token)
            .map_err(|error| app_error("feishuSecureStore", error.to_string()))?;
        if tokens.refresh_token.is_empty() {
            let _ = user_token_entry(KEYRING_USER_REFRESH_ACCOUNT)?.delete_credential();
        } else {
            user_token_entry(KEYRING_USER_REFRESH_ACCOUNT)?
                .set_password(&tokens.refresh_token)
                .map_err(|error| app_error("feishuSecureStore", error.to_string()))?;
        }
        user_token_entry(KEYRING_USER_EXPIRES_ACCOUNT)?
            .set_password(&tokens.expires_at.to_string())
            .map_err(|error| app_error("feishuSecureStore", error.to_string()))
    }
}

fn delete_user_tokens() -> Result<(), AppError> {
    #[cfg(target_os = "windows")]
    {
        let path = default_config_dir_for_tokens()?.join(USER_TOKENS_DPAPI_FILE);
        if path.exists() {
            fs::remove_file(path)?;
        }
    }
    for account in [
        KEYRING_USER_TOKENS_ACCOUNT,
        KEYRING_USER_ACCESS_ACCOUNT,
        KEYRING_USER_REFRESH_ACCOUNT,
        KEYRING_USER_EXPIRES_ACCOUNT,
    ] {
        match user_token_entry(account)?.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => {}
            Err(error) => return Err(app_error("feishuSecureStore", error.to_string())),
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn default_config_dir_for_tokens() -> Result<std::path::PathBuf, AppError> {
    super::notes::default_config_dir()
}

#[cfg(target_os = "windows")]
fn protect_with_dpapi(plain: &[u8]) -> Result<Vec<u8>, AppError> {
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::Cryptography::{CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB},
    };

    let input = CRYPT_INTEGER_BLOB {
        cbData: plain.len() as u32,
        pbData: plain.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    // SAFETY: input points to `plain` for the duration of the call; Windows
    // allocates output with LocalAlloc and ownership is released below.
    let ok = unsafe {
        CryptProtectData(
            &input,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if ok == 0 {
        return Err(app_error(
            "feishuSecureStore",
            std::io::Error::last_os_error().to_string(),
        ));
    }
    // SAFETY: a successful CryptProtectData call returns a valid buffer of
    // exactly cbData bytes, which remains valid until LocalFree.
    let encrypted =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    // SAFETY: output.pbData was allocated by CryptProtectData.
    unsafe { LocalFree(output.pbData.cast()) };
    Ok(encrypted)
}

#[cfg(target_os = "windows")]
fn unprotect_with_dpapi(encrypted: &[u8]) -> Result<Vec<u8>, AppError> {
    use windows_sys::{
        core::PWSTR,
        Win32::{
            Foundation::LocalFree,
            Security::Cryptography::{
                CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
            },
        },
    };

    let input = CRYPT_INTEGER_BLOB {
        cbData: encrypted.len() as u32,
        pbData: encrypted.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    let mut description: PWSTR = std::ptr::null_mut();
    // SAFETY: input points to `encrypted` for the duration of the call;
    // output and description are Windows-owned allocations released below.
    let ok = unsafe {
        CryptUnprotectData(
            &input,
            &mut description,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if ok == 0 {
        return Err(app_error(
            "feishuSecureStore",
            std::io::Error::last_os_error().to_string(),
        ));
    }
    // SAFETY: a successful CryptUnprotectData call returns a valid buffer of
    // exactly cbData bytes, which remains valid until LocalFree.
    let plain =
        unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
    // SAFETY: both pointers, when non-null, were allocated by CryptUnprotectData.
    unsafe {
        LocalFree(output.pbData.cast());
        if !description.is_null() {
            LocalFree(description.cast());
        }
    }
    Ok(plain)
}

#[cfg(target_os = "windows")]
fn get_user_tokens_dpapi() -> Result<Option<UserTokens>, AppError> {
    let path = default_config_dir_for_tokens()?.join(USER_TOKENS_DPAPI_FILE);
    if !path.exists() {
        return Ok(None);
    }
    let encrypted = fs::read(path)?;
    let plain = unprotect_with_dpapi(&encrypted)?;
    serde_json::from_slice(&plain)
        .map(Some)
        .map_err(|error| app_error("feishuSecureStore", error.to_string()))
}

#[cfg(target_os = "windows")]
fn save_user_tokens_dpapi(tokens: &UserTokens) -> Result<(), AppError> {
    let plain = serde_json::to_vec(tokens)
        .map_err(|error| app_error("feishuSecureStore", error.to_string()))?;
    let encrypted = protect_with_dpapi(&plain)?;
    let config_dir = default_config_dir_for_tokens()?;
    fs::create_dir_all(&config_dir)?;
    fs::write(config_dir.join(USER_TOKENS_DPAPI_FILE), encrypted)?;
    for account in [
        KEYRING_USER_TOKENS_ACCOUNT,
        KEYRING_USER_ACCESS_ACCOUNT,
        KEYRING_USER_REFRESH_ACCOUNT,
        KEYRING_USER_EXPIRES_ACCOUNT,
    ] {
        if let Ok(entry) = user_token_entry(account) {
            let _ = entry.delete_credential();
        }
    }
    Ok(())
}

fn settings_path(store: &NoteStore) -> std::path::PathBuf {
    store.config_dir().join(SETTINGS_FILE)
}

fn sync_path(store: &NoteStore) -> std::path::PathBuf {
    store.data_dir().join(SYNC_FILE)
}

fn write_auth_diagnostic(store: &NoteStore, value: Value) {
    if fs::create_dir_all(store.data_dir()).is_ok() {
        let _ = write_json_atomic(&store.data_dir().join(AUTH_DIAGNOSTIC_FILE), &value);
    }
}

fn load_stored_settings(store: &NoteStore) -> Result<StoredSettings, AppError> {
    let path = settings_path(store);
    if !path.exists() {
        return Ok(StoredSettings::default());
    }
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

pub fn get_settings(store: &NoteStore) -> Result<FeishuSettings, AppError> {
    let settings = load_stored_settings(store)?;
    Ok(FeishuSettings {
        enabled: settings.enabled,
        app_id: settings.app_id,
        folder_token: settings.folder_token,
        has_secret: get_secret()?.is_some(),
        connected: get_user_tokens()?.is_some(),
    })
}

pub fn save_settings(
    store: &NoteStore,
    runtime: &FeishuSyncRuntime,
    settings: FeishuSettings,
    app_secret: Option<String>,
) -> Result<FeishuSettings, AppError> {
    let stored = StoredSettings {
        enabled: settings.enabled,
        app_id: settings.app_id.trim().to_string(),
        folder_token: settings.folder_token.trim().to_string(),
    };
    if stored.enabled && stored.app_id.is_empty() {
        return Err(app_error("feishuAppIdEmpty", "飞书 App ID 不能为空"));
    }

    if let Some(secret) = app_secret {
        let secret = secret.trim();
        if !secret.is_empty() {
            keyring_entry()?
                .set_password(secret)
                .map_err(|error| app_error("feishuSecureStore", error.to_string()))?;
        }
    }

    fs::create_dir_all(store.config_dir())?;
    write_json_atomic(&settings_path(store), &stored)?;
    runtime.clear_token();
    get_settings(store)
}

pub fn clear_secret(
    store: &NoteStore,
    runtime: &FeishuSyncRuntime,
) -> Result<FeishuSettings, AppError> {
    match keyring_entry()?.delete_credential() {
        Ok(()) | Err(KeyringError::NoEntry) => {}
        Err(error) => return Err(app_error("feishuSecureStore", error.to_string())),
    }
    runtime.clear_token();
    delete_user_tokens()?;
    get_settings(store)
}

fn configured(settings: &StoredSettings, secret: &Option<String>) -> bool {
    !settings.app_id.is_empty() && secret.is_some()
}

pub fn test_connection(store: &NoteStore, runtime: &FeishuSyncRuntime) -> Result<(), AppError> {
    let settings = load_stored_settings(store)?;
    let secret =
        get_secret()?.ok_or_else(|| app_error("feishuSecretMissing", "请先保存飞书 App Secret"))?;
    if settings.app_id.is_empty() {
        return Err(app_error("feishuAppIdEmpty", "飞书 App ID 不能为空"));
    }
    // A newly published app version can change the granted scopes while the
    // previously issued tenant token is still cached locally.
    runtime.clear_token();
    runtime.tenant_token(&settings, &secret)?;
    Ok(())
}

fn oauth_field<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    value
        .get(name)
        .or_else(|| value.get("data").and_then(|data| data.get(name)))
}

fn tokens_from_oauth(value: &Value) -> Result<UserTokens, AppError> {
    let access_token = oauth_field(value, "access_token")
        .and_then(Value::as_str)
        .ok_or_else(|| app_error("feishuOAuth", "飞书未返回 user_access_token"))?;
    // Device Flow may issue a usable access token without a refresh token for
    // some app/tenant policies. Accept it for the current session and ask the
    // user to authorize again only after it expires.
    let refresh_token = oauth_field(value, "refresh_token")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let expires_in = oauth_field(value, "expires_in")
        .and_then(Value::as_i64)
        .unwrap_or(7200);
    Ok(UserTokens {
        access_token: access_token.to_string(),
        refresh_token: refresh_token.to_string(),
        expires_at: Utc::now().timestamp() + expires_in.saturating_sub(60),
    })
}

pub fn start_device_authorization(store: &NoteStore) -> Result<DeviceAuthorization, AppError> {
    let settings = load_stored_settings(store)?;
    if settings.app_id.is_empty() {
        return Err(app_error("feishuAppIdEmpty", "飞书 App ID 不能为空"));
    }
    let secret =
        get_secret()?.ok_or_else(|| app_error("feishuSecretMissing", "请先保存飞书 App Secret"))?;
    let value = send_json(
        http_client()?
            .post(format!("{ACCOUNTS_BASE}/oauth/v1/device_authorization"))
            .basic_auth(&settings.app_id, Some(&secret))
            .form(&[
                ("client_id", settings.app_id.as_str()),
                ("scope", "offline_access docx:document drive:drive"),
            ]),
    )?;
    let string_field = |name: &str| {
        oauth_field(&value, name)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| app_error("feishuOAuth", format!("飞书未返回 {name}")))
    };
    Ok(DeviceAuthorization {
        device_code: string_field("device_code")?,
        user_code: string_field("user_code")?,
        verification_uri: string_field("verification_uri")?,
        verification_uri_complete: string_field("verification_uri_complete")?,
        expires_in: oauth_field(&value, "expires_in")
            .and_then(Value::as_u64)
            .unwrap_or(600),
        interval: oauth_field(&value, "interval")
            .and_then(Value::as_u64)
            .unwrap_or(5),
    })
}

pub fn poll_device_authorization(
    store: &NoteStore,
    device_code: &str,
) -> Result<DeviceAuthorizationPoll, AppError> {
    let settings = load_stored_settings(store)?;
    let secret =
        get_secret()?.ok_or_else(|| app_error("feishuSecretMissing", "请先保存飞书 App Secret"))?;
    let response = http_client()?
        .post(format!("{ACCOUNTS_BASE}/oauth/v3/token"))
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("client_id", settings.app_id.as_str()),
            ("client_secret", secret.as_str()),
            ("device_code", device_code),
        ])
        .send()
        .map_err(|error| app_error("feishuNetwork", error.to_string()))?;
    let status = response.status();
    let body = response.text().map_err(|error| {
        write_auth_diagnostic(
            store,
            json!({ "stage": "read", "httpStatus": status.as_u16(), "error": error.to_string() }),
        );
        app_error("feishuResponse", format!("读取飞书令牌响应失败：{error}"))
    })?;
    let value: Value = serde_json::from_str(&body).map_err(|error| {
        write_auth_diagnostic(
            store,
            json!({
                "stage": "parse",
                "httpStatus": status.as_u16(),
                "error": error.to_string(),
                "bodyPreview": body.chars().take(160).collect::<String>()
            }),
        );
        app_error(
            "feishuResponse",
            format!("飞书令牌响应不是有效 JSON（HTTP {status}）：{error}"),
        )
    })?;
    write_auth_diagnostic(
        store,
        json!({
            "stage": "token",
            "httpStatus": status.as_u16(),
            "code": value.get("code"),
            "error": value.get("error"),
            "errorDescription": value.get("error_description"),
            "hasAccessToken": oauth_field(&value, "access_token").is_some(),
            "hasRefreshToken": oauth_field(&value, "refresh_token").is_some(),
            "tokenType": oauth_field(&value, "token_type"),
            "scope": oauth_field(&value, "scope")
        }),
    );
    if oauth_field(&value, "access_token").is_some() {
        let tokens = tokens_from_oauth(&value).inspect_err(|error| {
            write_auth_diagnostic(
                store,
                json!({ "stage": "token-parse", "error": error.message.clone() }),
            );
        })?;
        save_user_tokens(&tokens).inspect_err(|error| {
            write_auth_diagnostic(
                store,
                json!({ "stage": "keyring-save", "error": error.message.clone() }),
            );
        })?;
        write_auth_diagnostic(store, json!({ "stage": "connected" }));
        return Ok(DeviceAuthorizationPoll {
            pending: false,
            connected: true,
        });
    }
    let code = value
        .get("code")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    if matches!(code, 99_991_671 | 99_991_673) {
        return Ok(DeviceAuthorizationPoll {
            pending: true,
            connected: false,
        });
    }
    let error = value
        .get("error")
        .and_then(Value::as_str)
        .or_else(|| value.get("msg").and_then(Value::as_str))
        .unwrap_or("飞书授权失败");
    if matches!(error, "authorization_pending" | "slow_down") {
        return Ok(DeviceAuthorizationPoll {
            pending: true,
            connected: false,
        });
    }
    Err(app_error(
        "feishuOAuth",
        format!("{error}（HTTP {status}，code {code}）"),
    ))
}

fn user_access_token(store: &NoteStore) -> Result<String, AppError> {
    let mut tokens = get_user_tokens()?
        .ok_or_else(|| app_error("feishuLoginRequired", "请先登录飞书并完成用户授权"))?;
    if tokens.expires_at > Utc::now().timestamp() {
        return Ok(tokens.access_token);
    }
    if tokens.refresh_token.is_empty() {
        delete_user_tokens()?;
        return Err(app_error(
            "feishuLoginRequired",
            "飞书授权已过期且未提供刷新令牌，请重新登录飞书",
        ));
    }
    let settings = load_stored_settings(store)?;
    let secret =
        get_secret()?.ok_or_else(|| app_error("feishuSecretMissing", "请先保存飞书 App Secret"))?;
    let value = send_json(
        http_client()?
            .post(format!("{ACCOUNTS_BASE}/oauth/v3/token"))
            .form(&[
                ("grant_type", "refresh_token"),
                ("client_id", settings.app_id.as_str()),
                ("client_secret", secret.as_str()),
                ("refresh_token", tokens.refresh_token.as_str()),
            ]),
    )?;
    tokens = tokens_from_oauth(&value)?;
    save_user_tokens(&tokens)?;
    Ok(tokens.access_token)
}

fn load_sync_file(store: &NoteStore) -> Result<SyncFile, AppError> {
    let path = sync_path(store);
    if !path.exists() {
        return Ok(SyncFile::default());
    }
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

fn save_sync_file(store: &NoteStore, sync: &SyncFile) -> Result<(), AppError> {
    fs::create_dir_all(store.data_dir())?;
    write_json_atomic(&sync_path(store), sync)?;
    Ok(())
}

fn mark_failed(store: &NoteStore, note_id: &str, error: &AppError) -> Result<(), AppError> {
    let mut sync = load_sync_file(store)?;
    let record = sync.notes.entry(note_id.to_string()).or_default();
    record.status = "failed".into();
    record.last_error = Some(error.message.clone());
    save_sync_file(store, &sync)
}

pub fn overview(store: &NoteStore, runtime: &FeishuSyncRuntime) -> Result<SyncOverview, AppError> {
    let settings = load_stored_settings(store)?;
    let secret = get_secret()?;
    let sync = load_sync_file(store)?;
    let notes = store.list_notes()?;
    let mut result = SyncOverview {
        enabled: settings.enabled,
        configured: configured(&settings, &secret) && get_user_tokens()?.is_some(),
        total: notes.len(),
        syncing: runtime.is_active(),
        ..SyncOverview::default()
    };

    for note in notes {
        match sync
            .notes
            .get(&note.id)
            .map(|record| record.status.as_str())
        {
            Some("synced") => result.synced += 1,
            Some("failed") => result.failed += 1,
            _ => result.pending += 1,
        }
    }
    result.last_synced_at = sync
        .notes
        .values()
        .filter_map(|record| record.last_synced_at.clone())
        .max();
    result.last_error = sync
        .notes
        .values()
        .filter_map(|record| record.last_error.clone())
        .next_back();
    Ok(result)
}

fn request_tenant_token(app_id: &str, secret: &str) -> Result<String, AppError> {
    let client = http_client()?;
    let value = send_json(
        client
            .post(format!("{API_BASE}/auth/v3/tenant_access_token/internal"))
            .json(&json!({ "app_id": app_id, "app_secret": secret })),
    )?;
    value
        .get("tenant_access_token")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| app_error("feishuAuth", "飞书未返回 tenant_access_token"))
}

fn http_client() -> Result<Client, AppError> {
    Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(30))
        .user_agent(format!("floral-notepaper/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| app_error("feishuNetwork", error.to_string()))
}

fn send_json(request: RequestBuilder) -> Result<Value, AppError> {
    let response = request
        .send()
        .map_err(|error| app_error("feishuNetwork", error.to_string()))?;
    let status = response.status();
    let body = response
        .text()
        .map_err(|error| app_error("feishuResponse", format!("读取飞书响应失败：{error}")))?;
    let value: Value = serde_json::from_str(&body).map_err(|error| {
        let preview = body.chars().take(160).collect::<String>();
        app_error(
            "feishuResponse",
            format!("飞书返回了非 JSON 响应（HTTP {status}）：{error}；{preview}"),
        )
    })?;
    let code = value
        .get("code")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    if !status.is_success() || code != 0 {
        let raw_message = value
            .get("msg")
            .and_then(Value::as_str)
            .unwrap_or("飞书 API 请求失败");
        let message = if code == 1_770_040 {
            "当前应用没有目标文件夹权限。请在飞书开放平台为应用开通“查看、评论、编辑和管理云空间中所有文件”（drive:drive），发布新版本后重试"
        } else {
            raw_message
        };
        return Err(app_error(
            "feishuApi",
            format!("{message}（HTTP {status}，code {code}）"),
        ));
    }
    Ok(value)
}

struct FeishuClient {
    http: Client,
    token: String,
    folder_token: String,
}

impl FeishuClient {
    fn new(token: String, folder_token: String) -> Result<Self, AppError> {
        Ok(Self {
            http: http_client()?,
            token,
            folder_token,
        })
    }

    fn authorized(&self, request: RequestBuilder) -> RequestBuilder {
        request.bearer_auth(&self.token)
    }

    fn create_document(&self, title: &str) -> Result<String, AppError> {
        let mut body = json!({ "title": normalized_title(title) });
        if !self.folder_token.is_empty() {
            body["folder_token"] = Value::String(self.folder_token.clone());
        }
        let value = send_json(
            self.authorized(
                self.http
                    .post(format!("{API_BASE}/docx/v1/documents"))
                    .json(&body),
            ),
        )?;
        value
            .pointer("/data/document/document_id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| app_error("feishuResponse", "飞书未返回 document_id"))
    }

    fn child_count(&self, document_id: &str) -> Result<usize, AppError> {
        let value = send_json(self.authorized(self.http.get(format!(
            "{API_BASE}/docx/v1/documents/{document_id}/blocks/{document_id}/children?page_size=500&document_revision_id=-1"
        ))))?;
        Ok(value
            .pointer("/data/items")
            .and_then(Value::as_array)
            .map_or(0, Vec::len))
    }

    fn delete_children(&self, document_id: &str, count: usize) -> Result<(), AppError> {
        if count == 0 {
            return Ok(());
        }
        send_json(self.authorized(
            self.http
                .delete(format!(
                    "{API_BASE}/docx/v1/documents/{document_id}/blocks/{document_id}/children/batch_delete?document_revision_id=-1"
                ))
                .json(&json!({ "start_index": 0, "end_index": count })),
        ))?;
        Ok(())
    }

    fn delete_all_children(&self, document_id: &str) -> Result<(), AppError> {
        loop {
            let count = self.child_count(document_id)?;
            if count == 0 {
                return Ok(());
            }
            self.delete_children(document_id, count)?;
        }
    }

    fn create_blocks(&self, document_id: &str, blocks: &[Value]) -> Result<(), AppError> {
        for (chunk_index, chunk) in blocks.chunks(MAX_BLOCKS_PER_REQUEST).enumerate() {
            send_json(self.authorized(
                self.http
                    .post(format!(
                        "{API_BASE}/docx/v1/documents/{document_id}/blocks/{document_id}/children?document_revision_id=-1"
                    ))
                    .json(&json!({
                        "children": chunk,
                        "index": chunk_index * MAX_BLOCKS_PER_REQUEST
                    })),
            ))?;
        }
        Ok(())
    }

    fn replace_document(&self, document_id: &str, note: &Note) -> Result<(), AppError> {
        self.delete_all_children(document_id)?;
        self.create_blocks(document_id, &markdown_blocks(&note.content))
    }
}

fn normalized_title(title: &str) -> &str {
    let title = title.trim();
    if title.is_empty() {
        "未命名笔记"
    } else {
        title
    }
}

fn note_hash(note: &Note) -> String {
    let mut hasher = Sha256::new();
    hasher.update(note.title.as_bytes());
    hasher.update([0]);
    hasher.update(note.category.as_bytes());
    hasher.update([0]);
    hasher.update(note.content.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn text_elements(content: &str) -> Value {
    if content.is_empty() {
        // Feishu rejects text-like blocks whose `elements` array is empty
        // with docx error 1770001. A single space renders as an empty line
        // while keeping the block payload valid.
        return json!([{ "text_run": { "content": " ", "text_element_style": {} } }]);
    }
    json!([{ "text_run": { "content": content, "text_element_style": {} } }])
}

fn text_block(block_type: u64, key: &str, content: &str, extra: Option<Value>) -> Value {
    let mut body = json!({ "elements": text_elements(content), "style": {} });
    if let Some(extra) = extra {
        if let (Some(body), Some(extra)) = (body.as_object_mut(), extra.as_object()) {
            body.extend(extra.clone());
        }
    }
    json!({ "block_type": block_type, key: body })
}

fn split_text(content: &str) -> Vec<String> {
    if content.chars().count() <= MAX_TEXT_CHARS {
        return vec![content.to_string()];
    }
    let mut chunks = Vec::new();
    let mut current = String::new();
    for ch in content.chars() {
        current.push(ch);
        if current.chars().count() >= MAX_TEXT_CHARS {
            chunks.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    chunks
}

fn ordered_content(line: &str) -> Option<&str> {
    let digit_count = line.bytes().take_while(u8::is_ascii_digit).count();
    (digit_count > 0 && line.as_bytes().get(digit_count..digit_count + 2) == Some(b". "))
        .then(|| &line[digit_count + 2..])
}

fn markdown_blocks(content: &str) -> Vec<Value> {
    let mut blocks = Vec::new();
    let mut code = None::<String>;

    for line in content.lines() {
        if line.trim_start().starts_with("```") {
            if let Some(code_content) = code.take() {
                for chunk in split_text(code_content.trim_end_matches('\n')) {
                    blocks.push(text_block(14, "code", &chunk, None));
                }
            } else {
                code = Some(String::new());
            }
            continue;
        }
        if let Some(code_content) = code.as_mut() {
            code_content.push_str(line);
            code_content.push('\n');
            continue;
        }

        let trimmed = line.trim_start();
        let heading_marks = trimmed.bytes().take_while(|byte| *byte == b'#').count();
        if (1..=9).contains(&heading_marks) && trimmed.as_bytes().get(heading_marks) == Some(&b' ')
        {
            blocks.push(text_block(
                2 + heading_marks as u64,
                &format!("heading{heading_marks}"),
                &trimmed[heading_marks + 1..],
                None,
            ));
            continue;
        }
        if let Some(todo) = trimmed.strip_prefix("- [ ] ") {
            blocks.push(text_block(17, "todo", todo, Some(json!({ "done": false }))));
            continue;
        }
        if let Some(todo) = trimmed
            .strip_prefix("- [x] ")
            .or_else(|| trimmed.strip_prefix("- [X] "))
        {
            blocks.push(text_block(17, "todo", todo, Some(json!({ "done": true }))));
            continue;
        }
        if let Some(item) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            blocks.push(text_block(12, "bullet", item, None));
            continue;
        }
        if let Some(item) = ordered_content(trimmed) {
            blocks.push(text_block(13, "ordered", item, None));
            continue;
        }
        if let Some(quote) = trimmed.strip_prefix("> ") {
            blocks.push(text_block(15, "quote", quote, None));
            continue;
        }
        if matches!(trimmed, "---" | "***" | "___") {
            blocks.push(json!({ "block_type": 22, "divider": {} }));
            continue;
        }
        for chunk in split_text(line) {
            blocks.push(text_block(2, "text", &chunk, None));
        }
    }

    if let Some(code_content) = code {
        for chunk in split_text(code_content.trim_end_matches('\n')) {
            blocks.push(text_block(14, "code", &chunk, None));
        }
    }
    if blocks.is_empty() {
        blocks.push(text_block(2, "text", "", None));
    }
    blocks
}

fn sync_one(
    store: &NoteStore,
    runtime: &FeishuSyncRuntime,
    client: &FeishuClient,
    note_id: &str,
) -> Result<bool, AppError> {
    let note = store.read_note(note_id)?;
    let hash = note_hash(&note);
    let mut sync = load_sync_file(store)?;
    let existing = sync.notes.get(note_id).cloned().unwrap_or_default();
    if !existing.document_id.is_empty() && existing.content_hash == hash {
        let record = sync.notes.entry(note_id.to_string()).or_default();
        record.status = "synced".into();
        record.last_error = None;
        save_sync_file(store, &sync)?;
        return Ok(false);
    }

    let document_id = if existing.document_id.is_empty() {
        client.create_document(&note.title)?
    } else {
        existing.document_id
    };

    let sync_result = client.replace_document(&document_id, &note);
    let record = sync.notes.entry(note_id.to_string()).or_default();
    record.document_id = document_id;
    match sync_result {
        Ok(()) => {
            record.content_hash = hash;
            record.status = "synced".into();
            record.last_synced_at = Some(Utc::now().to_rfc3339());
            record.last_error = None;
            save_sync_file(store, &sync)?;
            Ok(true)
        }
        Err(error) => {
            record.status = "failed".into();
            record.last_error = Some(error.message.clone());
            save_sync_file(store, &sync)?;
            runtime.clear_token();
            Err(error)
        }
    }
}

fn client_for(
    store: &NoteStore,
    _runtime: &FeishuSyncRuntime,
) -> Result<Option<FeishuClient>, AppError> {
    let settings = load_stored_settings(store)?;
    let secret = get_secret()?;
    if !settings.enabled || !configured(&settings, &secret) {
        return Ok(None);
    }
    let token = user_access_token(store)?;
    Ok(Some(FeishuClient::new(token, settings.folder_token)?))
}

pub fn sync_all(store: &NoteStore, runtime: &FeishuSyncRuntime) -> Result<SyncSummary, AppError> {
    let _guard = runtime.sync_lock.lock().unwrap_or_else(|e| e.into_inner());
    let mut client = client_for(store, runtime)?
        .ok_or_else(|| app_error("feishuNotConfigured", "请先启用并配置飞书同步"))?;
    let notes = store.list_notes()?;
    let mut summary = SyncSummary {
        total: notes.len(),
        ..SyncSummary::default()
    };
    for note in notes {
        let mut result = sync_one(store, runtime, &client, &note.id);
        if result.is_err() {
            // Permissions may have been published after this process obtained
            // its cached tenant token. Refresh it and retry the note once.
            runtime.clear_token();
            result = match client_for(store, runtime) {
                Ok(Some(refreshed_client)) => {
                    client = refreshed_client;
                    sync_one(store, runtime, &client, &note.id)
                }
                Ok(None) => Err(app_error("feishuNotConfigured", "请先启用并配置飞书同步")),
                Err(error) => Err(error),
            };
        }
        match result {
            Ok(true) => summary.synced += 1,
            Ok(false) => summary.skipped += 1,
            Err(error) => {
                // Creation can fail before `sync_one` has a document id to persist.
                // Keep that error as a per-note record so the UI can explain the
                // actual Feishu permission/folder/network problem to the user.
                let _ = mark_failed(store, &note.id, &error);
                summary.failed += 1;
            }
        }
    }
    Ok(summary)
}

fn emit_overview(app: &AppHandle) {
    if let Ok(store) = super::notes::default_store() {
        let runtime = app.state::<FeishuSyncRuntime>();
        if let Ok(status) = overview(&store, &runtime) {
            let _ = app.emit("feishu-sync-status", status);
        }
    }
}

pub fn enqueue_note(app: &AppHandle, note_id: String) {
    let runtime = app.state::<FeishuSyncRuntime>();
    let generation = runtime.next_generation(&note_id);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(SYNC_DEBOUNCE).await;
        let runtime = app.state::<FeishuSyncRuntime>();
        if !runtime.is_current(&note_id, generation) {
            return;
        }

        runtime.set_active(1);
        emit_overview(&app);
        for attempt in 0..=RETRY_DELAYS.len() {
            if !runtime.is_current(&note_id, generation) {
                break;
            }
            let task_app = app.clone();
            let task_note_id = note_id.clone();
            let result = tauri::async_runtime::spawn_blocking(move || {
                let runtime = task_app.state::<FeishuSyncRuntime>();
                let _guard = runtime.sync_lock.lock().unwrap_or_else(|e| e.into_inner());
                let store = super::notes::default_store()?;
                let result = (|| {
                    let Some(client) = client_for(&store, &runtime)? else {
                        return Ok(false);
                    };
                    sync_one(&store, &runtime, &client, &task_note_id)
                })();
                if let Err(error) = &result {
                    let _ = mark_failed(&store, &task_note_id, error);
                    runtime.clear_token();
                }
                result
            })
            .await;

            if matches!(result, Ok(Ok(_))) {
                break;
            }
            if let Some(delay) = RETRY_DELAYS.get(attempt) {
                tokio::time::sleep(*delay).await;
            }
        }
        runtime.set_active(-1);
        emit_overview(&app);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_conversion_maps_common_blocks() {
        let blocks = markdown_blocks(
            "# 标题\n正文\n- 项目\n1. 顺序\n- [x] 完成\n> 引用\n```rust\nfn main() {}\n```\n---",
        );
        let types = blocks
            .iter()
            .filter_map(|block| block.get("block_type").and_then(Value::as_u64))
            .collect::<Vec<_>>();
        assert_eq!(types, vec![3, 2, 12, 13, 17, 15, 14, 22]);
    }

    #[test]
    fn markdown_conversion_preserves_unclosed_code_fence() {
        let blocks = markdown_blocks("```\n原始内容");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0]["block_type"], 14);
        assert_eq!(
            blocks[0]["code"]["elements"][0]["text_run"]["content"],
            "原始内容"
        );
    }

    #[test]
    fn long_lines_are_split_without_breaking_unicode() {
        let content = "花".repeat(MAX_TEXT_CHARS + 1);
        let blocks = markdown_blocks(&content);
        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn blank_lines_use_a_valid_text_element() {
        let blocks = markdown_blocks("第一行\n\n第三行");
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[1]["text"]["elements"][0]["text_run"]["content"], " ");
    }
}
