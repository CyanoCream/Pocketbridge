//! Port 8080: home picker (/), shared notes (/notes), AI gateway settings (/gateway),
//! browser launcher setup (/setup), and the lightweight in-page browser (/lite, /p).

use crate::config;
use crate::gateway;
use crate::http::*;
use crate::notes;
use crate::proxy;
use crate::rewrite;
use crate::webproxy;
use serde_json::{json, Value};
use std::io::{self, BufReader, BufWriter, Write};
use std::net::TcpStream;
use std::time::Duration;
use url::Url;

pub const PORT: u16 = 8080;
const JSON: &str = "application/json; charset=utf-8";
const HTML: &str = "text/html; charset=utf-8";
const TEXT: &str = "text/plain; charset=utf-8";

static HOME: &str = include_str!("../web/home.html");
static NOTES: &str = include_str!("../web/index.html");
static GATEWAY: &str = include_str!("../web/gateway.html");
static SETUP: &str = include_str!("../web/internet.html");
static LITE: &str = include_str!("../web/browser.html");
static LOGIN: &str = include_str!("../web/login.html");
static ICON: &str = include_str!("../web/icon.svg");
static LAUNCHER_PS1: &str = include_str!("../web/launch.ps1");

pub fn handle(stream: TcpStream) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    let ip = stream.peer_addr().map(|a| a.ip().to_string()).unwrap_or_default();
    let mut r = BufReader::new(stream.try_clone()?);
    let mut w = BufWriter::new(stream);
    loop {
        let Some(mut req) = read_request(&mut r)? else { return Ok(()) };
        read_body(&mut req, &mut r, &mut w)?;
        let keep = route(&req, &mut w, &ip)?;
        if !keep || !req.keep_alive() {
            return Ok(());
        }
    }
}

/// Returns true when the connection can be reused.
fn route<W: Write>(req: &Request, w: &mut W, ip: &str) -> io::Result<bool> {
    let ka = req.keep_alive();
    let path = req.path();
    let cfg = config::get();
    match path {
        "/favicon.ico" | "/icon.svg" => {
            text(w, 200, ICON, "image/svg+xml", vec![hdr("Cache-Control", "max-age=86400")], ka)?;
            return Ok(ka);
        }
        "/login" => {
            login(req, w, ka, ip, &cfg.password)?;
            return Ok(ka);
        }
        "/logout" => {
            config::untrust(ip);
            redirect(w, "/login", 303, ka, vec![hdr("Set-Cookie", "pb_auth=; Path=/; Max-Age=0")])?;
            return Ok(ka);
        }
        // Lets the Windows launcher confirm an address really is PocketBridge before using it as a proxy.
        "/pb-ping" => {
            text(w, 200, "pocketbridge", TEXT, vec![], ka)?;
            return Ok(ka);
        }
        // Downloaded by the setup .bat with curl (no cookie); it contains no secrets.
        "/launch/launch.ps1" => {
            text(w, 200, LAUNCHER_PS1, TEXT, vec![], ka)?;
            return Ok(ka);
        }
        // Fetched by the OS/browser without cookies, so it stays public; it only contains the phone's IP.
        "/proxy.pac" => {
            let host = host_of(req);
            let port = proxy::PORT;
            let pac = format!(
                "function FindProxyForURL(url, host) {{\n  if (isPlainHostName(host) || host === \"{host}\" || host === \"localhost\" || host === \"127.0.0.1\") return \"DIRECT\";\n  return \"PROXY {host}:{port}\";\n}}\n"
            );
            text(w, 200, &pac, "application/x-ns-proxy-autoconfig", vec![], ka)?;
            return Ok(ka);
        }
        _ => {}
    }

    let token = config::auth_token(&cfg.password);
    if !cfg.password.is_empty() {
        if req.cookie("pb_auth").as_deref() != Some(token.as_str()) {
            if path.starts_with("/api/") {
                text(w, 401, r#"{"error":"unauthorized"}"#, JSON, vec![], ka)?;
            } else {
                redirect(w, "/login", 302, ka, vec![])?;
            }
            return Ok(ka);
        }
        // A device that can open the web UI may also use the proxy without a password prompt.
        config::trust(ip, &token);
    }

    let no_store = || vec![hdr("Cache-Control", "no-store")];
    match (path, req.method.as_str()) {
        ("/" | "/index.html", _) => text(w, 200, HOME, HTML, no_store(), ka)?,
        ("/notes", _) => text(w, 200, NOTES, HTML, no_store(), ka)?,
        ("/gateway", _) => text(w, 200, GATEWAY, HTML, no_store(), ka)?,
        ("/setup", _) => text(w, 200, SETUP, HTML, no_store(), ka)?,
        ("/browser", _) => redirect(w, "/setup", 301, ka, vec![])?,
        ("/lite", _) => text(w, 200, LITE, HTML, no_store(), ka)?,
        ("/api/config", "GET") => text(w, 200, &config_json(), JSON, no_store(), ka)?,
        ("/api/config", "POST") => match update_config(&req.body) {
            Ok(()) => text(w, 200, &config_json(), JSON, no_store(), ka)?,
            Err(msg) => text(w, 400, &json!({ "error": msg }).to_string(), JSON, vec![], ka)?,
        },
        ("/launch/PocketBridge-Setup.bat", _) => {
            let bat = windows_setup(&host_of(req)).replace('\n', "\r\n");
            let extra = vec![hdr("Content-Disposition", "attachment; filename=\"PocketBridge-Setup.bat\"")];
            text(w, 200, &bat, "application/octet-stream", extra, ka)?;
        }
        ("/api/notes", "GET") => {
            let since = req.query("since").and_then(|s| s.parse::<i64>().ok());
            let v = notes::version();
            let body = if since == Some(v) { format!(r#"{{"v":{v},"same":true}}"#) } else { notes::list_json() };
            text(w, 200, &body, JSON, no_store(), ka)?;
        }
        ("/api/notes", "POST") => text(w, 200, &notes::create(req.body_str()), JSON, vec![], ka)?,
        ("/p", _) => return webproxy::handle(req, w),
        _ if path.starts_with("/api/notes/") => {
            let id = &path["/api/notes/".len()..];
            match req.method.as_str() {
                "PUT" | "POST" => match notes::update(id, req.body_str()) {
                    Some(json) => text(w, 200, &json, JSON, vec![], ka)?,
                    None => text(w, 404, r#"{"error":"not found"}"#, JSON, vec![], ka)?,
                },
                "DELETE" => {
                    let v = notes::delete(id);
                    text(w, 200, &format!(r#"{{"ok":true,"v":{v}}}"#), JSON, vec![], ka)?;
                }
                _ => text(w, 405, r#"{"error":"method"}"#, JSON, vec![], ka)?,
            }
        }
        _ => {
            // Absolute paths that escaped rewriting inside a proxied page (e.g. "/search?q=").
            let abs = req
                .header("referer")
                .and_then(webproxy::original_from_proxied)
                .and_then(|orig| Url::parse(&orig).ok()?.join(&req.target).ok());
            match abs {
                Some(abs) => redirect(w, &rewrite::wrap(abs.as_str()), 307, ka, vec![])?,
                None => text(w, 404, "Not found", TEXT, vec![], ka)?,
            }
        }
    }
    Ok(ka)
}

/// Public view of the settings: the API key itself is never sent back, only its last 4 characters.
fn config_json() -> String {
    let c = config::get();
    let hint = if c.api_key.chars().count() > 8 {
        let tail: String = c.api_key.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect();
        format!("…{tail}")
    } else {
        String::new()
    };
    json!({
        "upstream": c.upstream,
        "hasKey": !c.api_key.is_empty(),
        "keyHint": hint,
        "hasPassword": !c.password.is_empty(),
        "proxyPort": proxy::PORT,
        "gatewayPort": gateway::PORT,
    })
    .to_string()
}

/// Applies `{"upstream"?, "apiKey"?, "clearKey"?}` from the web UI. Changing the upstream drops the stored
/// key unless a new one is given, so nobody on the LAN can redirect the phone's key to their own server.
fn update_config(body: &[u8]) -> Result<(), &'static str> {
    let v: Value = serde_json::from_slice(body).map_err(|_| "JSON tidak valid")?;
    let mut c = config::get();
    if let Some(u) = v["upstream"].as_str() {
        let u = u.trim().trim_end_matches('/').to_string();
        let lower = u.to_ascii_lowercase();
        if !u.is_empty() && !(lower.starts_with("http://") || lower.starts_with("https://")) {
            return Err("Upstream harus diawali http:// atau https://");
        }
        if u != c.upstream {
            c.upstream = u;
            c.api_key.clear();
        }
    }
    if v["clearKey"].as_bool() == Some(true) {
        c.api_key.clear();
    }
    if let Some(k) = v["apiKey"].as_str().map(str::trim).filter(|k| !k.is_empty()) {
        c.api_key = k.to_string();
    }
    config::set(c);
    Ok(())
}

fn host_of(req: &Request) -> String {
    req.header("host")
        .map(|h| h.rsplit_once(':').map_or(h, |(host, _)| host))
        .filter(|h| !h.is_empty())
        .unwrap_or("127.0.0.1")
        .to_string()
}

fn login<W: Write>(req: &Request, w: &mut W, ka: bool, ip: &str, password: &str) -> io::Result<()> {
    if password.is_empty() {
        return redirect(w, "/", 302, ka, vec![]);
    }
    let mut error = "";
    if req.method == "POST" {
        if form_value(&req.body_str(), "password").as_deref() == Some(password) {
            let token = config::auth_token(password);
            config::trust(ip, &token);
            let cookie = format!("pb_auth={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age=2592000");
            return redirect(w, "/", 303, ka, vec![hdr("Set-Cookie", cookie)]);
        }
        error = "Password salah";
    }
    let html = LOGIN.replace("{{ERROR}}", error);
    text(w, 200, &html, HTML, vec![hdr("Cache-Control", "no-store")], ka)
}

/// One-time setup: downloads launch.ps1 from the phone, then registers pocketbridge:// links and
/// per-browser shortcuts (Chrome, Edge, Brave, Firefox).
fn windows_setup(host: &str) -> String {
    format!(
        r#"@echo off
setlocal
set "DIR=%LOCALAPPDATA%\PocketBridge"
if not exist "%DIR%" mkdir "%DIR%"
echo Mengunduh PocketBridge launcher dari HP...
curl.exe -fsS -o "%DIR%\launch.ps1" "http://{host}:{PORT}/launch/launch.ps1"
if errorlevel 1 (
  echo Gagal mengunduh. Pastikan server PocketBridge di HP menyala.
  pause
  exit /b 1
)
powershell -NoProfile -ExecutionPolicy Bypass -File "%DIR%\launch.ps1" -Install -PhoneHost {host}
"#
    )
}
