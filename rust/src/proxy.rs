//! Port 8888: plain HTTP/HTTPS forward proxy (CONNECT tunnel). Bytes are piped as-is, so heavy sites
//! (ChatGPT, Gemini, Google login) work exactly like a normal internet connection.

use crate::config;
use crate::http::*;
use base64::Engine;
use std::io::{self, BufReader, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::thread;
use std::time::Duration;
use url::Url;

pub const PORT: u16 = 8888;
const TEXT: &str = "text/plain; charset=utf-8";

pub fn handle(stream: TcpStream) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    let ip = stream.peer_addr().map(|a| a.ip().to_string()).unwrap_or_default();
    let mut r = BufReader::new(stream.try_clone()?);
    let mut w = stream.try_clone()?;
    let Some(req) = read_request(&mut r)? else { return Ok(()) };

    if !authorized(&req, &ip) {
        let extra = vec![hdr("Proxy-Authenticate", "Basic realm=\"PocketBridge\"")];
        return text(&mut w, 407, "PocketBridge: proxy butuh password", TEXT, extra, false);
    }
    stream.set_read_timeout(None)?;

    if req.method == "CONNECT" {
        let (host, port) = match req.target.rsplit_once(':') {
            Some((h, p)) => (h.trim_matches(['[', ']']), p.parse().unwrap_or(443)),
            None => (req.target.as_str(), 443),
        };
        let remote = match connect(host, port) {
            Ok(s) => s,
            Err(e) => return text(&mut w, 502, &format!("PocketBridge: gagal konek ke {host}:{port} ({e})"), TEXT, vec![], false),
        };
        w.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")?;
        return tunnel(r, stream, remote);
    }

    let url = match Url::parse(&req.target) {
        Ok(u) if u.scheme() == "http" && u.host_str().is_some() => u,
        _ => return text(&mut w, 400, "Ini HTTP proxy. Set sebagai proxy di browser/OS, bukan dibuka langsung.", TEXT, vec![], false),
    };
    let host = url.host_str().unwrap_or_default().to_string();
    let port = url.port_or_known_default().unwrap_or(80);
    let mut remote = match connect(&host, port) {
        Ok(s) => s,
        Err(e) => return text(&mut w, 502, &format!("PocketBridge: gagal konek ke {host}:{port} ({e})"), TEXT, vec![], false),
    };
    let path = match url.query() {
        Some(q) => format!("{}?{}", url.path(), q),
        None => url.path().to_string(),
    };
    let mut head = format!("{} {} HTTP/1.1\r\n", req.method, path);
    let mut has_host = false;
    for (k, v) in &req.headers {
        let l = k.to_ascii_lowercase();
        if l.starts_with("proxy-") || l == "connection" || l == "keep-alive" {
            continue;
        }
        has_host |= l == "host";
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    if !has_host {
        head.push_str(&format!("Host: {}\r\n", url.authority()));
    }
    head.push_str("Connection: close\r\n\r\n");
    remote.write_all(&latin1(&head))?;
    tunnel(r, stream, remote)
}

fn authorized(req: &Request, ip: &str) -> bool {
    let cfg = config::get();
    if cfg.password.is_empty() || config::is_trusted(ip, &config::auth_token(&cfg.password)) {
        return true;
    }
    let Some(h) = req.header("proxy-authorization") else { return false };
    let Some(b64) = h.get(6..).filter(|_| h[..6].eq_ignore_ascii_case("basic ")) else { return false };
    let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(b64.trim()) else { return false };
    let decoded = String::from_utf8_lossy(&decoded);
    decoded.split_once(':').map_or(false, |(_, pw)| pw == cfg.password)
}

fn connect(host: &str, port: u16) -> io::Result<TcpStream> {
    let mut last = None;
    for addr in (host, port).to_socket_addrs()? {
        match TcpStream::connect_timeout(&addr, Duration::from_secs(15)) {
            Ok(s) => return Ok(s),
            Err(e) => last = Some(e),
        }
    }
    Err(last.unwrap_or_else(|| bad("no address")))
}

/// Pipes bytes both ways until either side closes. `client_r` may already hold buffered bytes.
fn tunnel(mut client_r: BufReader<TcpStream>, client: TcpStream, remote: TcpStream) -> io::Result<()> {
    let mut remote_w = remote.try_clone()?;
    let up = thread::spawn(move || {
        let _ = io::copy(&mut client_r, &mut remote_w);
        let _ = remote_w.shutdown(Shutdown::Write);
    });
    let mut remote_r = remote;
    let mut client_w = client.try_clone()?;
    let _ = io::copy(&mut remote_r, &mut client_w);
    let _ = client.shutdown(Shutdown::Both);
    let _ = remote_r.shutdown(Shutdown::Both);
    let _ = up.join();
    Ok(())
}
