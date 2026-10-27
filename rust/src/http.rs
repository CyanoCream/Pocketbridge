//! Minimal HTTP/1.1 on top of std sockets — just enough for the three servers.

use std::collections::HashMap;
use std::io::{self, BufRead, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const MAX_LINE: u64 = 64 * 1024;
const MAX_BODY: usize = 64 * 1024 * 1024;

pub type Headers = Vec<(String, String)>;

pub fn hdr(k: &str, v: impl Into<String>) -> (String, String) {
    (k.to_string(), v.into())
}

pub fn bad(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.to_string())
}

pub fn conn_header(keep_alive: bool) -> &'static str {
    if keep_alive { "keep-alive" } else { "close" }
}

pub struct Request {
    pub method: String,
    pub target: String,
    pub headers: Headers,
    pub body: Vec<u8>,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }

    pub fn path(&self) -> &str {
        self.target.split('?').next().unwrap_or("")
    }

    pub fn query(&self, key: &str) -> Option<String> {
        let q = self.target.split_once('?')?.1;
        form_value(q, key)
    }

    pub fn cookie(&self, name: &str) -> Option<String> {
        self.header("cookie")?
            .split(';')
            .map(str::trim)
            .find_map(|kv| kv.strip_prefix(name)?.strip_prefix('=').map(str::to_string))
    }

    pub fn keep_alive(&self) -> bool {
        !self.header("connection").is_some_and(|v| v.eq_ignore_ascii_case("close"))
    }

    pub fn body_str(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

pub fn form_value(s: &str, key: &str) -> Option<String> {
    url::form_urlencoded::parse(s.as_bytes()).find(|(k, _)| k == key).map(|(_, v)| v.into_owned())
}

/// Percent-encodes everything except unreserved characters (like encodeURIComponent).
pub fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Reads one line as Latin-1 (bytes round-trip unchanged), without the trailing CRLF.
pub fn read_line<R: BufRead>(r: &mut R) -> io::Result<Option<String>> {
    let mut buf = Vec::new();
    let n = r.by_ref().take(MAX_LINE).read_until(b'\n', &mut buf)?;
    if n == 0 {
        return Ok(None);
    }
    if buf.last() != Some(&b'\n') && n as u64 >= MAX_LINE {
        return Err(bad("line too long"));
    }
    while matches!(buf.last(), Some(b'\n' | b'\r')) {
        buf.pop();
    }
    Ok(Some(buf.iter().map(|&b| b as char).collect()))
}

pub fn latin1(s: &str) -> Vec<u8> {
    s.chars().map(|c| c as u32 as u8).collect()
}

pub fn read_request<R: BufRead>(r: &mut R) -> io::Result<Option<Request>> {
    let mut line = match read_line(r)? {
        Some(l) => l,
        None => return Ok(None),
    };
    while line.is_empty() {
        line = match read_line(r)? {
            Some(l) => l,
            None => return Ok(None),
        };
    }
    let mut parts = line.split(' ');
    let method = parts.next().unwrap_or("").to_ascii_uppercase();
    let target = parts.next().ok_or_else(|| bad("bad request line"))?.to_string();
    let mut headers = Vec::new();
    loop {
        let h = read_line(r)?.ok_or_else(|| bad("eof in headers"))?;
        if h.is_empty() {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            headers.push((k.trim().to_string(), v.trim().to_string()));
        }
        if headers.len() > 200 {
            return Err(bad("too many headers"));
        }
    }
    Ok(Some(Request { method, target, headers, body: Vec::new() }))
}

pub fn read_body<R: BufRead, W: Write>(req: &mut Request, r: &mut R, w: &mut W) -> io::Result<()> {
    if req.header("expect").is_some_and(|v| v.eq_ignore_ascii_case("100-continue")) {
        w.write_all(b"HTTP/1.1 100 Continue\r\n\r\n")?;
        w.flush()?;
    }
    let chunked = req.header("transfer-encoding").is_some_and(|v| v.to_ascii_lowercase().contains("chunked"));
    if chunked {
        let mut out = Vec::new();
        loop {
            let size_line = read_line(r)?.ok_or_else(|| bad("eof in chunk"))?;
            let size = usize::from_str_radix(size_line.split(';').next().unwrap_or("").trim(), 16)
                .map_err(|_| bad("bad chunk size"))?;
            if size == 0 {
                while let Some(t) = read_line(r)? {
                    if t.is_empty() {
                        break;
                    }
                }
                break;
            }
            if out.len() + size > MAX_BODY {
                return Err(bad("body too large"));
            }
            let start = out.len();
            out.resize(start + size, 0);
            r.read_exact(&mut out[start..])?;
            read_line(r)?;
        }
        req.body = out;
    } else {
        let len: usize = req.header("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
        if len > MAX_BODY {
            return Err(bad("body too large"));
        }
        let mut b = vec![0u8; len];
        r.read_exact(&mut b)?;
        req.body = b;
    }
    Ok(())
}

pub fn reason(code: u16) -> &'static str {
    match code {
        200 => "OK",
        204 => "No Content",
        206 => "Partial Content",
        301 => "Moved Permanently",
        302 => "Found",
        303 => "See Other",
        304 => "Not Modified",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        407 => "Proxy Authentication Required",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Status",
    }
}

pub fn write_head<W: Write>(w: &mut W, code: u16, headers: &[(String, String)], reason_text: Option<&str>) -> io::Result<()> {
    let reason_text = reason_text.filter(|r| !r.trim().is_empty()).unwrap_or(reason(code));
    let mut s = format!("HTTP/1.1 {code} {reason_text}\r\n");
    for (k, v) in headers {
        s.push_str(k);
        s.push_str(": ");
        s.push_str(&v.replace(['\r', '\n'], ""));
        s.push_str("\r\n");
    }
    s.push_str("\r\n");
    w.write_all(s.as_bytes())
}

pub fn respond<W: Write>(w: &mut W, code: u16, ctype: Option<&str>, body: &[u8], extra: Headers, keep_alive: bool) -> io::Result<()> {
    let mut h = Vec::with_capacity(extra.len() + 3);
    if let Some(ct) = ctype {
        h.push(hdr("Content-Type", ct));
    }
    h.push(hdr("Content-Length", body.len().to_string()));
    h.push(hdr("Connection", conn_header(keep_alive)));
    h.extend(extra);
    write_head(w, code, &h, None)?;
    w.write_all(body)?;
    w.flush()
}

pub fn text<W: Write>(w: &mut W, code: u16, s: &str, ctype: &str, extra: Headers, keep_alive: bool) -> io::Result<()> {
    respond(w, code, Some(ctype), s.as_bytes(), extra, keep_alive)
}

pub fn redirect<W: Write>(w: &mut W, location: &str, code: u16, keep_alive: bool, mut extra: Headers) -> io::Result<()> {
    extra.insert(0, hdr("Location", location));
    respond(w, code, None, b"", extra, keep_alive)
}

/// Copies `r` as a chunked body, flushing every read so SSE/streaming arrives immediately.
pub fn stream_chunked<R: Read + ?Sized, W: Write>(r: &mut R, w: &mut W) -> io::Result<()> {
    let mut buf = [0u8; 16 * 1024];
    loop {
        let n = match r.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        write!(w, "{n:x}\r\n")?;
        w.write_all(&buf[..n])?;
        w.write_all(b"\r\n")?;
        w.flush()?;
    }
    end_chunked(w)
}

pub fn end_chunked<W: Write>(w: &mut W) -> io::Result<()> {
    w.write_all(b"0\r\n\r\n")?;
    w.flush()
}

pub type Handler = Arc<dyn Fn(TcpStream) -> io::Result<()> + Send + Sync>;

/// Thread-per-connection TCP server that can be stopped and restarted on the same port.
pub struct Server {
    port: u16,
    stop: Arc<AtomicBool>,
    conns: Arc<Mutex<HashMap<u64, TcpStream>>>,
    accept: Option<thread::JoinHandle<()>>,
}

impl Server {
    pub fn start(name: &str, port: u16, handler: Handler) -> io::Result<Server> {
        let listener = TcpListener::bind(("0.0.0.0", port))?;
        let stop = Arc::new(AtomicBool::new(false));
        let conns: Arc<Mutex<HashMap<u64, TcpStream>>> = Arc::new(Mutex::new(HashMap::new()));
        let (stop2, conns2) = (stop.clone(), conns.clone());
        let next = AtomicU64::new(0);
        let accept = thread::Builder::new().name(format!("{name}-accept")).spawn(move || {
            for stream in listener.incoming() {
                if stop2.load(Ordering::SeqCst) {
                    break;
                }
                let stream = match stream {
                    Ok(s) => s,
                    Err(_) => {
                        thread::sleep(Duration::from_millis(50));
                        continue;
                    }
                };
                let id = next.fetch_add(1, Ordering::Relaxed);
                if let Ok(clone) = stream.try_clone() {
                    conns2.lock().unwrap().insert(id, clone);
                }
                let (h, conns3) = (handler.clone(), conns2.clone());
                let spawned = thread::Builder::new().spawn(move || {
                    let _ = h(stream);
                    conns3.lock().unwrap().remove(&id);
                });
                if spawned.is_err() {
                    conns2.lock().unwrap().remove(&id);
                }
            }
        })?;
        Ok(Server { port, stop, conns, accept: Some(accept) })
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // Wake the blocking accept() so the listener is dropped and the port freed.
        let _ = TcpStream::connect_timeout(&([127, 0, 0, 1], self.port).into(), Duration::from_secs(1));
        if let Some(t) = self.accept.take() {
            let _ = t.join();
        }
        for (_, c) in self.conns.lock().unwrap().drain() {
            let _ = c.shutdown(Shutdown::Both);
        }
    }
}
