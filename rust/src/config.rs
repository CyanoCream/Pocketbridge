use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, RwLock};

/// Settings shared by the Android UI and the web UI; persisted to `config.json` in the app data dir.
#[derive(Clone)]
pub struct Config {
    pub password: String,
    /// Base URL of the AI provider, e.g. https://api.openai.com/v1
    pub upstream: String,
    /// Optional key injected by the phone so the laptop never needs the real one.
    pub api_key: String,
}

static CONFIG: RwLock<Config> = RwLock::new(Config {
    password: String::new(),
    upstream: String::new(),
    api_key: String::new(),
});
static FILE: Mutex<Option<PathBuf>> = Mutex::new(None);

/// ip → auth token at the time that device logged in to the web UI.
static TRUSTED: LazyLock<Mutex<HashMap<String, String>>> = LazyLock::new(Default::default);

/// Loads `config.json` from `dir`. Returns false when it does not exist yet (caller may migrate old settings).
pub fn init(dir: &Path) -> bool {
    let file = dir.join("config.json");
    *FILE.lock().unwrap() = Some(file.clone());
    let Some(v) = fs::read_to_string(&file).ok().and_then(|s| serde_json::from_str::<Value>(&s).ok()) else {
        return false;
    };
    let field = |k: &str| v[k].as_str().unwrap_or_default().to_string();
    *CONFIG.write().unwrap() = Config { password: field("password"), upstream: field("upstream"), api_key: field("api_key") };
    true
}

pub fn get() -> Config {
    CONFIG.read().unwrap().clone()
}

pub fn set(c: Config) {
    let data = json!({ "password": c.password, "upstream": c.upstream, "api_key": c.api_key }).to_string();
    *CONFIG.write().unwrap() = c;
    if let Some(file) = FILE.lock().unwrap().clone() {
        let tmp = file.with_extension("json.tmp");
        if fs::write(&tmp, data).is_ok() {
            let _ = fs::rename(&tmp, &file);
        }
    }
}

/// Cookie value proving the browser knows the password; changes when the password changes.
pub fn auth_token(password: &str) -> String {
    Sha256::digest(format!("pocketbridge|{password}").as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn trust(ip: &str, token: &str) {
    TRUSTED.lock().unwrap().insert(ip.to_string(), token.to_string());
}

pub fn untrust(ip: &str) {
    TRUSTED.lock().unwrap().remove(ip);
}

pub fn is_trusted(ip: &str, token: &str) -> bool {
    TRUSTED.lock().unwrap().get(ip).is_some_and(|t| t == token)
}
