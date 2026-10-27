//! PocketBridge native core: all servers live here; Kotlin only hosts the UI and foreground service.

mod config;
mod gateway;
mod http;
mod notes;
mod proxy;
mod rewrite;
mod web;
mod webproxy;

use http::{Handler, Server};
use jni::objects::{JClass, JString};
use jni::sys::{jboolean, jstring};
use jni::JNIEnv;
use std::path::Path;
use std::sync::{Arc, Mutex};

static SERVERS: Mutex<Vec<Server>> = Mutex::new(Vec::new());

/// Starts all servers; returns "" on success or one line per server that failed to bind.
fn start(data_dir: &str) -> String {
    let mut servers = SERVERS.lock().unwrap();
    if !servers.is_empty() {
        return String::new();
    }
    notes::init(Path::new(data_dir));
    let all: [(&str, u16, Handler); 3] = [
        ("web", web::PORT, Arc::new(web::handle)),
        ("proxy", proxy::PORT, Arc::new(proxy::handle)),
        ("gateway", gateway::PORT, Arc::new(gateway::handle)),
    ];
    let mut errors = Vec::new();
    for (name, port, handler) in all {
        match Server::start(name, port, handler) {
            Ok(s) => servers.push(s),
            Err(e) => errors.push(format!("{name} :{port} gagal — {e}")),
        }
    }
    errors.join("\n")
}

fn stop() {
    for mut s in SERVERS.lock().unwrap().drain(..) {
        s.stop();
    }
}

fn jstr(env: &mut JNIEnv, s: &JString) -> String {
    env.get_string(s).map(String::from).unwrap_or_default()
}

#[no_mangle]
pub extern "system" fn Java_com_mks_pocketbridge_Native_start<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    data_dir: JString<'l>,
) -> jstring {
    let dir = jstr(&mut env, &data_dir);
    let msg = start(&dir);
    env.new_string(msg).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_mks_pocketbridge_Native_stop<'l>(_env: JNIEnv<'l>, _class: JClass<'l>) {
    stop();
}

#[no_mangle]
pub extern "system" fn Java_com_mks_pocketbridge_Native_init<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    data_dir: JString<'l>,
) -> jboolean {
    let dir = jstr(&mut env, &data_dir);
    config::init(Path::new(&dir)) as jboolean
}

fn config_string(env: &mut JNIEnv, pick: fn(&config::Config) -> &str) -> jstring {
    let c = config::get();
    env.new_string(pick(&c)).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_mks_pocketbridge_Native_getPassword<'l>(mut env: JNIEnv<'l>, _class: JClass<'l>) -> jstring {
    config_string(&mut env, |c| &c.password)
}

#[no_mangle]
pub extern "system" fn Java_com_mks_pocketbridge_Native_getUpstream<'l>(mut env: JNIEnv<'l>, _class: JClass<'l>) -> jstring {
    config_string(&mut env, |c| &c.upstream)
}

#[no_mangle]
pub extern "system" fn Java_com_mks_pocketbridge_Native_getApiKey<'l>(mut env: JNIEnv<'l>, _class: JClass<'l>) -> jstring {
    config_string(&mut env, |c| &c.api_key)
}

#[no_mangle]
pub extern "system" fn Java_com_mks_pocketbridge_Native_setConfig<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    password: JString<'l>,
    upstream: JString<'l>,
    api_key: JString<'l>,
) {
    let cfg = config::Config {
        password: jstr(&mut env, &password),
        upstream: jstr(&mut env, &upstream),
        api_key: jstr(&mut env, &api_key),
    };
    config::set(cfg);
}
