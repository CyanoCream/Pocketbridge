//! Rewrites proxied HTML/CSS so every URL comes back through /p?u=…

use crate::http::url_encode;
use regex::{Captures, Regex};
use std::sync::LazyLock;
use url::Url;

macro_rules! re {
    ($name:ident, $pat:expr) => {
        static $name: LazyLock<Regex> = LazyLock::new(|| Regex::new($pat).unwrap());
    };
}

re!(SKIP, r"(?i)^(#|javascript:|data:|mailto:|tel:|about:|blob:|sms:)");
re!(BASE, r"(?i)<base\b[^>]*>");
re!(BASE_HREF, r#"(?i)href\s*=\s*["']?([^"'\s>]+)"#);
re!(CSP_META, r#"(?i)<meta[^>]+http-equiv\s*=\s*["']?content-security-policy[^>]*>"#);
re!(INTEGRITY, r#"(?i)\sintegrity\s*=\s*("[^"]*"|'[^']*')"#);
re!(REFRESH, r#"(?i)(<meta[^>]+http-equiv\s*=\s*["']?refresh["']?[^>]*content\s*=\s*["']\s*\d+\s*;\s*url\s*=\s*)([^"'>]+)"#);
re!(ATTR, r#"(?i)(\s(?:href|src|action|poster|data-src|background|formaction))(\s*=\s*)(?:"([^"]*)"|'([^']*)'|([^\s>"'`]+))"#);
re!(SRCSET, r#"(?i)(\s(?:srcset|data-srcset))(\s*=\s*)(?:"([^"]*)"|'([^']*)')"#);
re!(STYLE_BLOCK, r"(?i)(<style\b[^>]*>)([\s\S]*?)(</style>)");
re!(STYLE_ATTR, r#"(?i)(\sstyle\s*=\s*)(?:"([^"]*)"|'([^']*)')"#);
re!(CSS_URL, r#"url\(\s*(?:"([^"]*)"|'([^']*)'|([^'")\s]+))\s*\)"#);
re!(CSS_IMPORT, r#"@import\s+(?:"([^"]*)"|'([^']*)')"#);
re!(HEAD, r"(?i)<head\b[^>]*>");
re!(HTML_TAG, r"(?i)<html\b[^>]*>");

/// First matching capture among `groups`, or "".
fn pick<'a>(c: &'a Captures, groups: &[usize]) -> &'a str {
    groups.iter().find_map(|&i| c.get(i)).map_or("", |m| m.as_str())
}

pub fn wrap(abs: &str) -> String {
    let (u, frag) = match abs.find('#') {
        Some(i) => (&abs[..i], &abs[i..]),
        None => (abs, ""),
    };
    format!("/p?u={}{}", url_encode(u), frag)
}

pub fn resolve(base: &Url, raw: &str) -> Option<String> {
    let v = raw.trim().replace("&amp;", "&").replace("&#38;", "&").replace("&#x2F;", "/").replace("&#47;", "/");
    if v.is_empty() || SKIP.is_match(&v) || v.starts_with("/p?u=") {
        return None;
    }
    let u = base.join(&v).ok()?;
    matches!(u.scheme(), "http" | "https").then(|| wrap(u.as_str()))
}

pub fn css(text: &str, base: &Url) -> String {
    let s = CSS_URL.replace_all(text, |c: &Captures| match resolve(base, pick(c, &[1, 2, 3])) {
        Some(r) => format!("url(\"{r}\")"),
        None => c[0].to_string(),
    });
    CSS_IMPORT
        .replace_all(&s, |c: &Captures| match resolve(base, pick(c, &[1, 2])) {
            Some(r) => format!("@import \"{r}\""),
            None => c[0].to_string(),
        })
        .into_owned()
}

pub fn html(src: &str, page: &Url) -> String {
    let mut base = page.clone();
    if let Some(href) = BASE.find(src).and_then(|m| BASE_HREF.captures(m.as_str())).map(|c| c[1].replace("&amp;", "&")) {
        if let Ok(b) = page.join(&href) {
            base = b;
        }
    }
    let base = &base;
    let s = BASE.replace_all(src, "");
    let s = CSP_META.replace_all(&s, "");
    let s = INTEGRITY.replace_all(&s, "");
    let s = REFRESH.replace_all(&s, |c: &Captures| {
        format!("{}{}", &c[1], resolve(base, &c[2]).unwrap_or_else(|| c[2].to_string()))
    });
    let s = ATTR.replace_all(&s, |c: &Captures| match resolve(base, pick(c, &[3, 4, 5])) {
        Some(r) => format!("{}{}\"{}\"", &c[1], &c[2], r),
        None => c[0].to_string(),
    });
    let s = SRCSET.replace_all(&s, |c: &Captures| {
        let list = pick(c, &[3, 4])
            .split(',')
            .map(|part| {
                let p = part.trim();
                let (u, rest) = p.split_once(' ').map_or((p, ""), |(u, r)| (u, r));
                if p.is_empty() {
                    String::new()
                } else {
                    let u = resolve(base, u).unwrap_or_else(|| u.to_string());
                    if rest.is_empty() { u } else { format!("{u} {rest}") }
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!("{}{}\"{}\"", &c[1], &c[2], list)
    });
    let s = STYLE_BLOCK.replace_all(&s, |c: &Captures| format!("{}{}{}", &c[1], css(&c[2], base), &c[3]));
    let s = STYLE_ATTR.replace_all(&s, |c: &Captures| {
        let v = pick(c, &[2, 3]).replace("&quot;", "'");
        format!("{}\"{}\"", &c[1], css(&v, base).replace('"', "'"))
    });

    let base_js = serde_json::to_string(base.as_str()).unwrap_or_default().replace("</", "<\\/");
    let script = format!("<script>{}</script>", INJECT.replace("%BASE%", &base_js));
    let at = HEAD.find(&s).or_else(|| HTML_TAG.find(&s)).map_or(0, |m| m.end());
    let mut out = String::with_capacity(s.len() + script.len());
    out.push_str(&s[..at]);
    out.push_str(&script);
    out.push_str(&s[at..]);
    out
}

/// Runs first inside every proxied page: routes fetch/XHR/forms/history through /p and reports URL to the shell.
const INJECT: &str = r##"
(function(){
var B=%BASE%,PX='/p?u=';
function un(u){if(typeof u==='string'&&u.indexOf(PX)===0){try{return decodeURIComponent(u.slice(PX.length).split('#')[0])}catch(e){}}return u}
function W(u){try{if(u==null)return u;u=String(u);if(!u||/^(#|javascript:|data:|blob:|about:|mailto:|tel:)/i.test(u))return u;
if(u.indexOf(PX)===0)return u;var a=new URL(u,B);
if(a.origin===location.origin){if(a.pathname==='/p')return a.pathname+a.search+a.hash;a=new URL(a.pathname+a.search+a.hash,B)}
if(!/^https?:$/.test(a.protocol))return u;var h=a.hash;a.hash='';return PX+encodeURIComponent(a.href)+h}catch(e){return u}}
window.__pbWrap=W;
function notify(){try{if(parent!==window)parent.postMessage({pb:1,url:B,title:document.title},'*')}catch(e){}}
var F=window.fetch;if(F)window.fetch=function(i,o){try{if(typeof i==='string'||i instanceof URL)i=W(String(i));else if(i&&i.url)i=new Request(W(i.url),i)}catch(e){}return F.call(this,i,o)};
var XO=XMLHttpRequest.prototype.open;XMLHttpRequest.prototype.open=function(m,u){var a=[].slice.call(arguments);a[1]=W(u);return XO.apply(this,a)};
window.open=function(u){if(u)location.href=W(u);return window};
['pushState','replaceState'].forEach(function(k){var o=history[k];history[k]=function(s,t,u){if(u!=null){try{B=new URL(u,B).href;u=W(B)}catch(e){}}var r=o.call(this,s,t,u);notify();return r}});
try{if(navigator.serviceWorker)navigator.serviceWorker.register=function(){return Promise.reject(new Error('disabled'))}}catch(e){}
document.addEventListener('click',function(e){var a=e.target&&e.target.closest&&e.target.closest('a[href]');if(!a)return;var h=a.getAttribute('href');
if(h&&h.indexOf(PX)!==0&&!/^(#|javascript:)/i.test(h))a.setAttribute('href',W(h));if(a.target&&a.target!=='_self')a.target='_self'},true);
window.addEventListener('submit',function(e){if(e.defaultPrevented)return;var f=e.target;if(!f||f.tagName!=='FORM')return;
var act=un(f.getAttribute('action')||'')||B,abs;try{abs=new URL(act,B)}catch(x){return}
if((f.getAttribute('method')||'get').toLowerCase()==='get'){e.preventDefault();var q=new URLSearchParams(new FormData(f));
if(e.submitter&&e.submitter.name)q.append(e.submitter.name,e.submitter.value||'');abs.search=q.toString();location.href=W(abs.href)}
else{f.setAttribute('action',W(abs.href));f.target='_self'}});
document.addEventListener('DOMContentLoaded',notify);window.addEventListener('load',notify);notify();
})();
"##;
