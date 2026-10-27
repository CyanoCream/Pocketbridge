//! Port 8090: reverse proxy to the AI provider configured on the phone.
//! Point a coding agent's base URL at http://<phone-ip>:8090 — streaming (SSE) passes through as-is.

use crate::config;
use crate::http::*;
use std::collections::HashSet;
use std::io::{self, BufReader, BufWriter};
use std::net::TcpStream;
use std::sync::LazyLock;
use std::time::Duration;
use url::Url;

pub const PORT: u16 = 8090;
const JSON: &str = "application/json; charset=utf-8";
const HOP_REQ: [&str; 11] = [
    "host", "connection", "content-length", "transfer-encoding", "accept-encoding", "keep-alive",
    "proxy-connection", "proxy-authorization", "te", "trailer", "upgrade",
];
const HOP_RESP: [&str; 5] = ["transfer-encoding", "content-length", "connection", "content-encoding", "keep-alive"];
const KEY_HEADERS: [&str; 4] = ["authorization", "x-api-key", "x-goog-api-key", "api-key"];

static AGENT: LazyLock<ureq::Agent> =
    LazyLock::new(|| ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(30)).build());

fn cors() -> Headers {
    vec![
        hdr("Access-Control-Allow-Origin", "*"),
        hdr("Access-Control-Allow-Headers", "*"),
        hdr("Access-Control-Allow-Methods", "GET, POST, PUT, PATCH, DELETE, OPTIONS"),
    ]
}

fn error_json(msg: &str) -> String {
    serde_json::json!({ "error": { "message": format!("PocketBridge: {msg}") } }).to_string()
}

pub fn handle(stream: TcpStream) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(60)))?;
    let mut r = BufReader::new(stream.try_clone()?);
    let mut w = BufWriter::new(stream.try_clone()?);
    let Some(mut req) = read_request(&mut r)? else { return Ok(()) };
    read_body(&mut req, &mut r, &mut w)?;
    stream.set_read_timeout(None)?;

    if req.method == "OPTIONS" {
        return respond(&mut w, 204, None, b"", cors(), false);
    }
    let cfg = config::get();
    let upstream = cfg.upstream.trim().trim_end_matches('/').to_string();
    if upstream.is_empty() {
        return text(&mut w, 503, &error_json("AI gateway upstream belum diset di HP"), JSON, cors(), false);
    }
    let key = cfg.api_key.trim().to_string();
    // When the phone holds the real key, LAN clients must present the app password as their API key.
    if !key.is_empty() && !cfg.password.is_empty() && client_key(&req).as_deref() != Some(cfg.password.as_str()) {
        return text(&mut w, 401, &error_json("API key harus = password aplikasi"), JSON, cors(), false);
    }

    let target = build_target(&upstream, &req.target);
    let mut rq = AGENT.request(&req.method, &target);
    for (k, v) in &req.headers {
        let l = k.to_ascii_lowercase();
        if HOP_REQ.contains(&l.as_str()) || (!key.is_empty() && KEY_HEADERS.contains(&l.as_str())) {
            continue;
        }
        rq = rq.set(k, v);
    }
    if !key.is_empty() {
        let had: HashSet<String> = req.headers.iter().map(|(k, _)| k.to_ascii_lowercase()).collect();
        let mut set = false;
        for h in ["x-api-key", "x-goog-api-key", "api-key"] {
            if had.contains(h) {
                rq = rq.set(h, &key);
                set = true;
            }
        }
        if had.contains("authorization") || !set {
            rq = rq.set("Authorization", &format!("Bearer {key}"));
        }
    }

    let result = if req.body.is_empty() { rq.call() } else { rq.send_bytes(&req.body) };
    let resp = match result {
        Ok(x) | Err(ureq::Error::Status(_, x)) => x,
        Err(e) => return text(&mut w, 502, &error_json(&format!("{target}: {e}")), JSON, cors(), false),
    };

    let code = resp.status();
    let reason_text = resp.status_text().to_string();
    let mut headers = Headers::new();
    for name in resp.headers_names() {
        let l = name.to_ascii_lowercase();
        if HOP_RESP.contains(&l.as_str()) || l.starts_with("access-control-") {
            continue;
        }
        for v in resp.all(&name) {
            headers.push(hdr(&name, v));
        }
    }
    headers.extend(cors());
    headers.push(hdr("Transfer-Encoding", "chunked"));
    headers.push(hdr("Connection", "close"));
    write_head(&mut w, code, &headers, Some(&reason_text))?;
    let mut reader = resp.into_reader();
    if req.method == "HEAD" { end_chunked(&mut w) } else { stream_chunked(&mut reader, &mut w) }
}

fn client_key(req: &Request) -> Option<String> {
    if let Some(a) = req.header("authorization") {
        let a = a.trim();
        let key = if a.len() > 7 && a[..7].eq_ignore_ascii_case("bearer ") { &a[7..] } else { a };
        return Some(key.trim().to_string());
    }
    ["x-api-key", "x-goog-api-key", "api-key"]
        .iter()
        .find_map(|h| req.header(h).map(str::to_string))
        .or_else(|| req.query("key"))
}

/// Joins upstream + request path, dropping a duplicated prefix (e.g. upstream …/v1 + /v1/chat).
fn build_target(upstream: &str, req_target: &str) -> String {
    let base_path = Url::parse(upstream).map(|u| u.path().trim_end_matches('/').to_string()).unwrap_or_default();
    let mut path = if req_target.starts_with('/') { req_target.to_string() } else { format!("/{req_target}") };
    if !base_path.is_empty()
        && (path == base_path || path.starts_with(&format!("{base_path}/")) || path.starts_with(&format!("{base_path}?")))
    {
        path = path[base_path.len()..].to_string();
    } else if base_path.ends_with("/v1") && path.starts_with("/v1/") {
        path = path[3..].to_string();
    }
    format!("{upstream}{path}")
}
