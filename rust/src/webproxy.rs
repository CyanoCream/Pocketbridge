//! GET/POST /p?u=<url> — the phone fetches the page, rewrites links so they come back through /p,
//! and the laptop sees it inside the /lite iframe.

use crate::http::*;
use crate::rewrite;
use regex::Regex;
use std::io::{self, Read, Write};
use std::sync::LazyLock;
use std::time::Duration;
use url::Url;

const HTML: &str = "text/html; charset=utf-8";
const TEXT: &str = "text/plain; charset=utf-8";
const FORWARD_REQ: [&str; 8] = [
    "Accept", "Accept-Language", "Content-Type", "Range", "If-None-Match", "If-Modified-Since", "User-Agent",
    "X-Requested-With",
];
const FORWARD_RESP: [&str; 6] =
    ["Cache-Control", "Last-Modified", "ETag", "Content-Disposition", "Content-Range", "Accept-Ranges"];

/// Shared agent: keeps the phone-side cookie jar between requests.
static AGENT: LazyLock<ureq::Agent> = LazyLock::new(|| {
    ureq::AgentBuilder::new()
        .redirects(0)
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(60))
        .build()
});
static CT_CHARSET: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)charset="?([\w.:-]+)"#).unwrap());
static META_CHARSET: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)<meta[^>]+charset=["']?([\w.:-]+)"#).unwrap());

pub fn original_from_proxied(referer: &str) -> Option<String> {
    let i = referer.find("/p?")?;
    form_value(&referer[i + 3..], "u")
}

pub fn handle<W: Write>(req: &Request, w: &mut W) -> io::Result<bool> {
    let ka = req.keep_alive();
    let raw = req.query("u").map(|s| s.trim().to_string()).unwrap_or_default();
    if raw.is_empty() {
        redirect(w, "/lite", 302, ka, vec![])?;
        return Ok(ka);
    }
    let lower = raw.to_ascii_lowercase();
    let target = if lower.starts_with("http://") || lower.starts_with("https://") { raw.clone() } else { format!("https://{raw}") };
    let Ok(url) = Url::parse(&target) else {
        text(w, 400, &format!("URL tidak valid: {raw}"), TEXT, vec![], ka)?;
        return Ok(ka);
    };

    let method = match req.method.as_str() {
        m @ ("GET" | "POST" | "HEAD" | "PUT" | "DELETE" | "OPTIONS") => m,
        _ => "GET",
    };
    let mut r = AGENT.request(method, url.as_str());
    for h in FORWARD_REQ {
        if let Some(v) = req.header(h) {
            r = r.set(h, v);
        }
    }
    if let Some(orig) = req.header("referer").and_then(original_from_proxied) {
        r = r.set("Referer", &orig);
    }
    if req.header("origin").is_some() {
        r = r.set("Origin", &url.origin().ascii_serialization());
    }
    let result = if !req.body.is_empty() && method != "GET" && method != "HEAD" { r.send_bytes(&req.body) } else { r.call() };
    let resp = match result {
        Ok(x) | Err(ureq::Error::Status(_, x)) => x,
        Err(e) => {
            text(w, 502, &error_page(url.as_str(), &e.to_string()), HTML, vec![], ka)?;
            return Ok(ka);
        }
    };

    let code = resp.status();
    if (300..400).contains(&code) {
        if let Some(abs) = resp.header("location").and_then(|loc| url.join(loc).ok()) {
            let code = if code == 307 || code == 308 { code } else { 302 };
            redirect(w, &rewrite::wrap(abs.as_str()), code, ka, vec![])?;
            return Ok(ka);
        }
    }

    let mut headers: Headers = FORWARD_RESP.iter().filter_map(|&h| resp.header(h).map(|v| hdr(h, v))).collect();
    let ct = resp.header("content-type").unwrap_or("").to_string();
    if method == "HEAD" || code == 204 || code == 304 {
        if code == 204 || code == 304 {
            headers.push(hdr("Connection", conn_header(ka)));
            write_head(w, code, &headers, None)?;
            w.flush()?;
        } else {
            respond(w, code, (!ct.is_empty()).then_some(ct.as_str()), b"", headers, ka)?;
        }
        return Ok(ka);
    }

    let lct = ct.to_ascii_lowercase();
    let mut reader = resp.into_reader();
    let is_html = lct.contains("text/html") || lct.contains("application/xhtml");
    if is_html || lct.contains("text/css") {
        let mut bytes = Vec::new();
        if let Err(e) = reader.read_to_end(&mut bytes) {
            text(w, 502, &error_page(url.as_str(), &e.to_string()), HTML, vec![], ka)?;
            return Ok(ka);
        }
        let decoded = decode(&bytes, &ct);
        let (body, out_ct) = if is_html {
            (rewrite::html(&decoded, &url), HTML)
        } else {
            (rewrite::css(&decoded, &url), "text/css; charset=utf-8")
        };
        respond(w, code, Some(out_ct), body.as_bytes(), headers, ka)?;
    } else {
        headers.push(hdr("Content-Type", if ct.is_empty() { "application/octet-stream" } else { ct.as_str() }));
        headers.push(hdr("Transfer-Encoding", "chunked"));
        headers.push(hdr("Connection", conn_header(ka)));
        write_head(w, code, &headers, None)?;
        if stream_chunked(&mut reader, w).is_err() {
            return Ok(false);
        }
    }
    Ok(ka)
}

fn decode(bytes: &[u8], content_type: &str) -> String {
    let label = CT_CHARSET.captures(content_type).map(|c| c[1].to_string()).or_else(|| {
        let head = String::from_utf8_lossy(&bytes[..bytes.len().min(4096)]).into_owned();
        META_CHARSET.captures(&head).map(|c| c[1].to_string())
    });
    let enc = label.and_then(|l| encoding_rs::Encoding::for_label(l.as_bytes())).unwrap_or(encoding_rs::UTF_8);
    enc.decode(bytes).0.into_owned()
}

fn error_page(url: &str, msg: &str) -> String {
    let esc = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;");
    format!(
        r#"<!doctype html><meta charset="utf-8"><body style="font-family:system-ui;background:#0b0d17;color:#e8eaf6;padding:40px">
<h2>Gagal membuka halaman</h2><p style="color:#8b90b0">{}</p><pre style="white-space:pre-wrap">{}</pre>
<p style="color:#8b90b0">Pastikan HP punya koneksi internet.</p></body>"#,
        esc(url),
        esc(msg)
    )
}
