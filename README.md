# PocketBridge

Aplikasi Android: UI Kotlin + core server Rust (`rust/` → `libpocketbridge.so` via JNI) yang menjadikan HP sebagai jembatan LAN untuk laptop yang internetnya terblokir VPN.
Satu switch ON/OFF menyalakan 3 server di HP:

| Port | Fungsi | Cara pakai |
|------|--------|-----------|
| 8080 | Halaman utama (`/`): catatan bersama (`/notes`), launcher browser, AI gateway (`/gateway`), setup (`/setup`), browser ringan (`/lite`) | Buka `http://IP-HP:8080` di browser laptop |
| 8888 | HTTP/HTTPS forward proxy (CONNECT tunnel) | Dipakai launcher Windows: jendela Chrome/Edge/Brave/Firefox terpisah yang internetnya lewat HP |
| 8090 | AI gateway (reverse proxy ke provider AI, streaming SSE) | Base URL coding agent = `http://IP-HP:8090` |

Semua perangkat harus ada di WiFi/hotspot yang sama dengan HP.

## Launcher Windows
Di halaman utama, klik **Pasang launcher (sekali saja)** → jalankan `PocketBridge-Setup.bat`.
Setup mendaftarkan link `pocketbridge://` dan shortcut per browser, jadi klik icon browser di halaman utama langsung membuka jendela browser yang lewat proxy HP.
Uninstall: `powershell -File "%LOCALAPPDATA%\PocketBridge\launch.ps1" -Uninstall`.

## Password
Diisi di aplikasi atau di web. Berlaku untuk:
- Web 8080 → halaman login (cookie 30 hari).
- Proxy 8888 → IP yang sudah login di web otomatis diizinkan; selain itu Basic auth (username bebas, password = password aplikasi).
- Gateway 8090 → hanya jika **API key diisi di HP**: client wajib kirim password aplikasi sebagai API key-nya.
  Jika API key di HP kosong, request diteruskan apa adanya (pakai key dari client).

## AI gateway — contoh
Upstream di HP: `https://api.openai.com/v1` (atau `https://openrouter.ai/api/v1`, `https://api.anthropic.com`, dst).

```bash
# OpenAI-compatible
export OPENAI_BASE_URL=http://192.168.43.1:8090/v1
export OPENAI_API_KEY=<password-aplikasi atau key asli>

# Anthropic
export ANTHROPIC_BASE_URL=http://192.168.43.1:8090
```
Prefix ganda dibuang otomatis (upstream `.../v1` + request `/v1/chat/completions` → `.../v1/chat/completions`).

## Batasan browser ringan `/lite`
Halaman diambil HP lalu link-nya ditulis ulang agar lewat `/p?u=...`. Situs ringan/artikel/DuckDuckGo jalan baik;
aplikasi web berat sering gagal. Untuk itu pakai proxy port 8888.

## Build
Butuh JDK 17+, Android SDK, NDK 27, Rust (`rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android`) dan `cargo install cargo-ndk`.
Gradle otomatis menjalankan `cargo ndk` sebelum build.
```
gradlew assembleDebug
```
APK: `app/build/outputs/apk/debug/app-debug.apk`. minSdk 26, targetSdk 36, native library selaras 16 KB.

Build release butuh `keystore.properties` (tidak ikut repo):
```
storeFile=../keystore/pocketbridge.jks
storePassword=...
keyAlias=...
keyPassword=...
```

## Struktur
- `app/` — UI Android (Kotlin), foreground service, JNI.
- `rust/` — server web/proxy/gateway, halaman web di `rust/web/`.
- `store/` — teks listing Play Store, screenshot, feature graphic, video demo.
- `docs/` — GitHub Pages: [beranda](https://cyanocream.github.io/Pocketbridge/), [kebijakan privasi](https://cyanocream.github.io/Pocketbridge/privacy.html), [syarat & ketentuan](https://cyanocream.github.io/Pocketbridge/terms.html).
