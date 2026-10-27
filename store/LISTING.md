# PocketBridge — Paket Listing Google Play

Semua aset ada di `store/out/`. Teks di bawah tinggal salin ke Play Console.
Bahasa default disarankan: **Indonesia (id-ID)**, terjemahan: **English (en-US)**.

---

## 1. Aset grafis

| Slot Play Console | File | Ukuran |
|---|---|---|
| App icon | `out/icon-512.png` | 512×512 PNG |
| Feature graphic (ID) | `out/feature-graphic-id.png` | 1024×500 |
| Feature graphic (EN) | `out/feature-graphic-en.png` | 1024×500 |
| Phone screenshots (ID) | `out/screenshots/id-01.png` … `id-05.png` | 1080×1920 |
| Phone screenshots (EN) | `out/screenshots/en-01.png` … `en-05.png` | 1080×1920 |
| Video demo foreground service | `out/video/pocketbridge-fgs-demo.mp4` | 720×1600, 35 dtk |

Urutan screenshot: 01 aplikasi & switch → 02 catatan sinkron → 03 tampilan Notes → 04 internet lewat HP → 05 AI gateway.
Sumber desain (HTML) ada di `store/src/` — render ulang dengan `node shot.mjs <url> <out> <w> <h>`.

---

## 2. Detail aplikasi — Bahasa Indonesia

**Nama aplikasi** (maks 30)
```
PocketBridge – Catatan & Proxy
```

**Deskripsi singkat** (maks 80)
```
Catatan bersama, internet & AI gateway dari HP untuk laptop di jaringan lokal
```

**Deskripsi lengkap** (maks 4000)
```
PocketBridge menjadikan HP Android kamu sebagai jembatan untuk laptop dan komputer lain di WiFi atau hotspot yang sama. Cukup nyalakan satu tombol, alamat server langsung muncul, lalu buka dari browser di laptop. Tidak perlu akun, tidak perlu cloud, tidak perlu memasang apa pun di laptop.

📝 CATATAN BERSAMA
Tempel teks di laptop A, langsung muncul di laptop B. Tampilan seperti aplikasi Notes: daftar catatan, pencarian, auto-save, dan mode gelap otomatis. Semua catatan tersimpan di HP kamu sendiri.
• Sinkron otomatis antar semua perangkat yang membuka halaman
• Buat, cari, dan hapus catatan dengan cepat
• Tampilan rapi di layar laptop maupun HP

🌐 INTERNET LEWAT HP
Butuh internet dari HP di laptop tanpa mengubah pengaturan jaringan laptop? PocketBridge menyediakan proxy HTTP/HTTPS lokal dan satu klik untuk membuka jendela browser khusus yang internetnya lewat HP. Jendela browser lain dan aplikasi di laptop tetap memakai koneksi biasanya.
• Peluncur sekali klik untuk Windows, plus panduan macOS, Linux, dan Firefox
• Web app modern, login akun, upload dan download berjalan normal
• Mode browser ringan di dalam halaman untuk artikel sederhana

🤖 AI GATEWAY UNTUK CODING AGENT
Arahkan base URL coding agent atau tool AI di laptop ke HP. PocketBridge meneruskan permintaan ke provider AI pilihanmu, termasuk respons streaming.
• Isi alamat API provider di HP
• Simpan API key di HP saja — laptop cukup memakai password aplikasi
• Mendukung streaming (SSE)

🔒 PRIVASI & KEAMANAN
• Server hanya berjalan saat kamu menyalakannya dan hanya di jaringan lokal
• Lindungi dengan password: halaman web, proxy, dan gateway
• Tanpa iklan, tanpa analitik, tanpa server pihak ketiga — data kamu tidak dikirim ke pengembang

⚡ RINGAN & CEPAT
Inti server ditulis dengan Rust sehingga hemat baterai dan memori. Notifikasi berjalan menandakan server aktif, dan bisa dimatikan kapan saja dari notifikasi.

CARA PAKAI
1. Sambungkan laptop ke WiFi yang sama dengan HP, atau ke hotspot HP.
2. Buka PocketBridge, nyalakan switch Server.
3. Buka alamat yang muncul (mis. http://192.168.43.1:8080) di browser laptop.

Catatan: kuota internet yang dipakai lewat proxy dihitung dari koneksi HP kamu.
```

**Catatan rilis v1.0.0**
```
Rilis perdana PocketBridge:
• Catatan bersama yang sinkron antar perangkat
• Internet lewat HP dengan peluncur browser sekali klik
• AI gateway dengan dukungan streaming
• Proteksi password
```

---

## 3. App details — English

**App name** (max 30)
```
PocketBridge – Notes & Proxy
```

**Short description** (max 80)
```
Shared notes, internet & AI gateway from your phone to laptops on your LAN
```

**Full description** (max 4000)
```
PocketBridge turns your Android phone into a bridge for laptops and computers on the same Wi-Fi or hotspot. Flip one switch, get an address, and open it from any browser on your laptop. No account, no cloud, nothing to install on the laptop.

📝 SHARED NOTES
Paste text on laptop A and it appears on laptop B. A familiar Notes-style layout: note list, search, auto-save and automatic dark mode. Every note is stored on your own phone.
• Syncs automatically across every device that has the page open
• Create, search and delete notes quickly
• Looks great on laptop and phone screens

🌐 INTERNET THROUGH YOUR PHONE
Need your phone's connection on your laptop without changing the laptop's network settings? PocketBridge provides a local HTTP/HTTPS proxy and a one-click launcher that opens a dedicated browser window going online through your phone. Other browser windows and apps on the laptop keep using their normal connection.
• One-click launcher for Windows, plus guides for macOS, Linux and Firefox
• Modern web apps, account sign-in, uploads and downloads work normally
• Lightweight in-page browser mode for simple articles

🤖 AI GATEWAY FOR CODING AGENTS
Point the base URL of a coding agent or AI tool on your laptop at your phone. PocketBridge forwards requests to the AI provider you choose, including streaming responses.
• Set the provider's API address on the phone
• Keep the API key on the phone only — the laptop just uses the app password
• Streaming (SSE) supported

🔒 PRIVACY & SECURITY
• The server only runs while you switch it on, and only on your local network
• Password protection for the web page, proxy and gateway
• No ads, no analytics, no third-party servers — your data is never sent to the developer

⚡ LIGHT & FAST
The server core is written in Rust, so it is easy on battery and memory. A persistent notification shows when the server is running, and you can stop it right from the notification.

HOW TO USE
1. Connect your laptop to the same Wi-Fi as your phone, or to your phone's hotspot.
2. Open PocketBridge and turn on the Server switch.
3. Open the address shown (e.g. http://192.168.43.1:8080) in your laptop's browser.

Note: data used through the proxy counts against your phone's connection.
```

**Release notes v1.0.0**
```
First release of PocketBridge:
• Shared notes that sync across devices
• Internet through your phone with a one-click browser launcher
• AI gateway with streaming support
• Password protection
```

---

## 4. Kategori, kontak, dan lain-lain

| Isian | Nilai |
|---|---|
| Jenis aplikasi | App |
| Kategori | **Tools** (alternatif: Productivity) |
| Tag (pilih di konsol) | Tools, Productivity, Networking / Note taking |
| Harga | Gratis |
| Iklan | **Tidak ada iklan** |
| Email kontak | `<email-kontak-kamu>` (wajib) |
| Website | `https://cyanocream.github.io/Pocketbridge/` |
| Kebijakan privasi | `https://cyanocream.github.io/Pocketbridge/privacy.html` (lihat §6) |

---

## 5. App content (Policy) — jawaban yang disarankan

**Ads:** No, my app does not contain ads.

**App access** (instruksi untuk reviewer):
```
No login is required. To test:
1. Open the app and turn on the "Server" switch (allow the notification permission).
2. On a computer connected to the same Wi-Fi as the phone (or to the phone's hotspot), open the address shown in the app, e.g. http://<phone-ip>:8080.
3. Notes: create a note, open the same address on a second device, it appears there.
4. Proxy: on the "Internet" page, follow the launcher instructions (proxy <phone-ip>:8888).
5. AI gateway: enter an API base URL in the app (e.g. https://api.openai.com/v1) and send requests to http://<phone-ip>:8090.
The password field is optional; leave it empty for testing.
```

**Content rating (IARC):** kategori *Utility, Productivity, Communication, or Other*. Jawab "Tidak" untuk kekerasan, seksual, judi, dll.
Pertanyaan *"Does the app allow users to access unrestricted internet content (e.g. a web browser)?"* → **Yes** (fitur proxy / browser ringan). Ini wajar untuk aplikasi utilitas jaringan; rating biasanya tetap rendah.
Interaksi antar pengguna / berbagi konten dengan orang lain → **No** (catatan hanya antar perangkat milik pengguna sendiri).

**Target audience:** 18+ (hindari program Families yang punya syarat tambahan).

**News app:** No. **COVID-19 app:** No. **Government app:** No. **Financial features:** None. **Health:** None.

**Data safety** (rekomendasi — pastikan sesuai dengan pemahamanmu):
- *Does your app collect or share any of the required user data types?* → **No**.
  Alasan: catatan dan pengaturan hanya disimpan di HP. Data hanya berpindah ke perangkat yang pengguna sendiri buka di jaringan lokal, atau ke situs/provider AI yang pengguna sendiri tuju (seperti browser). Pengembang dan pihak ketiga tidak menerima data apa pun; tidak ada analitik atau SDK pelacak.
- Jika reviewer meminta detail: data di-transfer lewat HTTP lokal (tidak terenkripsi di LAN), dan pengguna bisa menghapus catatan kapan saja atau meng-uninstall untuk menghapus semua data.

**Foreground service — `FOREGROUND_SERVICE_SPECIAL_USE`** (wajib dideklarasikan):
- Jenis tugas: *Other / special use*
- Deskripsi:
```
PocketBridge runs a user-started local network server (shared notes web page on port 8080, an HTTP/HTTPS proxy on port 8888 and an AI API gateway on port 8090) so that the user's own laptops on the same Wi-Fi/hotspot can connect to it. The server must keep listening for incoming connections while the app is in the background or the screen is off; it only runs after the user turns on the switch, shows a persistent notification, and can be stopped from the app or the notification. No standard foreground service type covers serving incoming local network connections.
```
- Video demo: **sudah dibuat** → `store/out/video/pocketbridge-fgs-demo.mp4` (38 dtk, UI terbaru: nyalakan switch → notifikasi "PocketBridge aktif" → halaman PocketBridge di browser → catatan dari laptop muncul → server dimatikan). Unggah ke YouTube sebagai *Unlisted*, tempel link-nya di formulir.

---

## 6. Kebijakan privasi & ketentuan

Di-host lewat GitHub Pages dari folder `docs/` (branch `main`):
- Beranda: `https://cyanocream.github.io/Pocketbridge/`
- Kebijakan privasi: `https://cyanocream.github.io/Pocketbridge/privacy.html`
- Syarat & ketentuan: `https://cyanocream.github.io/Pocketbridge/terms.html`

Kontak di kedua halaman memakai GitHub Issues. Email kontak tetap wajib diisi di Play Console (tidak ditampilkan di halaman ini).

---

## 7. File untuk diunggah

| | File |
|---|---|
| **Upload ke Play Console** | `release/PocketBridge-1.0.0.aab` (Android App Bundle — wajib untuk aplikasi baru) |
| Install manual / sideload | `release/PocketBridge-1.0.0.apk` |

- Target SDK 36, min SDK 26, native library selaras 16 KB (syarat Play).
- Keystore `keystore/pocketbridge.jks` = **upload key**. Aktifkan *Play App Signing* saat upload pertama (default). Simpan keystore + `keystore.properties` di tempat aman; tanpa itu kamu tidak bisa merilis update.
- Untuk update berikutnya: naikkan `versionCode` (dan `versionName`) di `app/build.gradle.kts`, lalu `gradlew bundleRelease`.

## 8. Checklist sebelum submit

- [ ] Akun Google Play Console (biaya pendaftaran sekali bayar) dan verifikasi identitas
- [ ] Akun personal baru: wajib **closed testing minimal 12 tester selama 14 hari** sebelum bisa rilis ke Production
- [ ] Isi email kontak di Play Console (kebijakan privasi sudah di-host di GitHub Pages)
- [x] Rekam video demo foreground service
- [ ] Uji sekali lagi di HP: build release (bukan debug), fitur catatan, proxy di browser sungguhan, dan AI gateway dengan provider asli
