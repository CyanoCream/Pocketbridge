//! Shared notes, persisted as one JSON file. `version` changes on every edit so clients can poll cheaply.

use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

struct Note {
    id: String,
    text: String,
    created: i64,
    updated: i64,
}

impl Note {
    fn to_json(&self) -> Value {
        json!({ "id": self.id, "text": self.text, "created": self.created, "updated": self.updated })
    }
}

struct Store {
    file: PathBuf,
    notes: Vec<Note>,
    version: i64,
}

static STORE: Mutex<Option<Store>> = Mutex::new(None);
static COUNTER: AtomicU64 = AtomicU64::new(0);

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn new_id() -> String {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("{nanos:x}-{:x}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

pub fn init(dir: &Path) {
    let mut guard = STORE.lock().unwrap();
    if guard.is_some() {
        return;
    }
    let file = dir.join("notes.json");
    let mut store = Store { file, notes: Vec::new(), version: 0 };
    if let Ok(v) = fs::read_to_string(&store.file).map(|s| serde_json::from_str::<Value>(&s)) {
        if let Ok(v) = v {
            store.version = v["v"].as_i64().unwrap_or(0);
            for n in v["notes"].as_array().into_iter().flatten() {
                store.notes.push(Note {
                    id: n["id"].as_str().unwrap_or_default().to_string(),
                    text: n["text"].as_str().unwrap_or_default().to_string(),
                    created: n["created"].as_i64().unwrap_or(0),
                    updated: n["updated"].as_i64().unwrap_or(0),
                });
            }
        }
    }
    // Migrate the old single-clipboard file from v1.
    let old = dir.join("clipboard.txt");
    if let Ok(text) = fs::read_to_string(&old) {
        if !text.trim().is_empty() {
            create_in(&mut store, text);
        }
        let _ = fs::remove_file(old);
    }
    *guard = Some(store);
}

fn with<T>(f: impl FnOnce(&mut Store) -> T) -> T {
    let mut guard = STORE.lock().unwrap();
    f(guard.as_mut().expect("notes not initialised"))
}

pub fn version() -> i64 {
    with(|s| s.version)
}

pub fn list_json() -> String {
    with(|s| {
        let mut sorted: Vec<&Note> = s.notes.iter().collect();
        sorted.sort_by(|a, b| b.updated.cmp(&a.updated));
        json!({ "v": s.version, "notes": sorted.iter().map(|n| n.to_json()).collect::<Vec<_>>() }).to_string()
    })
}

pub fn create(text: String) -> String {
    with(|s| {
        let mut v = create_in(s, text);
        v["v"] = json!(s.version);
        v.to_string()
    })
}

pub fn update(id: &str, text: String) -> Option<String> {
    with(|s| {
        let i = s.notes.iter().position(|n| n.id == id)?;
        if s.notes[i].text != text {
            s.notes[i].text = text;
            s.notes[i].updated = now_ms();
            changed(s);
        }
        let mut v = s.notes[i].to_json();
        v["v"] = json!(s.version);
        Some(v.to_string())
    })
}

pub fn delete(id: &str) -> i64 {
    with(|s| {
        let before = s.notes.len();
        s.notes.retain(|n| n.id != id);
        if s.notes.len() != before {
            changed(s);
        }
        s.version
    })
}

fn create_in(s: &mut Store, text: String) -> Value {
    let now = now_ms();
    let n = Note { id: new_id(), text, created: now, updated: now };
    let v = n.to_json();
    s.notes.push(n);
    changed(s);
    v
}

fn changed(s: &mut Store) {
    s.version = now_ms().max(s.version + 1);
    let data = json!({ "v": s.version, "notes": s.notes.iter().map(|n| n.to_json()).collect::<Vec<_>>() });
    let tmp = s.file.with_extension("json.tmp");
    if fs::write(&tmp, data.to_string()).is_ok() {
        let _ = fs::rename(&tmp, &s.file);
    }
}
